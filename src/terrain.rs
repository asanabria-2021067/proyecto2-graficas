use crate::material::block;
use crate::noise::{Pcg32, Perlin};
use crate::world::World;

/// Surface height (top of grass/sand, or i32::MIN if outside the island) per
/// (x,z) column, so later phases can place structures relative to the real
/// terrain height instead of a fixed y.
pub struct Heightmap {
    nx: i32,
    nz: i32,
    top: Vec<i32>,
    on_island: Vec<bool>,
}

impl Heightmap {
    #[inline]
    fn idx(&self, x: i32, z: i32) -> usize {
        (z * self.nx + x) as usize
    }

    /// Surface y at (x,z), or `None` if outside the world or not part of the island.
    pub fn top_at(&self, x: i32, z: i32) -> Option<i32> {
        if x < 0 || z < 0 || x >= self.nx || z >= self.nz {
            return None;
        }
        let i = self.idx(x, z);
        if self.on_island[i] {
            Some(self.top[i])
        } else {
            None
        }
    }
}

pub struct IslandParams {
    pub center_x: i32,
    pub center_z: i32,
    pub radius: f32,
    pub top_y: i32,
    pub water_level: i32,
    pub max_depth: f32,
    pub tree_count: u32,
}

impl IslandParams {
    /// Cada isla vive en su propia grilla chica (en vez de que todas
    /// compartan una grilla gigante mayormente vacia), asi que el centro se
    /// calcula solo a partir del radio: siempre es el centro de la grilla
    /// que le toca (ver `grid_dims`). `top_y`/`water_level`/`max_depth`
    /// quedan con valores por defecto razonables; se pueden pisar despues de
    /// construir si una isla (Nether, End) necesita un perfil distinto.
    pub fn new(radius: f32, tree_count: u32) -> Self {
        let half = radius.ceil() as i32 + 10;
        IslandParams {
            center_x: half,
            center_z: half,
            radius,
            top_y: 38,
            water_level: 29,
            max_depth: radius * 0.62,
            tree_count,
        }
    }

    /// Tamano de grilla que le corresponde a esta isla: justo el radio mas
    /// un margen, sin compartir espacio vacio con otras islas.
    pub fn grid_dims(&self) -> (i32, i32, i32) {
        let half = self.radius.ceil() as i32 + 10;
        (half * 2, 80, half * 2)
    }
}

/// Genera una isla flotante en su propia grilla (creada aqui, del tamano
/// justo que necesita): heightmap fBm enmascarado por un radio deformado con
/// ruido (para que la orilla no sea un circulo perfecto), capas grass/dirt/
/// stone_bricks con arena cerca del nivel de agua y agua llenando las
/// depresiones, una base conica irregular hacia abajo (ruido 3D), y arboles
/// colocados con distancia minima entre ellos. Devuelve la grilla nueva y su
/// heightmap para que fases posteriores ubiquen estructuras encima.
pub fn generate_island(seed: u32, p: &IslandParams) -> (World, Heightmap) {
    let (nx, ny, nz) = p.grid_dims();
    let mut world = World::new(nx, ny, nz);
    let perlin = Perlin::new(seed);
    let mut top = vec![i32::MIN; (world.nx * world.nz) as usize];
    let mut on_island = vec![false; (world.nx * world.nz) as usize];

    for z in 0..world.nz {
        for x in 0..world.nx {
            let dx = (x - p.center_x) as f32;
            let dz = (z - p.center_z) as f32;
            let dist = (dx * dx + dz * dz).sqrt() / p.radius;

            let edge_noise = perlin.fbm2(x as f32 * 0.045 + 500.0, z as f32 * 0.045, 4, 2.0, 0.5);
            let dist_deformed = dist + edge_noise * 0.3;
            if dist_deformed >= 1.0 {
                continue;
            }

            let height_noise = perlin.fbm2(x as f32 * 0.07, z as f32 * 0.07, 5, 2.0, 0.5);
            let surface_y = p.top_y + (height_noise * 6.0) as i32;

            let dist_norm = dist.clamp(0.0, 1.0);
            let cone = p.max_depth * (1.0 - dist_norm).powf(1.5);
            let jag = perlin.noise3(x as f32 * 0.1, z as f32 * 0.1, 7.3);
            let bottom_y = (p.top_y as f32 - cone - (jag * 0.5 + 0.5) * 4.0).round() as i32;
            let bottom_y = bottom_y.max(1);

            let is_underwater_dip = surface_y < p.water_level;

            // Roca hasta 3 bloques bajo la superficie, luego tierra, luego la
            // capa superior (pasto o arena cerca del agua).
            let dirt_layers = 3;
            world.fill_box((x, bottom_y, z), (x, (surface_y - dirt_layers).max(bottom_y), z), block::STONE_BRICKS);
            world.fill_box((x, (surface_y - dirt_layers + 1).max(bottom_y), z), (x, surface_y - 1, z), block::DIRT);

            let near_water = surface_y <= p.water_level + 1;
            let top_block = if near_water { block::SAND } else { block::GRASS };
            world.set(x, surface_y, z, top_block);

            if is_underwater_dip {
                world.fill_box((x, surface_y + 1, z), (x, p.water_level, z), block::WATER);
            }

            let i = (z * world.nx + x) as usize;
            top[i] = if is_underwater_dip { p.water_level } else { surface_y };
            on_island[i] = true;
        }
    }

    let heightmap = Heightmap { nx: world.nx, nz: world.nz, top, on_island };
    place_trees(&mut world, &heightmap, p, seed);
    (world, heightmap)
}

/// Como `generate_island`, pero con un perfil de ruido "ridged" (se pliega el
/// fBm sobre si mismo) para que en vez de colinas suaves salgan picos
/// filosos y grietas entre ellos -- pensado para la isla del Nether. Capas
/// solo de netherrack (sin pasto/tierra/agua), con parches de magma al azar
/// en la superficie marcando las grietas que brillan. No planta arboles.
pub fn generate_rugged_island(seed: u32, p: &IslandParams) -> (World, Heightmap) {
    let (nx, ny, nz) = p.grid_dims();
    let mut world = World::new(nx, ny, nz);
    let perlin = Perlin::new(seed);
    let mut top = vec![i32::MIN; (world.nx * world.nz) as usize];
    let mut on_island = vec![false; (world.nx * world.nz) as usize];

    for z in 0..world.nz {
        for x in 0..world.nx {
            let dx = (x - p.center_x) as f32;
            let dz = (z - p.center_z) as f32;
            let dist = (dx * dx + dz * dz).sqrt() / p.radius;

            let edge_noise = perlin.fbm2(x as f32 * 0.06 + 700.0, z as f32 * 0.06, 4, 2.0, 0.55);
            let dist_deformed = dist + edge_noise * 0.4; // orilla mas quebrada que la isla principal
            if dist_deformed >= 1.0 {
                continue;
            }

            let raw = perlin.fbm2(x as f32 * 0.09, z as f32 * 0.09, 5, 2.0, 0.5);
            let ridged = (1.0 - raw.abs() * 2.0).clamp(-1.0, 1.0);
            let surface_y = p.top_y + (ridged * 10.0) as i32;

            let dist_norm = dist.clamp(0.0, 1.0);
            let cone = p.max_depth * (1.0 - dist_norm).powf(1.3);
            let jag = perlin.noise3(x as f32 * 0.12, z as f32 * 0.12, 11.7);
            let bottom_y = (p.top_y as f32 - cone - (jag * 0.5 + 0.5) * 6.0).round() as i32;
            let bottom_y = bottom_y.max(1);

            let crack = perlin.noise3(x as f32 * 0.15, z as f32 * 0.15, 3.3) > 0.6;
            let top_block = if crack { block::MAGMA } else { block::NETHERRACK };

            world.fill_box((x, bottom_y, z), (x, (surface_y - 1).max(bottom_y), z), block::NETHERRACK);
            world.set(x, surface_y, z, top_block);

            let i = (z * world.nx + x) as usize;
            top[i] = surface_y;
            on_island[i] = true;
        }
    }

    let heightmap = Heightmap { nx: world.nx, nz: world.nz, top, on_island };
    (world, heightmap)
}

/// Como `generate_island`, pero con un perfil suave y redondeado (mucha
/// menos amplitud de ruido, borde apenas deformado) para la isla del End:
/// una loma achatada de bordes blandos en vez de colinas marcadas. Capas
/// solo de end_stone (sin pasto/tierra/agua). No planta arboles.
pub fn generate_soft_island(seed: u32, p: &IslandParams) -> (World, Heightmap) {
    let (nx, ny, nz) = p.grid_dims();
    let mut world = World::new(nx, ny, nz);
    let perlin = Perlin::new(seed);
    let mut top = vec![i32::MIN; (world.nx * world.nz) as usize];
    let mut on_island = vec![false; (world.nx * world.nz) as usize];

    for z in 0..world.nz {
        for x in 0..world.nx {
            let dx = (x - p.center_x) as f32;
            let dz = (z - p.center_z) as f32;
            let dist = (dx * dx + dz * dz).sqrt() / p.radius;

            let edge_noise = perlin.fbm2(x as f32 * 0.035 + 300.0, z as f32 * 0.035, 3, 2.0, 0.5);
            let dist_deformed = dist + edge_noise * 0.15; // borde mucho mas parejo que las demas islas
            if dist_deformed >= 1.0 {
                continue;
            }

            let height_noise = perlin.fbm2(x as f32 * 0.04, z as f32 * 0.04, 3, 2.0, 0.5);
            let surface_y = p.top_y + (height_noise * 2.5) as i32;

            let dist_norm = dist.clamp(0.0, 1.0);
            let cone = p.max_depth * (1.0 - dist_norm).powf(1.8); // achatada: cae mas de golpe cerca del borde
            let jag = perlin.noise3(x as f32 * 0.08, z as f32 * 0.08, 5.1);
            let bottom_y = (p.top_y as f32 - cone - (jag * 0.5 + 0.5) * 2.0).round() as i32;
            let bottom_y = bottom_y.max(1);

            world.fill_box((x, bottom_y, z), (x, surface_y, z), block::END_STONE);

            let i = (z * world.nx + x) as usize;
            top[i] = surface_y;
            on_island[i] = true;
        }
    }

    let heightmap = Heightmap { nx: world.nx, nz: world.nz, top, on_island };
    (world, heightmap)
}

fn place_trees(world: &mut World, hm: &Heightmap, p: &IslandParams, seed: u32) {
    let mut rng = Pcg32::new(seed as u64 ^ 0xA11C_E000, 0x7EE);
    let mut placed: Vec<(i32, i32)> = Vec::new();
    let min_spacing2 = 5 * 5;
    let mut attempts = 0;

    while placed.len() < p.tree_count as usize && attempts < p.tree_count * 40 {
        attempts += 1;
        let x = rng.range_i32(p.center_x - p.radius as i32, p.center_x + p.radius as i32);
        let z = rng.range_i32(p.center_z - p.radius as i32, p.center_z + p.radius as i32);
        let Some(top_y) = hm.top_at(x, z) else { continue };
        if world.get(x, top_y, z) != block::GRASS {
            continue;
        }
        if placed.iter().any(|&(px, pz)| (px - x) * (px - x) + (pz - z) * (pz - z) < min_spacing2) {
            continue;
        }
        let height = rng.range_i32(3, 5);
        place_tree(world, x, top_y, z, height, &mut rng);
        placed.push((x, z));
    }
}

fn place_tree(world: &mut World, x: i32, base_y: i32, z: i32, height: i32, rng: &mut Pcg32) {
    world.fill_box((x, base_y + 1, z), (x, base_y + height, z), block::OAK_LOG);

    let canopy_base = base_y + height - 1;
    for (layer, radius) in [(0, 2), (1, 2), (2, 1)] {
        let cy = canopy_base + layer;
        for dz in -radius..=radius {
            for dx in -radius..=radius {
                if dx == 0 && dz == 0 && layer < 2 {
                    continue; // el tronco ya ocupa el centro de las capas bajas
                }
                let d2 = dx * dx + dz * dz;
                if d2 > radius * radius {
                    continue;
                }
                if d2 == radius * radius && rng.next_f32() < 0.35 {
                    continue; // borde irregular
                }
                world.set(x + dx, cy, z + dz, block::LEAVES);
            }
        }
    }
}
