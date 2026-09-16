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
    pub fn main_island(world: &World) -> Self {
        IslandParams {
            center_x: world.nx / 2,
            center_z: world.nz / 2,
            radius: (world.nx.min(world.nz) as f32) * 0.42,
            top_y: (world.ny as f32 * 0.60) as i32,
            water_level: (world.ny as f32 * 0.46) as i32,
            max_depth: world.ny as f32 * 0.4,
            tree_count: 24,
        }
    }
}

/// Generates a floating island in `world`: an fBm heightmap masked by a
/// noise-deformed radial falloff (so the shoreline isn't a perfect circle),
/// grass/dirt/stone layering with sand near the waterline and water filling
/// the depressions, an irregular downward-tapering stone base (3D noise), and
/// randomly placed trees with a minimum spacing. Returns the resulting
/// heightmap for later phases to place structures on top of.
pub fn generate_island(world: &mut World, seed: u32, p: &IslandParams) -> Heightmap {
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
    place_trees(world, &heightmap, p, seed);
    heightmap
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
