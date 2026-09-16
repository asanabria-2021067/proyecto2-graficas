//! Fase 9 + "parte 2": "La isla del faro" y sus vecinas. Construye el faro,
//! el lago con cascada, el muelle, la casita, dos islas satelite (monolito e
//! jardin con fuente) y postes de luz, todo sobre el terreno procedural de
//! fase 8, usando funciones helper relativas a la altura real del terreno
//! para que funcionen con cualquier semilla. Desde la "parte 2" cada isla
//! vive en su propia grilla (`Island`, ver islands.rs) en vez de compartir
//! una grilla gigante mayormente vacia; los puentes entre islas son su
//! propia mini-grilla tambien.

use crate::islands::Island;
use crate::material::block;
use crate::math::Vec3;
use crate::noise::Pcg32;
use crate::terrain::{generate_island, Heightmap, IslandParams};
use crate::world::World;

// ---------- helpers genericos ----------

fn polar(cx: i32, cz: i32, angle_deg: f32, dist: f32) -> (i32, i32) {
    let r = angle_deg.to_radians();
    (cx + (r.cos() * dist).round() as i32, cz + (r.sin() * dist).round() as i32)
}

fn lerp_point(a: (i32, i32), b: (i32, i32), t: f32) -> (i32, i32) {
    (a.0 + ((b.0 - a.0) as f32 * t).round() as i32, a.1 + ((b.1 - a.1) as f32 * t).round() as i32)
}

/// Nivela una zona rectangular a `target_y`: limpia lo que sobresalga y
/// rellena lo que falte, para que una estructura se asiente parejo sin
/// importar la variacion natural del terreno debajo.
pub fn flatten_area(world: &mut World, x0: i32, z0: i32, x1: i32, z1: i32, target_y: i32, top_block: u8) {
    for z in z0..=z1 {
        for x in x0..=x1 {
            world.fill_box((x, target_y + 1, z), (x, (target_y + 12).min(world.ny - 1), z), block::AIR);
            world.fill_box((x, (target_y - 4).max(0), z), (x, target_y - 1, z), block::STONE_BRICKS);
            world.set(x, target_y, z, top_block);
        }
    }
}

pub fn place_pillar(world: &mut World, x: i32, z: i32, y0: i32, y1: i32, id: u8) {
    world.fill_box((x, y0, z), (x, y1, z), id);
}

/// Quita troncos y hojas en un radio alrededor de (cx,cz), en toda la altura
/// de la isla. Se llama antes de construir cada edificio para que los
/// arboles no queden pegados/encimados con las estructuras.
fn clear_trees_near(world: &mut World, cx: i32, cz: i32, radius: i32) {
    for dz in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dz * dz > radius * radius {
                continue;
            }
            let x = cx + dx;
            let z = cz + dz;
            for y in 0..world.ny {
                let id = world.get(x, y, z);
                if id == block::OAK_LOG || id == block::LEAVES {
                    world.set(x, y, z, block::AIR);
                }
            }
        }
    }
}

/// Escalera ascendente de `steps` escalones de `width` bloques de ancho,
/// avanzando en la direccion (step_dx,step_dz) y subiendo 1 bloque por escalon.
#[allow(clippy::too_many_arguments)]
pub fn place_stairs_like(world: &mut World, x0: i32, z0: i32, y_base: i32, step_dx: i32, step_dz: i32, steps: i32, width: i32, id: u8) {
    let (perp_x, perp_z) = (-step_dz, step_dx);
    for s in 0..steps {
        let cx = x0 + step_dx * s;
        let cz = z0 + step_dz * s;
        let y = y_base + s;
        for w in 0..width {
            let wx = cx + perp_x * w;
            let wz = cz + perp_z * w;
            world.fill_box((wx, y_base, wz), (wx, y, wz), id);
        }
    }
}

/// Camina desde (cx,cz) hacia `angle_rad` hasta salir de la isla; devuelve la
/// ultima columna que seguia siendo parte de ella (el borde real, sea cual
/// sea la semilla).
fn find_edge(hm: &Heightmap, cx: i32, cz: i32, angle_rad: f32, max_r: f32) -> (i32, i32, i32) {
    let (dx, dz) = (angle_rad.cos(), angle_rad.sin());
    let mut last = (cx, cz, hm.top_at(cx, cz).unwrap_or(0));
    let mut r = 1.0;
    while r <= max_r {
        let x = cx + (dx * r).round() as i32;
        let z = cz + (dz * r).round() as i32;
        match hm.top_at(x, z) {
            Some(y) => last = (x, z, y),
            None => break,
        }
        r += 1.0;
    }
    last
}

/// Camino de tablones de `width` bloques entre dos puntos (con postes de
/// tronco de 3 de alto cada 2 bloques a los lados, como baranda), usado
/// tanto para el muelle como para los puentes entre islas.
#[allow(clippy::too_many_arguments)]
pub fn build_walkway(world: &mut World, x0: i32, z0: i32, y0: i32, x1: i32, z1: i32, y1: i32, width: i32, deck_id: u8, post_id: u8) {
    let dx = x1 - x0;
    let dz = z1 - z0;
    let steps = dx.abs().max(dz.abs()).max(1);
    let len = ((dx * dx + dz * dz) as f32).sqrt().max(1.0);
    let (px, pz) = (-(dz as f32) / len, (dx as f32) / len);

    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let cx = x0 as f32 + dx as f32 * t;
        let cz = z0 as f32 + dz as f32 * t;
        let y = (y0 as f32 + (y1 - y0) as f32 * t).round() as i32;

        for w in 0..width {
            let off = w as f32 - (width as f32 - 1.0) / 2.0;
            let wx = (cx + px * off).round() as i32;
            let wz = (cz + pz * off).round() as i32;
            world.set(wx, y, wz, deck_id);
        }
        if i % 2 == 0 {
            let off = (width as f32 - 1.0) / 2.0 + 0.5;
            for sign in [-1.0f32, 1.0] {
                let wx = (cx + px * off * sign).round() as i32;
                let wz = (cz + pz * off * sign).round() as i32;
                world.fill_box((wx, y + 1, wz), (wx, y + 3, wz), post_id);
            }
        }
    }
}

// ---------- estructuras ----------

fn build_lighthouse(world: &mut World, hm: &Heightmap, cx: i32, cz: i32) {
    let base_y = hm.top_at(cx, cz).unwrap_or(world.ny / 2);
    flatten_area(world, cx - 4, cz - 4, cx + 4, cz + 4, base_y, block::GRASS);

    let mut y = base_y;
    world.fill_box((cx - 3, y + 1, cz - 3), (cx + 3, y + 4, cz + 3), block::STONE_BRICKS);
    y += 4;
    world.fill_box((cx - 3, y + 1, cz - 3), (cx + 3, y + 1, cz + 3), block::IRON_BLOCK);
    y += 1;
    world.fill_box((cx - 2, y + 1, cz - 2), (cx + 2, y + 4, cz + 2), block::STONE_BRICKS);
    y += 4;
    world.fill_box((cx - 2, y + 1, cz - 2), (cx + 2, y + 1, cz + 2), block::IRON_BLOCK);
    y += 1;
    world.fill_box((cx - 1, y + 1, cz - 1), (cx + 1, y + 3, cz + 1), block::STONE_BRICKS);
    y += 3;

    // Cuarto de linterna: vidrio hueco con glowstone adentro (se ve el brillo a traves del vidrio).
    world.hollow_box((cx - 1, y + 1, cz - 1), (cx + 1, y + 3, cz + 1), block::GLASS);
    world.set(cx, y + 2, cz, block::GLOWSTONE);
    y += 3;

    world.fill_box((cx - 1, y + 1, cz - 1), (cx + 1, y + 1, cz + 1), block::IRON_BLOCK);
    world.set(cx, y + 2, cz, block::IRON_BLOCK);
}

fn build_lake_and_waterfall(world: &mut World, hm: &Heightmap, cx: i32, cz: i32, radius: i32, water_level: i32, outward_angle_deg: f32) {
    for dz in -radius..=radius {
        for dx in -radius..=radius {
            let dist2 = dx * dx + dz * dz;
            if dist2 > radius * radius {
                continue;
            }
            let x = cx + dx;
            let z = cz + dz;
            let Some(top) = hm.top_at(x, z) else { continue };
            if top > water_level {
                world.fill_box((x, water_level + 1, z), (x, top, z), block::AIR);
            }
            // Fondo en forma de cuenco: mas hondo en el centro (hasta 3
            // bloques), se va achicando hacia la orilla, para que el lago se
            // lea como un cuerpo de agua real y no un charco de un bloque.
            let dist_norm = (dist2 as f32).sqrt() / radius as f32;
            let bowl_depth = ((1.0 - dist_norm) * 3.0) as i32;
            let bed_y = (water_level - 1 - bowl_depth).min(top);
            world.set(x, bed_y, z, block::SAND);
            world.fill_box((x, bed_y + 1, z), (x, water_level, z), block::WATER);
        }
    }

    // Cascada: un canal de agua corre desde el borde del lago hasta el borde
    // real de la isla (sea cual sea la distancia con esta semilla), y ahi se
    // desploma por el costado hacia el vacio.
    let angle = outward_angle_deg.to_radians();
    let (dx, dz) = (angle.cos(), angle.sin());
    let (edge_x, edge_z, _) = find_edge(hm, cx, cz, angle, radius as f32 + 80.0);
    let edge_dist = (((edge_x - cx) * (edge_x - cx) + (edge_z - cz) * (edge_z - cz)) as f32).sqrt();
    let (perp_x, perp_z) = (-dz, dx);

    let mut t = radius as f32 - 1.0;
    while t < edge_dist {
        let cx_f = cx as f32 + dx * t;
        let cz_f = cz as f32 + dz * t;
        for w in -1..=1 {
            let x = (cx_f + perp_x * w as f32).round() as i32;
            let z = (cz_f + perp_z * w as f32).round() as i32;
            let Some(top) = hm.top_at(x, z) else { continue };
            if top > water_level {
                world.fill_box((x, water_level, z), (x, top, z), block::AIR);
            }
            world.set(x, water_level, z, block::WATER);
        }
        t += 1.0;
    }

    for w in -1..=1 {
        let x = edge_x + (perp_x * w as f32).round() as i32;
        let z = edge_z + (perp_z * w as f32).round() as i32;
        for depth in 0..22 {
            let y = water_level - depth;
            if y < 1 {
                break;
            }
            world.set(x, y, z, block::WATER);
        }
    }
}

fn build_dock(world: &mut World, hm: &Heightmap, lake_cx: i32, lake_cz: i32, lake_radius: i32, shore_angle_deg: f32, water_level: i32) {
    let (shore_x, shore_z) = polar(lake_cx, lake_cz, shore_angle_deg, lake_radius as f32 + 1.0);
    let Some(shore_y) = hm.top_at(shore_x, shore_z) else { return };
    let dock_y = water_level + 1;
    let dir_deg = shore_angle_deg + 180.0;

    // Unos escalones bajan de la orilla al nivel del muelle cuando no coinciden:
    // la base (mas baja) queda del lado del lago, subiendo hacia la orilla.
    let start = if shore_y > dock_y {
        let drop = (shore_y - dock_y).min(6);
        let far = polar(shore_x, shore_z, dir_deg, drop as f32);
        let back = shore_angle_deg.to_radians();
        place_stairs_like(world, far.0, far.1, dock_y, back.cos().round() as i32, back.sin().round() as i32, drop, 3, block::OAK_PLANKS);
        far
    } else {
        (shore_x, shore_z)
    };

    let dock_len = (lake_radius - 1).max(3);
    let (end_x, end_z) = polar(start.0, start.1, dir_deg, dock_len as f32);

    build_walkway(world, start.0, start.1, dock_y, end_x, end_z, dock_y, 3, block::OAK_PLANKS, block::OAK_LOG);
    world.fill_box((end_x, dock_y + 1, end_z), (end_x, dock_y + 2, end_z), block::LAMP_FRAME);
    world.set(end_x, dock_y + 3, end_z, block::GLOWSTONE);
}

fn build_house(world: &mut World, hm: &Heightmap, cx: i32, cz: i32) {
    let base_y = hm.top_at(cx, cz).unwrap_or(world.ny / 2);
    flatten_area(world, cx - 5, cz - 5, cx + 5, cz + 5, base_y, block::GRASS);

    let h = 6;
    let r = 4; // medio-footprint: casa de 9x9
    let y0 = base_y + 1;
    let corners = [(-r, -r), (r, -r), (-r, r), (r, r)];

    world.hollow_box((cx - r, y0, cz - r), (cx + r, y0 + h - 1, cz + r), block::OAK_PLANKS);
    for &(dx, dz) in &corners {
        place_pillar(world, cx + dx, cz + dz, y0, y0 + h - 1, block::OAK_LOG);
    }

    // Dos ventanas por lado (salvo el lado de la puerta) para que se lea
    // grande y con mas detalle; puerta de doble alto en el frente (+z).
    for &off in &[-2, 2] {
        world.set(cx + off, y0 + 1, cz - r, block::GLASS);
        world.set(cx + off, y0 + 2, cz - r, block::GLASS);
        world.set(cx - r, y0 + 1, cz + off, block::GLASS);
        world.set(cx - r, y0 + 2, cz + off, block::GLASS);
        world.set(cx + r, y0 + 1, cz + off, block::GLASS);
        world.set(cx + r, y0 + 2, cz + off, block::GLASS);
    }
    world.fill_box((cx, y0, cz + r), (cx, y0 + 1, cz + r), block::AIR); // puerta
    world.set(cx, y0 + 2, cz, block::GLOWSTONE); // lampara interior, se ve por las ventanas de noche
    world.set(cx - 1, y0 + 2, cz, block::GLOWSTONE);

    let roof_y = y0 + h;
    for (layer, inset) in [(0, 0), (1, 1), (2, 2), (3, 3)] {
        world.fill_box((cx - r + inset, roof_y + layer, cz - r + inset), (cx + r - inset, roof_y + layer, cz + r - inset), block::OAK_PLANKS);
    }
    world.set(cx, roof_y + 4, cz, block::OAK_PLANKS);
}

fn build_statue(world: &mut World, hm: &Heightmap, cx: i32, cz: i32) {
    let base_y = hm.top_at(cx, cz).unwrap_or(world.ny / 2);
    flatten_area(world, cx - 2, cz - 2, cx + 2, cz + 2, base_y, block::GRASS);
    world.fill_box((cx - 1, base_y + 1, cz - 1), (cx + 1, base_y + 1, cz + 1), block::STONE_BRICKS);
    world.fill_box((cx, base_y + 2, cz), (cx, base_y + 7, cz), block::IRON_BLOCK);
    world.fill_box((cx - 1, base_y + 3, cz), (cx + 1, base_y + 3, cz), block::IRON_BLOCK);
}

fn build_garden(world: &mut World, hm: &Heightmap, cx: i32, cz: i32, seed: u32) {
    let base_y = hm.top_at(cx, cz).unwrap_or(world.ny / 2);
    flatten_area(world, cx - 5, cz - 5, cx + 5, cz + 5, base_y, block::GRASS);

    let r = 2;
    for dz in -r..=r {
        for dx in -r..=r {
            if dx * dx + dz * dz > r * r {
                continue;
            }
            world.set(cx + dx, base_y + 1, cz + dz, block::WATER);
        }
    }
    world.set(cx, base_y + 1, cz, block::IRON_BLOCK);

    let mut rng = Pcg32::new(seed as u64 ^ 0xDEAD_BEEF, 0x0A2D_E100);
    let mut placed = 0;
    let mut attempts = 0;
    while placed < 6 && attempts < 60 {
        attempts += 1;
        let bx = cx + rng.range_i32(-4, 4);
        let bz = cz + rng.range_i32(-4, 4);
        if (bx - cx) * (bx - cx) + (bz - cz) * (bz - cz) <= (r + 1) * (r + 1) {
            continue; // no encima de la fuente
        }
        world.set(bx, base_y + 1, bz, block::LEAVES);
        if rng.next_f32() < 0.4 {
            world.set(bx, base_y + 2, bz, block::LEAVES);
        }
        placed += 1;
    }
}

fn build_path_lights(world: &mut World, hm: &Heightmap, points: &[(i32, i32)]) {
    for &(x, z) in points {
        let Some(y) = hm.top_at(x, z) else { continue };
        let top = world.get(x, y, z);
        if top == block::GRASS || top == block::SAND {
            place_pillar(world, x, z, y + 1, y + 2, block::OAK_LOG);
            world.set(x, y + 3, z, block::GLOWSTONE);
        }
    }
}

/// Construye un puente entre dos islas COMO SU PROPIA MINI-GRILLA (`Island`
/// nueva, empujada a `islands`): encuentra el borde real de cada isla en
/// direccion a la otra (`find_edge`, funciona con cualquier semilla), nivela
/// un parche chico en cada isla para que la transicion no tenga escalon, y
/// tiende una pasarela dentro de la grilla nueva que solo cubre el hueco
/// entre ambas (mas un margen), no toda la distancia entre sus centros.
#[allow(clippy::too_many_arguments)]
fn build_bridge(islands: &mut Vec<Island>, idx_a: usize, hm_a: &Heightmap, pa: &IslandParams, idx_b: usize, hm_b: &Heightmap, pb: &IslandParams, min_world_y: i32) {
    let a_center_world = islands[idx_a].to_world_point(Vec3::new(pa.center_x as f32, pa.top_y as f32, pa.center_z as f32));
    let b_center_world = islands[idx_b].to_world_point(Vec3::new(pb.center_x as f32, pb.top_y as f32, pb.center_z as f32));
    let angle_deg = (b_center_world.z - a_center_world.z).atan2(b_center_world.x - a_center_world.x).to_degrees();

    let edge_a = find_edge(hm_a, pa.center_x, pa.center_z, angle_deg.to_radians(), pa.radius + 200.0);
    let edge_b = find_edge(hm_b, pb.center_x, pb.center_z, (angle_deg + 180.0).to_radians(), pb.radius + 200.0);

    let edge_a_world = islands[idx_a].to_world_point(Vec3::new(edge_a.0 as f32, edge_a.2 as f32, edge_a.1 as f32));
    let edge_b_world = islands[idx_b].to_world_point(Vec3::new(edge_b.0 as f32, edge_b.2 as f32, edge_b.1 as f32));
    let bridge_y_world = (((edge_a_world.y + edge_b_world.y) * 0.5).round() as i32).max(min_world_y);

    // Nivela un parche en cada isla (en SUS coordenadas locales) para que la
    // transicion al puente no tenga un escalon.
    let local_y_a = bridge_y_world - islands[idx_a].offset.1;
    flatten_area(&mut islands[idx_a].world, edge_a.0 - 1, edge_a.1 - 1, edge_a.0 + 1, edge_a.1 + 1, local_y_a, block::OAK_PLANKS);
    let local_y_b = bridge_y_world - islands[idx_b].offset.1;
    flatten_area(&mut islands[idx_b].world, edge_b.0 - 1, edge_b.1 - 1, edge_b.0 + 1, edge_b.1 + 1, local_y_b, block::OAK_PLANKS);

    // Grilla propia del puente: solo el hueco entre las dos islas, mas margen.
    let margin = 4;
    let min_x = edge_a_world.x.min(edge_b_world.x) as i32 - margin;
    let max_x = edge_a_world.x.max(edge_b_world.x) as i32 + margin;
    let min_z = edge_a_world.z.min(edge_b_world.z) as i32 - margin;
    let max_z = edge_a_world.z.max(edge_b_world.z) as i32 + margin;
    let min_y = bridge_y_world - 6;
    let bridge_offset = (min_x, min_y, min_z);
    let mut bridge_island = Island::new(World::new((max_x - min_x).max(4), 16, (max_z - min_z).max(4)), bridge_offset);

    let local_a = bridge_island.to_local_point(edge_a_world);
    let local_b = bridge_island.to_local_point(edge_b_world);
    let by = bridge_y_world - min_y;
    build_walkway(&mut bridge_island.world, local_a.x.round() as i32, local_a.z.round() as i32, by, local_b.x.round() as i32, local_b.z.round() as i32, by, 2, block::OAK_PLANKS, block::OAK_LOG);

    islands.push(bridge_island);
}

/// Todo lo que hace falta saber de una isla ya construida para seguir
/// ubicando cosas relativas a ella (puentes, camara, etc).
pub struct BuiltIsland {
    pub index: usize,
    pub heightmap: Heightmap,
    pub params: IslandParams,
}

/// Genera una isla nueva, la agrega a `islands` y devuelve sus datos.
fn spawn_island(islands: &mut Vec<Island>, seed: u32, params: IslandParams, offset: (i32, i32, i32)) -> BuiltIsland {
    let (world, heightmap) = generate_island(seed, &params);
    let index = islands.len();
    islands.push(Island::new(world, offset));
    BuiltIsland { index, heightmap, params }
}

/// Punto de mundo (x,y,z) sobre el centro de una isla ya construida.
fn island_world_center(islands: &[Island], b: &BuiltIsland) -> Vec3 {
    islands[b.index].to_world_point(Vec3::new(b.params.center_x as f32, b.params.top_y as f32, b.params.center_z as f32))
}

/// Construye la isla principal (el faro, el lago, la casita) y sus dos
/// satelites (monolito y jardin) con sus puentes. Devuelve la lista de
/// islas (cada una su propia mini-grilla) y el punto de mundo donde deberia
/// mirar la camara para la isla principal.
pub fn build_lighthouse_scene(seed: u32) -> (Vec<Island>, Vec3, f32) {
    let mut islands: Vec<Island> = Vec::new();

    let main = spawn_island(&mut islands, seed, IslandParams::new(42.0, 24), (0, 0, 0));
    let (cx, cz, r) = (main.params.center_x, main.params.center_z, main.params.radius);
    let water_level_world = main.params.water_level; // offset.y == 0 para la isla principal

    let (lx, lz) = polar(cx, cz, 250.0, r * 0.5);
    clear_trees_near(&mut islands[main.index].world, lx, lz, 6);
    build_lighthouse(&mut islands[main.index].world, &main.heightmap, lx, lz);

    let (kx, kz) = polar(cx, cz, 40.0, r * 0.32);
    let lake_r = (r * 0.30) as i32;
    build_lake_and_waterfall(&mut islands[main.index].world, &main.heightmap, kx, kz, lake_r, main.params.water_level, 40.0);
    build_dock(&mut islands[main.index].world, &main.heightmap, kx, kz, lake_r, 220.0, main.params.water_level);

    let (hx, hz) = polar(cx, cz, 150.0, r * 0.45);
    clear_trees_near(&mut islands[main.index].world, hx, hz, 7);
    build_house(&mut islands[main.index].world, &main.heightmap, hx, hz);

    let path_points = [lerp_point((lx, lz), (kx, kz), 0.33), lerp_point((lx, lz), (kx, kz), 0.66), lerp_point((kx, kz), (hx, hz), 0.33), lerp_point((kx, kz), (hx, hz), 0.66)];
    build_path_lights(&mut islands[main.index].world, &main.heightmap, &path_points);

    let main_center_world = island_world_center(&islands, &main);
    let sat_gap = 14.0;
    let sat_r = 13.0f32;

    // Isla satelite A: monolito de iron_block pulido (refleja el faro y el cielo solo).
    let a_offset_xz = (main_center_world.x + (250f32.to_radians().cos() * (r + sat_gap + sat_r)), main_center_world.z + (250f32.to_radians().sin() * (r + sat_gap + sat_r)));
    let params_a = IslandParams::new(sat_r, 4);
    let offset_a = ((a_offset_xz.0 as i32) - params_a.center_x, 0, (a_offset_xz.1 as i32) - params_a.center_z);
    let sat_a = spawn_island(&mut islands, seed.wrapping_add(101), params_a, offset_a);
    clear_trees_near(&mut islands[sat_a.index].world, sat_a.params.center_x, sat_a.params.center_z, 4);
    build_statue(&mut islands[sat_a.index].world, &sat_a.heightmap, sat_a.params.center_x, sat_a.params.center_z);
    build_bridge(&mut islands, main.index, &main.heightmap, &main.params, sat_a.index, &sat_a.heightmap, &sat_a.params, water_level_world + 2);

    // Isla satelite B: jardin con fuente.
    let b_offset_xz = (main_center_world.x + (70f32.to_radians().cos() * (r + sat_gap + sat_r)), main_center_world.z + (70f32.to_radians().sin() * (r + sat_gap + sat_r)));
    let params_b = IslandParams::new(sat_r, 3);
    let offset_b = ((b_offset_xz.0 as i32) - params_b.center_x, 0, (b_offset_xz.1 as i32) - params_b.center_z);
    let sat_b = spawn_island(&mut islands, seed.wrapping_add(202), params_b, offset_b);
    clear_trees_near(&mut islands[sat_b.index].world, sat_b.params.center_x, sat_b.params.center_z, 7);
    build_garden(&mut islands[sat_b.index].world, &sat_b.heightmap, sat_b.params.center_x, sat_b.params.center_z, seed);
    build_bridge(&mut islands, main.index, &main.heightmap, &main.params, sat_b.index, &sat_b.heightmap, &sat_b.params, water_level_world + 2);

    (islands, main_center_world, r)
}
