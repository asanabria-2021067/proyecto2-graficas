//! Fase 9: "La isla del faro". Construye el faro, el lago con cascada, el
//! muelle, la casita, dos islas satelite (monolito e jardin con fuente) y
//! postes de luz, todo sobre el terreno procedural de fase 8, usando
//! funciones helper relativas a la altura real del terreno para que
//! funcionen con cualquier semilla.

use crate::material::block;
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
/// tronco cada 4 bloques a los lados), usado tanto para el muelle como para
/// los puentes entre islas.
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
        if i % 4 == 0 {
            let off = (width as f32 - 1.0) / 2.0 + 0.5;
            for sign in [-1.0f32, 1.0] {
                let wx = (cx + px * off * sign).round() as i32;
                let wz = (cz + pz * off * sign).round() as i32;
                world.fill_box((wx, y + 1, wz), (wx, y + 2, wz), post_id);
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
            if dx * dx + dz * dz > radius * radius {
                continue;
            }
            let x = cx + dx;
            let z = cz + dz;
            let Some(top) = hm.top_at(x, z) else { continue };
            if top > water_level {
                world.fill_box((x, water_level + 1, z), (x, top, z), block::AIR);
            }
            let bed_y = (water_level - 1).min(top);
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
    flatten_area(world, cx - 3, cz - 3, cx + 3, cz + 3, base_y, block::GRASS);

    let h = 4;
    let y0 = base_y + 1;
    let corners = [(-2, -2), (2, -2), (-2, 2), (2, 2)];

    world.hollow_box((cx - 2, y0, cz - 2), (cx + 2, y0 + h - 1, cz + 2), block::OAK_PLANKS);
    for &(dx, dz) in &corners {
        place_pillar(world, cx + dx, cz + dz, y0, y0 + h - 1, block::OAK_LOG);
    }

    world.set(cx, y0 + 1, cz - 2, block::GLASS);
    world.set(cx, y0 + 1, cz + 2, block::GLASS);
    world.set(cx - 2, y0 + 1, cz, block::GLASS);
    world.set(cx, y0, cz + 2, block::AIR); // puerta
    world.set(cx, y0 + 1, cz, block::GLOWSTONE); // lampara interior, se ve por las ventanas de noche

    let roof_y = y0 + h;
    for (layer, inset) in [(0, 0), (1, 1), (2, 2)] {
        world.fill_box((cx - 2 + inset, roof_y + layer, cz - 2 + inset), (cx + 2 - inset, roof_y + layer, cz + 2 - inset), block::OAK_PLANKS);
    }
    world.set(cx, roof_y + 3, cz, block::OAK_PLANKS);
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

#[allow(clippy::too_many_arguments)]
fn build_bridge(world: &mut World, hm_a: &Heightmap, hm_b: &Heightmap, ax: i32, az: i32, bx: i32, bz: i32, water_level: i32) {
    let angle_a_to_b = (bz - az) as f32;
    let angle = (angle_a_to_b).atan2((bx - ax) as f32).to_degrees();
    let edge_a = find_edge(hm_a, ax, az, angle.to_radians(), 200.0);
    let edge_b = find_edge(hm_b, bx, bz, (angle + 180.0).to_radians(), 200.0);
    let bridge_y = ((edge_a.2 + edge_b.2) / 2).max(water_level + 2);

    flatten_area(world, edge_a.0 - 1, edge_a.1 - 1, edge_a.0 + 1, edge_a.1 + 1, bridge_y, block::OAK_PLANKS);
    flatten_area(world, edge_b.0 - 1, edge_b.1 - 1, edge_b.0 + 1, edge_b.1 + 1, bridge_y, block::OAK_PLANKS);
    build_walkway(world, edge_a.0, edge_a.1, bridge_y, edge_b.0, edge_b.1, bridge_y, 2, block::OAK_PLANKS, block::OAK_LOG);
}

/// Construye la escena completa: la isla principal con el faro, el lago y la
/// cascada, el muelle, la casita, dos islas satelite (monolito y jardin) con
/// sus puentes, y postes de luz a lo largo de los caminos.
pub fn build_lighthouse_scene(world: &mut World, seed: u32) -> (Heightmap, IslandParams) {
    let island = IslandParams::main_island(world);
    let mut hm = generate_island(world, seed, &island);
    let (cx, cz, r) = (island.center_x, island.center_z, island.radius);

    let (lx, lz) = polar(cx, cz, 250.0, r * 0.5);
    build_lighthouse(world, &hm, lx, lz);

    let (kx, kz) = polar(cx, cz, 40.0, r * 0.32);
    let lake_r = (r * 0.22) as i32;
    build_lake_and_waterfall(world, &hm, kx, kz, lake_r, island.water_level, 40.0);
    build_dock(world, &hm, kx, kz, lake_r, 220.0, island.water_level);

    let (hx, hz) = polar(cx, cz, 150.0, r * 0.45);
    build_house(world, &hm, hx, hz);

    let sat_gap = 14.0;
    let sat_r = 13.0f32;

    let (sax, saz) = polar(cx, cz, 250.0, r + sat_gap + sat_r);
    let island_a = IslandParams::new(sax, saz, sat_r, island.top_y, island.water_level, sat_r * 1.8, 4);
    let hm_a = generate_island(world, seed.wrapping_add(101), &island_a);
    build_statue(world, &hm_a, sax, saz);
    build_bridge(world, &hm, &hm_a, cx, cz, sax, saz, island.water_level);
    hm.merge(&hm_a);

    let (gx, gz) = polar(cx, cz, 70.0, r + sat_gap + sat_r);
    let island_b = IslandParams::new(gx, gz, sat_r, island.top_y, island.water_level, sat_r * 1.8, 3);
    let hm_b = generate_island(world, seed.wrapping_add(202), &island_b);
    build_garden(world, &hm_b, gx, gz, seed);
    build_bridge(world, &hm, &hm_b, cx, cz, gx, gz, island.water_level);
    hm.merge(&hm_b);

    let path_points = [lerp_point((lx, lz), (kx, kz), 0.33), lerp_point((lx, lz), (kx, kz), 0.66), lerp_point((kx, kz), (hx, hz), 0.33), lerp_point((kx, kz), (hx, hz), 0.66)];
    build_path_lights(world, &hm, &path_points);

    (hm, island)
}
