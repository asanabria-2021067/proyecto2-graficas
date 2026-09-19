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
use crate::terrain::{generate_island, generate_rugged_island, generate_soft_island, Heightmap, IslandParams};
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
            // Fondo en forma de cuenco: mas hondo en el centro, se va
            // achicando hacia la orilla, para que el lago se lea como un
            // cuerpo de agua real y no un charco de un bloque. Profundidad
            // maxima de 2 (antes 3): con mas de 2 bloques de agua encima la
            // absorcion Beer-Lambert oscurecia el fondo entero incluso ya
            // con el material mas claro, y la orilla quedaba angosta.
            let dist_norm = (dist2 as f32).sqrt() / radius as f32;
            let bowl_depth = ((1.0 - dist_norm) * 2.0) as i32;
            let bed_y = (water_level - 1 - bowl_depth).min(top);
            world.set(x, bed_y, z, block::SAND);
            world.fill_box((x, bed_y + 1, z), (x, water_level, z), block::WATER);
        }
    }

    // Orilla en rampa: el radio del lago solo despeja el POZO en si (hasta
    // `water_level`), pero el terreno natural justo afuera de ese radio
    // puede estar varios bloques mas alto (el heightmap de la isla no sabe
    // que ahi va un lago) -- eso dejaba un pozo de paredes casi verticales
    // de 6-15 bloques, que con el sol tan bajo (~12 grados) se auto-sombrea
    // por completo sin importar que tan claro sea el material del agua. Se
    // rebaja un anillo alrededor del lago en rampa suave, de la orilla
    // (agua_nivel+1) hasta la altura natural, para que sea un estanque
    // abierto y no un pozo.
    let shore = 9;
    for dz in -(radius + shore)..=(radius + shore) {
        for dx in -(radius + shore)..=(radius + shore) {
            let dist = ((dx * dx + dz * dz) as f32).sqrt();
            if dist <= radius as f32 || dist > (radius + shore) as f32 {
                continue;
            }
            let x = cx + dx;
            let z = cz + dz;
            let Some(top) = hm.top_at(x, z) else { continue };
            if top <= water_level + 1 {
                continue;
            }
            let t = ((dist - radius as f32) / shore as f32).clamp(0.0, 1.0);
            let eased = t * t;
            // Jitter chico y determinista por columna para que el anillo no
            // se lea como bandas concentricas perfectas (muy artificial
            // visto desde arriba/al costado).
            let jitter = ((x.wrapping_mul(374_761_393) ^ z.wrapping_mul(668_265_263)) as u32 % 3) as i32 - 1;
            let target = (water_level as f32 + 1.0 + eased * (top - water_level - 1) as f32).round() as i32 + jitter;
            if target < top {
                world.fill_box((x, target + 1, z), (x, top, z), block::AIR);
                let shore_block = if t < 0.5 { block::SAND } else { block::GRASS };
                world.set(x, target, z, shore_block);
            }
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

// ---------- Pueblo (parte 2) ----------

/// Techo a dos aguas: dos faldones que suben desde los muros largos hacia un
/// caballete central, con 1 bloque de alero que sobresale del muro a cada
/// lado. `ridge_along_x` decide si el caballete corre en X (el techo sube/
/// baja en Z) o en Z (sube/baja en X).
#[allow(clippy::too_many_arguments)]
fn build_gable_roof(world: &mut World, cx: i32, cz: i32, half_x: i32, half_z: i32, roof_y0: i32, ridge_along_x: bool, id: u8) {
    if ridge_along_x {
        let ortho_half = half_z + 1;
        let span_half = half_x + 1;
        for o in -ortho_half..=ortho_half {
            let y = roof_y0 + (ortho_half - o.abs());
            world.fill_box((cx - span_half, y, cz + o), (cx + span_half, y, cz + o), id);
        }
    } else {
        let ortho_half = half_x + 1;
        let span_half = half_z + 1;
        for o in -ortho_half..=ortho_half {
            let y = roof_y0 + (ortho_half - o.abs());
            world.fill_box((cx + o, y, cz - span_half), (cx + o, y, cz + span_half), id);
        }
    }
}

/// Casa de pueblo: zocalo de cobblestone, paredes de oak_planks con vigas de
/// oak_log en las 4 esquinas y un cinturon de troncos bajo el techo, techo a
/// dos aguas con alero, 2-3 ventanas de vidrio con marco de tronco, puerta al
/// frente y un farol junto a la entrada. Interior hueco (paredes con
/// `hollow_box`, no solidas) para que las ventanas dejen ver adentro, con un
/// farol propio ademas. `variant` (0/1/2) cambia footprint y orientacion del
/// techo para que las 3+ variantes pedidas no se vean todas iguales.
fn build_village_house(world: &mut World, hm: &Heightmap, cx: i32, cz: i32, variant: u32) {
    let base_y = hm.top_at(cx, cz).unwrap_or(world.ny / 2);
    let (half_w, half_d, wall_h, ridge_along_x) = match variant % 3 {
        0 => (2, 1, 4, true),
        1 => (1, 2, 4, false),
        _ => (2, 2, 5, true),
    };
    flatten_area(world, cx - half_w - 1, cz - half_d - 1, cx + half_w + 1, cz + half_d + 1, base_y, block::GRASS);

    world.fill_box((cx - half_w, base_y, cz - half_d), (cx + half_w, base_y, cz + half_d), block::COBBLESTONE);

    let y0 = base_y + 1;
    let y1 = y0 + wall_h - 1;
    world.hollow_box((cx - half_w, y0, cz - half_d), (cx + half_w, y1, cz + half_d), block::OAK_PLANKS);

    for &(dx, dz) in &[(-half_w, -half_d), (half_w, -half_d), (-half_w, half_d), (half_w, half_d)] {
        place_pillar(world, cx + dx, cz + dz, y0, y1, block::OAK_LOG);
    }
    world.hollow_box((cx - half_w, y1, cz - half_d), (cx + half_w, y1, cz + half_d), block::OAK_LOG);

    // Puerta al frente (+z), de 2 de alto.
    world.fill_box((cx, y0, cz + half_d), (cx, y0 + 1, cz + half_d), block::AIR);

    // Ventanas con marco de tronco en los lados que sobran (2-3 segun tamano).
    let wy = y0 + 1;
    world.set(cx - half_w, wy, cz, block::GLASS);
    world.set(cx + half_w, wy, cz, block::GLASS);
    world.set(cx, wy, cz - half_d, block::GLASS);
    if half_w >= 3 {
        world.set(cx - half_w, wy, cz - 1, block::GLASS);
    }

    // Farol junto a la entrada y luz interior (se ve por las ventanas de noche).
    world.set(cx + 1, y0, cz + half_d + 1, block::LANTERN);
    world.set(cx, y0 + 1, cz, block::LANTERN);

    build_gable_roof(world, cx, cz, half_w, half_d, y1 + 1, ridge_along_x, block::OAK_PLANKS);
}

/// Construccion de piedra mas grande, centro del pueblo: torre/capilla de
/// cobblestone y stone_bricks con techo piramidal de tablones, ventanas y un
/// farol en la punta. Coexiste con el faro (que se queda donde esta).
fn build_village_tower(world: &mut World, hm: &Heightmap, cx: i32, cz: i32) {
    let base_y = hm.top_at(cx, cz).unwrap_or(world.ny / 2);
    let r = 2;
    flatten_area(world, cx - r - 1, cz - r - 1, cx + r + 1, cz + r + 1, base_y, block::COBBLESTONE);

    let y0 = base_y + 1;
    let wall_h = 8;
    let y1 = y0 + wall_h - 1;
    world.hollow_box((cx - r, y0, cz - r), (cx + r, y1, cz + r), block::STONE_BRICKS);
    for &(dx, dz) in &[(-r, -r), (r, -r), (-r, r), (r, r)] {
        place_pillar(world, cx + dx, cz + dz, y0, y1, block::COBBLESTONE);
    }
    // Puerta y ventanas.
    world.fill_box((cx, y0, cz - r), (cx, y0 + 1, cz - r), block::AIR);
    for &off in &[-1, 1] {
        world.set(cx + off, y0 + 3, cz - r, block::GLASS);
        world.set(cx + off, y0 + 3, cz + r, block::GLASS);
        world.set(cx - r, y0 + 3, cz + off, block::GLASS);
        world.set(cx + r, y0 + 3, cz + off, block::GLASS);
    }

    // Techo piramidal SOLIDO de tablones (cada capa un poco mas chica que la
    // anterior), con farol en la punta.
    let mut y = y1 + 1;
    for inset in 0..r {
        world.fill_box((cx - r + inset, y, cz - r + inset), (cx + r - inset, y, cz + r - inset), block::OAK_PLANKS);
        y += 1;
    }
    world.set(cx, y, cz, block::LANTERN);
}

/// Parcela de cultivo: rectangulo de farmland con un canal de agua de 1
/// bloque por el medio, filas de `crops` en el resto, y una cerca de postes
/// de oak_log alrededor (el enunciado permite usar oak_log delgado en vez de
/// un material de cerca aparte).
fn build_farm_plot(world: &mut World, hm: &Heightmap, cx: i32, cz: i32, half_w: i32, half_d: i32) {
    let base_y = hm.top_at(cx, cz).unwrap_or(world.ny / 2);
    flatten_area(world, cx - half_w, cz - half_d, cx + half_w, cz + half_d, base_y, block::FARMLAND);

    for dz in -half_d..=half_d {
        world.set(cx, base_y, cz + dz, block::WATER);
    }
    for dz in -half_d..=half_d {
        for dx in -half_w..=half_w {
            if dx == 0 {
                continue; // canal
            }
            world.set(cx + dx, base_y + 1, cz + dz, block::CROPS);
        }
    }

    for dx in -half_w - 1..=half_w + 1 {
        let post = dx.rem_euclid(2) == 0 || dx == -half_w - 1 || dx == half_w + 1;
        if post {
            world.set(cx + dx, base_y + 1, cz - half_d - 1, block::OAK_LOG);
            world.set(cx + dx, base_y + 1, cz + half_d + 1, block::OAK_LOG);
        }
    }
    for dz in -half_d..=half_d {
        let post = dz.rem_euclid(2) == 0;
        if post {
            world.set(cx - half_w - 1, base_y + 1, cz + dz, block::OAK_LOG);
            world.set(cx + half_w + 1, base_y + 1, cz + dz, block::OAK_LOG);
        }
    }
}

/// Pozo de piedra con agua en el centro del pueblo: anillo de cobblestone
/// hueco con un espejo de agua adentro, 4 postes de tronco y un techito
/// piramidal de tablones.
fn build_well(world: &mut World, hm: &Heightmap, cx: i32, cz: i32) {
    let base_y = hm.top_at(cx, cz).unwrap_or(world.ny / 2);
    flatten_area(world, cx - 2, cz - 2, cx + 2, cz + 2, base_y, block::COBBLESTONE);
    world.hollow_box((cx - 1, base_y + 1, cz - 1), (cx + 1, base_y + 2, cz + 1), block::COBBLESTONE);
    world.fill_box((cx - 1, base_y + 1, cz - 1), (cx + 1, base_y + 1, cz + 1), block::WATER);

    for &(dx, dz) in &[(-1, -1), (1, -1), (-1, 1), (1, 1)] {
        place_pillar(world, cx + dx, cz + dz, base_y + 3, base_y + 5, block::OAK_LOG);
    }
    world.fill_box((cx - 2, base_y + 6, cz - 2), (cx + 2, base_y + 6, cz + 2), block::OAK_PLANKS);
    world.fill_box((cx - 1, base_y + 7, cz - 1), (cx + 1, base_y + 7, cz + 1), block::OAK_PLANKS);
    world.set(cx, base_y + 8, cz, block::OAK_PLANKS);
}

/// Camino de tierra/grava de `width` bloques que sigue la altura real del
/// terreno entre dos puntos, sin tocar agua ni estructuras (solo pisa
/// pasto/tierra/arena) -- para no dejar un camino flotando sobre el lago o
/// atravesando una pared.
fn build_ground_path(world: &mut World, hm: &Heightmap, x0: i32, z0: i32, x1: i32, z1: i32, width: i32) {
    let dx = x1 - x0;
    let dz = z1 - z0;
    let steps = dx.abs().max(dz.abs()).max(1);
    let len = ((dx * dx + dz * dz) as f32).sqrt().max(1.0);
    let (px, pz) = (-(dz as f32) / len, (dx as f32) / len);
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let cx = x0 as f32 + dx as f32 * t;
        let cz = z0 as f32 + dz as f32 * t;
        for w in 0..width {
            let off = w as f32 - (width as f32 - 1.0) / 2.0;
            let wx = (cx + px * off).round() as i32;
            let wz = (cz + pz * off).round() as i32;
            let Some(y) = hm.top_at(wx, wz) else { continue };
            let cur = world.get(wx, y, wz);
            if cur == block::GRASS || cur == block::SAND || cur == block::DIRT {
                world.set(wx, y, wz, block::DIRT_PATH);
                // Si habia un arbol parado justo en esta columna (plantado
                // antes de que existiera el camino), lo saca -- el enunciado
                // pide que no queden arboles encima de caminos ni parcelas.
                for dy in 1..7 {
                    let id = world.get(wx, y + dy, wz);
                    if id == block::OAK_LOG || id == block::LEAVES {
                        world.set(wx, y + dy, wz, block::AIR);
                    }
                }
            }
        }
    }
}

// ---------- Nether ----------

/// Lago/rio de lava con cascada por el borde, igual que `build_lake_and_waterfall`
/// pero con lava (que ademas ilumina como luz puntual, ver `lights.rs`) y
/// lecho de netherrack en vez de arena.
fn build_lava_lake_and_falls(world: &mut World, hm: &Heightmap, cx: i32, cz: i32, radius: i32, lava_level: i32, outward_angle_deg: f32) {
    for dz in -radius..=radius {
        for dx in -radius..=radius {
            let dist2 = dx * dx + dz * dz;
            if dist2 > radius * radius {
                continue;
            }
            let x = cx + dx;
            let z = cz + dz;
            let Some(top) = hm.top_at(x, z) else { continue };
            if top > lava_level {
                world.fill_box((x, lava_level + 1, z), (x, top, z), block::AIR);
            }
            let dist_norm = (dist2 as f32).sqrt() / radius as f32;
            let bowl_depth = ((1.0 - dist_norm) * 2.0) as i32;
            let bed_y = (lava_level - 1 - bowl_depth).min(top);
            world.set(x, bed_y, z, block::NETHERRACK);
            world.fill_box((x, bed_y + 1, z), (x, lava_level, z), block::LAVA);
        }
    }

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
            if top > lava_level {
                world.fill_box((x, lava_level, z), (x, top, z), block::AIR);
            }
            world.set(x, lava_level, z, block::LAVA);
        }
        t += 1.0;
    }

    for w in -1..=1 {
        let x = edge_x + (perp_x * w as f32).round() as i32;
        let z = edge_z + (perp_z * w as f32).round() as i32;
        // La cascada en si (3 de ancho) llega hasta 22 bloques; el carril
        // central sigue solo, como un hilo delgado de lava que se angosta
        // y sigue cayendo hacia el vacio bastante mas abajo de la isla.
        let depth_max = if w == 0 { 40 } else { 22 };
        for depth in 0..depth_max {
            let y = lava_level - depth;
            if y < 1 {
                break;
            }
            world.set(x, y, z, block::LAVA);
        }
    }

    // Hilo colgante: por debajo de la cascada de 3 de ancho, el carril
    // central sigue solo cayendo -- antes era una sola columna recta (se leia
    // como una barra plana). Ahora el grosor varia entre 1 y 2 bloques y se
    // bambolea un poco a los lados (jitter determinista por profundidad), como
    // un hilo de lava real en vez de una linea rigida.
    for depth in 22..40 {
        let y = lava_level - depth;
        if y < 1 {
            break;
        }
        let h1 = hash_i32(edge_x, edge_z, depth * 7 + 1) % 3 - 1;
        let h2 = hash_i32(edge_x, edge_z, depth * 7 + 2) % 3 - 1;
        let wide = hash_i32(edge_x, edge_z, depth * 7 + 3) % 4 == 0;
        let cx = edge_x + (perp_x * h1 as f32).round() as i32;
        let cz = edge_z + (perp_z * h1 as f32).round() as i32;
        world.set(cx, y, cz, block::LAVA);
        if wide {
            let ex = cx + (perp_x * h2 as f32).round() as i32 + (dx * 0.4).round() as i32;
            let ez = cz + (perp_z * h2 as f32).round() as i32 + (dz * 0.4).round() as i32;
            world.set(ex, y, ez, block::LAVA);
        }
    }
}

/// Hash entero determinista chico (no correlacionado con el ruido Perlin de
/// la isla) para jitter reproducible por columna/profundidad, sin necesitar
/// pasar un `Pcg32` mutable a traves de toda la cadena de llamadas.
#[inline]
fn hash_i32(x: i32, z: i32, salt: i32) -> i32 {
    let h = (x as u32).wrapping_mul(374_761_393) ^ (z as u32).wrapping_mul(668_265_263) ^ (salt as u32).wrapping_mul(2_246_822_519);
    let h = h ^ (h >> 15);
    (h % 1_000_000) as i32
}

/// Portal del Nether: marco de obsidiana de 4 de ancho x 5 de alto con
/// material `portal` rellenando el hueco interior (2x3).
fn build_nether_portal(world: &mut World, hm: &Heightmap, cx: i32, cz: i32) {
    let base_y = hm.top_at(cx, cz).unwrap_or(world.ny / 2);
    flatten_area(world, cx - 3, cz - 3, cx + 3, cz + 3, base_y, block::NETHERRACK);
    let y0 = base_y + 1;
    world.fill_box((cx - 2, y0, cz), (cx + 1, y0 + 4, cz), block::OBSIDIAN);
    world.fill_box((cx - 1, y0 + 1, cz), (cx, y0 + 3, cz), block::PORTAL);
}

/// Reemplaza netherrack por nylium (carmesi o distorsionado) en un parche
/// circular -- el suelo bajo cada arbol hongo gigante.
fn build_nylium_patch(world: &mut World, hm: &Heightmap, cx: i32, cz: i32, r: i32, nylium_id: u8) {
    for dz in -r..=r {
        for dx in -r..=r {
            if dx * dx + dz * dz > r * r {
                continue;
            }
            let x = cx + dx;
            let z = cz + dz;
            let Some(y) = hm.top_at(x, z) else { continue };
            if world.get(x, y, z) == block::NETHERRACK {
                world.set(x, y, z, nylium_id);
            }
        }
    }
}

/// Arbol hongo gigante (carmesi o distorsionado segun los materiales que se
/// le pasen): tronco delgado, copa irregular de 3 capas con ramas que caen
/// por el borde, y shroomlights emisivos en la punta y adentro de la copa.
#[allow(clippy::too_many_arguments)]
fn build_fungus_tree(world: &mut World, hm: &Heightmap, cx: i32, cz: i32, stem_id: u8, wart_id: u8, shroom_id: u8, height: i32, seed: u32) {
    let base_y = hm.top_at(cx, cz).unwrap_or(world.ny / 2);
    place_pillar(world, cx, cz, base_y + 1, base_y + height, stem_id);

    let mut rng = Pcg32::new(seed as u64 ^ 0x7075_1CA9, 0xFE);
    let canopy_y = base_y + height - 2;
    let r = 3;
    for layer in 0..3 {
        let cy = canopy_y + layer;
        let lr = if layer == 2 { r - 1 } else { r };
        for dz in -lr..=lr {
            for dx in -lr..=lr {
                let d2 = dx * dx + dz * dz;
                if d2 > lr * lr || (dx == 0 && dz == 0) {
                    continue;
                }
                if d2 == lr * lr && rng.next_f32() < 0.35 {
                    continue;
                }
                world.set(cx + dx, cy, cz + dz, wart_id);
            }
        }
    }
    for _ in 0..4 {
        let angle = rng.range_f32(0.0, std::f32::consts::PI * 2.0);
        let dist = r as f32 * rng.range_f32(0.7, 1.0);
        let dx = (angle.cos() * dist).round() as i32;
        let dz = (angle.sin() * dist).round() as i32;
        let drop = rng.range_i32(1, 2);
        world.fill_box((cx + dx, canopy_y - drop, cz + dz), (cx + dx, canopy_y - 1, cz + dz), wart_id);
    }
    world.set(cx, base_y + height + 1, cz, shroom_id);
    let inner_angle = rng.range_f32(0.0, std::f32::consts::PI * 2.0);
    let ix = (inner_angle.cos() * 1.5).round() as i32;
    let iz = (inner_angle.sin() * 1.5).round() as i32;
    world.set(cx + ix, canopy_y + 1, cz + iz, shroom_id);
}

/// Formacion de roca de blackstone/basalto con una fuente de lava en la
/// punta que cae como cascada por un costado hasta un charco al pie.
fn build_blackstone_formation(world: &mut World, hm: &Heightmap, cx: i32, cz: i32, seed: u32) {
    let base_y = hm.top_at(cx, cz).unwrap_or(world.ny / 2);
    let mut rng = Pcg32::new(seed as u64 ^ 0xB1AC_057E, 0x99);
    let r = 3;
    let peak_h = base_y + 5;
    for dz in -r..=r {
        for dx in -r..=r {
            let d2 = dx * dx + dz * dz;
            if d2 > r * r {
                continue;
            }
            let Some(top) = hm.top_at(cx + dx, cz + dz) else { continue };
            let h = (peak_h as f32 - (d2 as f32).sqrt() * 1.3).round() as i32;
            let mat = if rng.next_f32() < 0.3 { block::BASALT } else { block::BLACKSTONE };
            world.fill_box((cx + dx, top + 1, cz + dz), (cx + dx, h.max(top + 1), cz + dz), mat);
        }
    }
    let pool_r = 2;
    let (pcx, pcz) = (cx + r + 2, cz);
    for dz in -pool_r..=pool_r {
        for dx in -pool_r..=pool_r {
            if dx * dx + dz * dz > pool_r * pool_r {
                continue;
            }
            let Some(top) = hm.top_at(pcx + dx, pcz + dz) else { continue };
            world.set(pcx + dx, top, pcz + dz, block::LAVA);
        }
    }
    for y in base_y..=peak_h {
        world.set(cx + r, y, cz, block::LAVA);
    }
    world.set(cx, peak_h + 1, cz, block::LAVA);
}

/// Fuegos naranjas dispersos sobre netherrack/nylium (bloque con alpha
/// cutout, sin geometria de cruz explicita).
fn scatter_fire(world: &mut World, hm: &Heightmap, cx: i32, cz: i32, radius: f32, count: u32, seed: u32) {
    let mut rng = Pcg32::new(seed as u64 ^ 0xF17E_0000, 0x42);
    let mut placed = 0;
    let mut attempts = 0;
    while placed < count && attempts < count * 20 {
        attempts += 1;
        let x = cx + rng.range_i32(-radius as i32, radius as i32);
        let z = cz + rng.range_i32(-radius as i32, radius as i32);
        let Some(y) = hm.top_at(x, z) else { continue };
        let top = world.get(x, y, z);
        if top != block::NETHERRACK && top != block::CRIMSON_NYLIUM && top != block::WARPED_NYLIUM {
            continue;
        }
        world.set(x, y + 1, z, block::FIRE);
        placed += 1;
    }
}

/// Parche de soul_sand con un par de fuegos de alma azules, para contraste
/// de color con los fuegos naranjas.
fn build_soul_fire_patch(world: &mut World, hm: &Heightmap, cx: i32, cz: i32, r: i32, seed: u32) {
    let mut rng = Pcg32::new(seed as u64 ^ 0x50F1_0000, 0x17);
    for dz in -r..=r {
        for dx in -r..=r {
            if dx * dx + dz * dz > r * r {
                continue;
            }
            let x = cx + dx;
            let z = cz + dz;
            let Some(y) = hm.top_at(x, z) else { continue };
            world.set(x, y, z, block::SOUL_SAND);
            if rng.next_f32() < 0.35 {
                world.set(x, y + 1, z, block::SOUL_FIRE);
            }
        }
    }
}

/// Camino en zig-zag de blackstone que cuelga por debajo del borde real de
/// la isla, bajando en escalones como una raiz o escalera colgante.
fn build_hanging_zigzag_path(world: &mut World, hm: &Heightmap, cx: i32, cz: i32, start_angle_deg: f32, segments: i32) {
    let dir = start_angle_deg.to_radians();
    let (ex, ez, ey) = find_edge(hm, cx, cz, dir, 200.0);
    let (mut x, mut z, mut y) = (ex, ez, ey - 1);
    let (fwd_x, fwd_z) = (dir.cos(), dir.sin());
    let (perp_x, perp_z) = (-fwd_z, fwd_x);
    let seg_len = 3;
    for i in 0..segments {
        let side = if i % 2 == 0 { 1.0 } else { -1.0 };
        for _ in 0..seg_len {
            x += (perp_x * side).round() as i32;
            z += (perp_z * side).round() as i32;
            world.set(x, y, z, block::BLACKSTONE);
            y -= 1;
        }
        x += fwd_x.round() as i32;
        z += fwd_z.round() as i32;
        world.set(x, y, z, block::BLACKSTONE);
    }
}

/// Cuelga glowstone del borde real de la isla (calculado con `find_edge`,
/// asi funciona con cualquier semilla), unos bloques por debajo del filo.
fn hang_glowstone_edge(world: &mut World, hm: &Heightmap, cx: i32, cz: i32, radius: f32, count: u32, seed: u32) {
    let mut rng = Pcg32::new(seed as u64 ^ 0x9E17_44A0, 0x51ED);
    for _ in 0..count {
        let angle = rng.range_f32(0.0, std::f32::consts::PI * 2.0);
        let (ex, ez, ey) = find_edge(hm, cx, cz, angle, radius + 40.0);
        world.set(ex, ey - 2, ez, block::GLOWSTONE);
    }
}

// ---------- End ----------

/// Pilares de obsidiana de altura variable con un `end_crystal` flotando
/// (con un hueco de aire encima, no pegado) sobre cada uno -- en el borde
/// de la isla para no taparle el frente a la ciudad de torres.
fn build_end_pillars(world: &mut World, hm: &Heightmap, cx: i32, cz: i32, radius: f32, seed: u32) {
    let mut rng = Pcg32::new(seed as u64 ^ 0x3ED0_C1A1, 0x9E17);
    let count = 4;
    for i in 0..count {
        let angle = (i as f32 / count as f32) * std::f32::consts::PI * 2.0 + rng.range_f32(-0.2, 0.2);
        let dist = radius * rng.range_f32(0.75, 0.92);
        let px = cx + (angle.cos() * dist).round() as i32;
        let pz = cz + (angle.sin() * dist).round() as i32;
        let Some(base_y) = hm.top_at(px, pz) else { continue };
        let h = rng.range_i32(6, 12);
        place_pillar(world, px, pz, base_y + 1, base_y + h, block::OBSIDIAN);
        world.set(px, base_y + h + 2, pz, block::END_CRYSTAL);
    }
}

/// Torre central de la ciudad: base de purpur_block y una columna delgada
/// de purpur_pillar bien alta, rematada en un end_rod. Devuelve la altura
/// de la base para anclar las escaleras diagonales.
fn build_central_tower(world: &mut World, hm: &Heightmap, cx: i32, cz: i32, height: i32) -> i32 {
    let base_y = hm.top_at(cx, cz).unwrap_or(world.ny / 2);
    flatten_area(world, cx - 3, cz - 3, cx + 3, cz + 3, base_y, block::PURPUR);
    world.fill_box((cx - 1, base_y + 1, cz - 1), (cx + 1, base_y + 2, cz + 1), block::PURPUR);
    place_pillar(world, cx, cz, base_y + 3, base_y + height, block::PURPUR_PILLAR);
    world.set(cx, base_y + height + 1, cz, block::END_ROD);
    base_y
}

/// Torre secundaria de la ciudad: `floors` pisos de 7x7 apilados con
/// paredes de end_stone_bricks, un alero de purpur que sobresale 1 bloque
/// en la base de cada piso (con end_rods en sus esquinas) y ventanas de
/// magenta_glass. `pointed` remata en un techo escalonado con punta en vez
/// de quedar plano. Devuelve la altura de la base para las escaleras.
fn build_city_tower(world: &mut World, hm: &Heightmap, cx: i32, cz: i32, floors: i32, pointed: bool) -> i32 {
    let base_y = hm.top_at(cx, cz).unwrap_or(world.ny / 2);
    flatten_area(world, cx - 4, cz - 4, cx + 4, cz + 4, base_y, block::PURPUR);
    let r = 3; // piso de 7x7
    let mut y = base_y + 1;
    for floor in 0..floors {
        world.hollow_box((cx - r - 1, y, cz - r - 1), (cx + r + 1, y, cz + r + 1), block::PURPUR);
        for &(dx, dz) in &[(-r - 1, -r - 1), (r + 1, -r - 1), (-r - 1, r + 1), (r + 1, r + 1)] {
            world.set(cx + dx, y + 1, cz + dz, block::END_ROD);
        }
        y += 1;
        let floor_h = 3;
        world.hollow_box((cx - r, y, cz - r), (cx + r, y + floor_h - 1, cz + r), block::END_STONE_BRICKS);
        for &off in &[-1, 1] {
            world.set(cx + off, y + 1, cz - r, block::MAGENTA_GLASS);
            world.set(cx + off, y + 1, cz + r, block::MAGENTA_GLASS);
            world.set(cx - r, y + 1, cz + off, block::MAGENTA_GLASS);
            world.set(cx + r, y + 1, cz + off, block::MAGENTA_GLASS);
        }
        if floor == 0 {
            world.fill_box((cx, y, cz - r), (cx, y + 1, cz - r), block::AIR); // puerta
        }
        y += floor_h;
    }
    if pointed {
        let cap_r = 4;
        world.hollow_box((cx - cap_r, y, cz - cap_r), (cx + cap_r, y, cz + cap_r), block::PURPUR);
        y += 1;
        for inset in 1..cap_r {
            world.fill_box((cx - cap_r + inset, y, cz - cap_r + inset), (cx + cap_r - inset, y, cz + cap_r - inset), block::PURPUR);
            y += 1;
        }
        world.set(cx, y, cz, block::END_ROD);
    }
    base_y
}

/// Escalera diagonal de purpur (2 de ancho) entre dos puntos, con un
/// pasamanos de purpur_pillar de un lado -- conecta la torre central con
/// cada torre secundaria.
fn build_diagonal_stair(world: &mut World, x0: i32, z0: i32, y0: i32, x1: i32, z1: i32, y1: i32) {
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
        for w in 0..2 {
            let off = w as f32 - 0.5;
            let wx = (cx + px * off).round() as i32;
            let wz = (cz + pz * off).round() as i32;
            world.set(wx, y, wz, block::PURPUR);
        }
        let rx = (cx + px).round() as i32;
        let rz = (cz + pz).round() as i32;
        world.set(rx, y + 1, rz, block::PURPUR_PILLAR);
    }
}

/// Barco chico de purpur flotando cerca de la ciudad (sin apoyarse en el
/// suelo, como los pilares con cristal): casco alargado con proa, mastil
/// de purpur_pillar con una "vela"/cartel de end_stone_bricks y end_rods
/// de luz.
fn build_purpur_ship(world: &mut World, hm: &Heightmap, cx: i32, cz: i32) {
    let base_y = hm.top_at(cx, cz).unwrap_or(world.ny / 2) + 3;
    for i in -4i32..=4 {
        let width = if i.abs() > 3 { 1 } else { 2 };
        world.fill_box((cx + i, base_y, cz - width), (cx + i, base_y, cz + width), block::PURPUR);
    }
    world.set(cx + 5, base_y, cz, block::PURPUR);
    world.set(cx + 6, base_y, cz, block::PURPUR_PILLAR);
    place_pillar(world, cx, cz, base_y + 1, base_y + 5, block::PURPUR_PILLAR);
    world.fill_box((cx, base_y + 4, cz), (cx + 2, base_y + 4, cz), block::END_STONE_BRICKS);
    world.set(cx + 2, base_y + 5, cz, block::END_ROD);
    world.set(cx - 3, base_y + 1, cz, block::END_ROD);
}

/// Bosquecito de plantas chorus: tallos (`chorus_plant`) que crecen hacia
/// arriba con alguna desviacion lateral al azar, rematados en una flor
/// (`chorus`, alpha cutout).
fn build_chorus_grove(world: &mut World, hm: &Heightmap, cx: i32, cz: i32, count: u32, seed: u32) {
    let mut rng = Pcg32::new(seed as u64 ^ 0x6C02_C0DE, 0x0E27);
    let mut placed = 0;
    let mut attempts = 0;
    while placed < count && attempts < count * 20 {
        attempts += 1;
        let x = cx + rng.range_i32(-4, 4);
        let z = cz + rng.range_i32(-4, 4);
        let Some(y) = hm.top_at(x, z) else { continue };
        if world.get(x, y, z) != block::END_STONE {
            continue;
        }
        let (mut px, mut pz, mut py) = (x, z, y);
        let h = rng.range_i32(4, 7);
        for _ in 0..h {
            py += 1;
            if rng.next_f32() < 0.3 {
                px += rng.range_i32(-1, 1);
                pz += rng.range_i32(-1, 1);
            }
            world.set(px, py, pz, block::CHORUS_PLANT);
        }
        world.set(px, py + 1, pz, block::CHORUS);
        placed += 1;
    }
}

/// 2-3 mini-islas de end_stone (grillas propias, no bloques sueltos)
/// flotando alrededor de la isla principal del End, a distintas alturas.
fn build_mini_end_islands(islands: &mut Vec<Island>, center_world: Vec3, main_radius: f32, seed: u32) {
    let mut rng = Pcg32::new(seed as u64 ^ 0xE1D0_15DE, 0x33);
    let count = rng.range_i32(2, 3);
    for _ in 0..count {
        let angle = rng.range_f32(0.0, std::f32::consts::PI * 2.0);
        let dist = main_radius + rng.range_f32(6.0, 14.0);
        let radius = rng.range_i32(3, 5);
        let cx_w = center_world.x + angle.cos() * dist;
        let cz_w = center_world.z + angle.sin() * dist;
        let cy_w = center_world.y + rng.range_f32(-6.0, 8.0);
        let half = radius + 2;
        let mut mini = World::new(half * 2 + 1, 6, half * 2 + 1);
        for dz in -radius..=radius {
            for dx in -radius..=radius {
                if dx * dx + dz * dz > radius * radius {
                    continue;
                }
                mini.fill_box((half + dx, 0, half + dz), (half + dx, 2, half + dz), block::END_STONE);
            }
        }
        let offset = (cx_w.round() as i32 - half, cy_w.round() as i32 - 3, cz_w.round() as i32 - half);
        islands.push(Island::new(mini, offset));
    }
}

/// Parametros de un estilo de puente: que materiales usa y si el tablero
/// sube en el medio (arco, puentes "de piedra") o cuelga (catenaria,
/// puentes colgantes).
pub struct BridgeStyle {
    pub deck_id: u8,
    pub rail_id: u8,
    pub support_id: u8,
    pub lamp_id: u8,
    pub arch: bool,
    pub curve_amount: f32,
}

/// Cuanto puede llegar a medir un solo tramo antes de que `build_bridge`
/// meta una mini-isla de descanso en el medio (de a una por cada tramo que
/// siga siendo demasiado largo, hasta 2).
const BRIDGE_MAX_SPAN: f32 = 40.0;

/// Isla de descanso chica y plana (disco de end_stone) para partir un
/// puente demasiado largo en varios tramos mas cortos.
fn build_rest_island(islands: &mut Vec<Island>, center_world: Vec3, r: i32) -> Vec3 {
    let half = r + 2;
    let size = half * 2 + 1;
    let mut world = World::new(size, 6, size);
    for dz in -r..=r {
        for dx in -r..=r {
            if dx * dx + dz * dz > r * r {
                continue;
            }
            world.fill_box((half + dx, 0, half + dz), (half + dx, 2, half + dz), block::END_STONE_BRICKS);
            world.set(half + dx, 3, half + dz, block::END_STONE);
        }
    }
    let offset = (center_world.x.round() as i32 - half, center_world.y.round() as i32 - 3, center_world.z.round() as i32 - half);
    islands.push(Island::new(world, offset));
    Vec3::new(center_world.x.round(), center_world.y.round(), center_world.z.round())
}

/// Un solo tramo de puente entre dos puntos de MUNDO ya conocidos, como su
/// propia mini-grilla: tablero de 3 de ancho que sigue una curva (catenaria
/// o arco segun `style`), pasamanos continuo a los dos lados con postes
/// cada 3 pasos, linternas emisivas cada 6, y subestructura visible debajo
/// (vigas cruzadas + cuerdas colgando de pilares altos para los colgantes;
/// arcos de soporte + una estalactita en el punto mas alto para los de
/// piedra) -- para que se lea como un puente real desde lejos, no una
/// linea de bloques sueltos.
fn build_bridge_span(islands: &mut Vec<Island>, a: Vec3, b: Vec3, style: &BridgeStyle) {
    let dx = b.x - a.x;
    let dz = b.z - a.z;
    let horiz_len = (dx * dx + dz * dz).sqrt().max(1.0);
    let steps = horiz_len.round().max(1.0) as i32;
    let (perp_x, perp_z) = (-dz / horiz_len, dx / horiz_len);

    let margin = 5;
    let min_x = a.x.min(b.x) as i32 - margin;
    let max_x = a.x.max(b.x) as i32 + margin;
    let min_z = a.z.min(b.z) as i32 - margin;
    let max_z = a.z.max(b.z) as i32 + margin;
    let extra_up = if style.arch { style.curve_amount } else { 9.0 };
    let min_y = (a.y.min(b.y) - style.curve_amount - 7.0) as i32;
    let max_y = (a.y.max(b.y) + extra_up + 2.0) as i32;
    let offset = (min_x, min_y, min_z);
    let mut span = Island::new(World::new((max_x - min_x).max(4), (max_y - min_y).max(16), (max_z - min_z).max(4)), offset);

    let local_a = span.to_local_point(a);
    let local_b = span.to_local_point(b);

    // Pilares altos en cada extremo, de donde "cuelgan" las cuerdas del
    // puente colgante -- no hace falta para el de arco (su soporte va
    // debajo del tablero, no arriba).
    if !style.arch {
        for &sign in &[-1.0f32, 1.0] {
            for (lx, lz, ly) in [(local_a.x, local_a.z, local_a.y), (local_b.x, local_b.z, local_b.y)] {
                let px = (lx + perp_x * 1.5 * sign).round() as i32;
                let pz = (lz + perp_z * 1.5 * sign).round() as i32;
                place_pillar(&mut span.world, px, pz, ly.round() as i32 + 1, ly.round() as i32 + 8, style.rail_id);
            }
        }
    }

    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let cx = local_a.x + (local_b.x - local_a.x) * t;
        let cz = local_a.z + (local_b.z - local_a.z) * t;
        let base_deck_y = local_a.y + (local_b.y - local_a.y) * t;
        let shape = 4.0 * t * (1.0 - t); // 0 en los extremos, 1 en el medio
        let deck_y = if style.arch { (base_deck_y + style.curve_amount * shape).round() as i32 } else { (base_deck_y - style.curve_amount * shape).round() as i32 };

        for w in -1..=1 {
            let wx = (cx + perp_x * w as f32).round() as i32;
            let wz = (cz + perp_z * w as f32).round() as i32;
            span.world.set(wx, deck_y, wz, style.deck_id);
        }

        // Pasamanos continuo a ambos lados (se nota como puente desde
        // lejos, no como una fila de bloques sueltos), con postes mas
        // altos cada 3 pasos para que no quede parejo/monotono.
        for sign in [-1.0f32, 1.0] {
            let rx = (cx + perp_x * 1.5 * sign).round() as i32;
            let rz = (cz + perp_z * 1.5 * sign).round() as i32;
            if i % 3 == 0 {
                span.world.fill_box((rx, deck_y + 1, rz), (rx, deck_y + 3, rz), style.rail_id);
            } else {
                span.world.set(rx, deck_y + 1, rz, style.rail_id);
            }
        }

        // Linternas emisivas cada 6 pasos, alternando de lado.
        if i % 6 == 3 {
            let sign = if (i / 6) % 2 == 0 { 1.0 } else { -1.0 };
            let lx = (cx + perp_x * 1.5 * sign).round() as i32;
            let lz = (cz + perp_z * 1.5 * sign).round() as i32;
            span.world.set(lx, deck_y + 3, lz, style.lamp_id);
        }

        if style.arch {
            // Arcos de soporte cada 5 pasos (no en las puntas): dos
            // pilares bajando desde el tablero con un dintel que los une.
            if i % 5 == 0 && t > 0.12 && t < 0.88 {
                let drop = 3;
                for sign in [-1.0f32, 1.0] {
                    let rx = (cx + perp_x * 1.3 * sign).round() as i32;
                    let rz = (cz + perp_z * 1.3 * sign).round() as i32;
                    span.world.fill_box((rx, deck_y - drop, rz), (rx, deck_y - 1, rz), style.support_id);
                }
                for w in -1..=1 {
                    let wx = (cx + perp_x * w as f32).round() as i32;
                    let wz = (cz + perp_z * w as f32).round() as i32;
                    span.world.set(wx, deck_y - drop, wz, style.support_id);
                }
            }
            // Estalactita en el punto mas alto del arco.
            if i == steps / 2 {
                let sx = cx.round() as i32;
                let sz = cz.round() as i32;
                place_pillar(&mut span.world, sx, sz, deck_y - 6, deck_y - 1, style.support_id);
                span.world.set(sx, deck_y - 7, sz, style.support_id);
            }
        } else {
            // Vigas cruzadas cada 4 pasos, debajo del tablero.
            if i % 4 == 0 {
                for w in -1..=1 {
                    let wx = (cx + perp_x * w as f32).round() as i32;
                    let wz = (cz + perp_z * w as f32).round() as i32;
                    span.world.set(wx, deck_y - 1, wz, style.support_id);
                }
            }
            // Cuerdas: cuelgan de un cable principal recto entre las
            // puntas de los dos pilares hasta el pasamanos del tablero.
            if i % 2 == 0 {
                let pillar_top_a = local_a.y.round() + 8.0;
                let pillar_top_b = local_b.y.round() + 8.0;
                let cable_y = (pillar_top_a + (pillar_top_b - pillar_top_a) * t) as i32;
                let rail_y = deck_y + 1;
                if cable_y > rail_y {
                    for sign in [-1.0f32, 1.0] {
                        let rx = (cx + perp_x * 1.5 * sign).round() as i32;
                        let rz = (cz + perp_z * 1.5 * sign).round() as i32;
                        span.world.fill_box((rx, rail_y, rz), (rx, cable_y, rz), style.support_id);
                    }
                }
            }
        }
    }

    islands.push(span);
}

/// Arma la ruta completa entre dos puntos de mundo: si la distancia supera
/// `BRIDGE_MAX_SPAN`, mete 1 o 2 mini-islas de descanso (`build_rest_island`)
/// repartidas en el medio y encadena un tramo (`build_bridge_span`) entre
/// cada punto consecutivo, en vez de un solo tramo gigante.
fn build_bridge_route(islands: &mut Vec<Island>, a: Vec3, b: Vec3, style: &BridgeStyle) {
    let dist = ((b.x - a.x).powi(2) + (b.z - a.z).powi(2)).sqrt();
    if dist <= BRIDGE_MAX_SPAN {
        build_bridge_span(islands, a, b, style);
        return;
    }
    let rests = if dist > BRIDGE_MAX_SPAN * 2.0 { 2 } else { 1 };
    let mut points = vec![a];
    for k in 1..=rests {
        let t = k as f32 / (rests as f32 + 1.0);
        let mid = Vec3::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t, a.z + (b.z - a.z) * t);
        points.push(build_rest_island(islands, mid, 5));
    }
    points.push(b);
    for pair in points.windows(2) {
        build_bridge_span(islands, pair[0], pair[1], style);
    }
}

/// Construye un puente entre dos islas: encuentra el borde real de cada una
/// en direccion a la otra (`find_edge`, funciona con cualquier semilla),
/// nivela y refuerza un estribo en cada punta (parche aplanado con el
/// material del tablero), y arma la ruta (`build_bridge_route`, con
/// mini-islas de descanso si el hueco es muy grande) en el estilo pedido.
#[allow(clippy::too_many_arguments)]
fn build_bridge(islands: &mut Vec<Island>, idx_a: usize, hm_a: &Heightmap, pa: &IslandParams, idx_b: usize, hm_b: &Heightmap, pb: &IslandParams, min_world_y: i32, style: &BridgeStyle) {
    let a_center_world = islands[idx_a].to_world_point(Vec3::new(pa.center_x as f32, pa.top_y as f32, pa.center_z as f32));
    let b_center_world = islands[idx_b].to_world_point(Vec3::new(pb.center_x as f32, pb.top_y as f32, pb.center_z as f32));
    let angle_deg = (b_center_world.z - a_center_world.z).atan2(b_center_world.x - a_center_world.x).to_degrees();

    let edge_a = find_edge(hm_a, pa.center_x, pa.center_z, angle_deg.to_radians(), pa.radius + 200.0);
    let edge_b = find_edge(hm_b, pb.center_x, pb.center_z, (angle_deg + 180.0).to_radians(), pb.radius + 200.0);

    let edge_a_world = islands[idx_a].to_world_point(Vec3::new(edge_a.0 as f32, edge_a.2 as f32, edge_a.1 as f32));
    let edge_b_world = islands[idx_b].to_world_point(Vec3::new(edge_b.0 as f32, edge_b.2 as f32, edge_b.1 as f32));
    let bridge_y_world = (((edge_a_world.y + edge_b_world.y) * 0.5).round() as i32).max(min_world_y);

    // Estribos: aplana y refuerza un parche de 5x5 en cada isla (en SUS
    // coordenadas locales) para que la transicion al puente no tenga
    // escalon y se lea como un anclaje real, no un tablero que arranca de
    // la nada.
    let local_y_a = bridge_y_world - islands[idx_a].offset.1;
    flatten_area(&mut islands[idx_a].world, edge_a.0 - 2, edge_a.1 - 2, edge_a.0 + 2, edge_a.1 + 2, local_y_a, style.deck_id);
    let local_y_b = bridge_y_world - islands[idx_b].offset.1;
    flatten_area(&mut islands[idx_b].world, edge_b.0 - 2, edge_b.1 - 2, edge_b.0 + 2, edge_b.1 + 2, local_y_b, style.deck_id);

    let a_anchor = Vec3::new(edge_a_world.x, bridge_y_world as f32, edge_a_world.z);
    let b_anchor = Vec3::new(edge_b_world.x, bridge_y_world as f32, edge_b_world.z);
    build_bridge_route(islands, a_anchor, b_anchor, style);
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

/// Todo lo que la parte 5 necesita para la camara: la lista de islas, y el
/// punto de mundo (mas el radio, para la principal) donde centrar la
/// orbita para cada una de las tres islas "grandes" -- usado tanto para la
/// vista general (centro/distancia de la principal) como para las teclas
/// 4/5/6 que recentran la camara en la principal/Nether/End.
pub struct SceneIslands {
    pub islands: Vec<Island>,
    pub main_center: Vec3,
    pub main_radius: f32,
    pub nether_center: Vec3,
    pub end_center: Vec3,
    /// Isla satelite del monolito de iron_block (para encuadrarlo de cerca
    /// en `--record`, ver `record.rs`).
    pub monolith_center: Vec3,
    /// Centro del pueblo en la isla principal (para encuadrarlo de cerca en
    /// `--record`, igual que el monolito).
    pub village_center: Vec3,
}

/// Construye la isla principal (el faro, el lago, la casita) y sus dos
/// satelites (monolito y jardin), la isla del Nether y la isla del End, con
/// sus puentes/caminos. Devuelve la lista de islas (cada una su propia
/// mini-grilla) y los puntos de mundo donde deberia mirar la camara para
/// cada una de las tres islas grandes.
pub fn build_lighthouse_scene(seed: u32) -> SceneIslands {
    let mut islands: Vec<Island> = Vec::new();

    // Antes 24 arboles: con el pueblo nuevo ocupando buena parte del interior
    // (`clear_trees_near` ya saca los que caen encima), bajar la cantidad de
    // base deja menos reintentos desperdiciados y una isla menos apretada.
    let main = spawn_island(&mut islands, seed, IslandParams::new(42.0, 16), (0, 0, 0));
    let (cx, cz, r) = (main.params.center_x, main.params.center_z, main.params.radius);
    let water_level_world = main.params.water_level; // offset.y == 0 para la isla principal

    let (lx, lz) = polar(cx, cz, 250.0, r * 0.5);
    clear_trees_near(&mut islands[main.index].world, lx, lz, 6);
    build_lighthouse(&mut islands[main.index].world, &main.heightmap, lx, lz);

    let (kx, kz) = polar(cx, cz, 40.0, r * 0.32);
    let lake_r = (r * 0.30) as i32;
    // Sin esto, los arboles plantados por generate_island (anteriores a
    // cualquier estructura) podian quedar justo encima/al borde del lago,
    // con la copa colgando sobre el agua: la sombreaba casi por completo y
    // encima la tapaba visualmente. Limpiar antes de excavarlo, igual que
    // ya se hacia para el faro y la casita.
    clear_trees_near(&mut islands[main.index].world, kx, kz, lake_r + 4);
    build_lake_and_waterfall(&mut islands[main.index].world, &main.heightmap, kx, kz, lake_r, main.params.water_level, 40.0);
    build_dock(&mut islands[main.index].world, &main.heightmap, kx, kz, lake_r, 220.0, main.params.water_level);

    let (hx, hz) = polar(cx, cz, 150.0, r * 0.45);
    clear_trees_near(&mut islands[main.index].world, hx, hz, 7);
    build_house(&mut islands[main.index].world, &main.heightmap, hx, hz);

    let path_points = [lerp_point((lx, lz), (kx, kz), 0.33), lerp_point((lx, lz), (kx, kz), 0.66), lerp_point((kx, kz), (hx, hz), 0.33), lerp_point((kx, kz), (hx, hz), 0.66)];
    build_path_lights(&mut islands[main.index].world, &main.heightmap, &path_points);

    // Pueblo: el interior de la isla ya tiene 3 cosas grandes (faro a 250
    // grados, lago a 40 -- su orilla en rampa ocupa un disco enorme, radio +
    // 9 de sombra -- y casita a 150), asi que el hueco angular mas ancho
    // entre ellas cae cerca de 200 grados. Layout hexagonal alrededor de la
    // torre (centro): 3 casas y 2 parcelas intercaladas cada 60 grados, el
    // pozo mas cerca del centro en el sexto hueco.
    let village_center = polar(cx, cz, 200.0, r * 0.48);
    let village_top_y = main.heightmap.top_at(village_center.0, village_center.1).unwrap_or(main.params.top_y);
    let village_center_world = islands[main.index].to_world_point(Vec3::new(village_center.0 as f32, village_top_y as f32, village_center.1 as f32));
    clear_trees_near(&mut islands[main.index].world, village_center.0, village_center.1, 13);

    // Offsets en cartesianas (no polares): con estructuras cuadradas, dos
    // puntos a la misma distancia angular pueden terminar con sus AABB
    // pisandose por poco (la diagonal de un cuadrado mide mas que su lado).
    // Elegidos a mano para que ningun par de rectangulos (torre/casas/pozo/
    // parcelas, cada uno con su alcance real incluido) se toque.
    let house0 = (village_center.0 + 8, village_center.1);
    let house1 = (village_center.0, village_center.1 + 8);
    let house2 = (village_center.0 - 9, village_center.1 - 2);
    build_village_house(&mut islands[main.index].world, &main.heightmap, house0.0, house0.1, 0);
    build_village_house(&mut islands[main.index].world, &main.heightmap, house1.0, house1.1, 1);
    build_village_house(&mut islands[main.index].world, &main.heightmap, house2.0, house2.1, 2);

    build_village_tower(&mut islands[main.index].world, &main.heightmap, village_center.0, village_center.1);

    let well_pos = (village_center.0, village_center.1 - 7);
    build_well(&mut islands[main.index].world, &main.heightmap, well_pos.0, well_pos.1);

    let farm0 = (village_center.0 + 9, village_center.1 + 8);
    let farm1 = (village_center.0 - 9, village_center.1 - 9);
    build_farm_plot(&mut islands[main.index].world, &main.heightmap, farm0.0, farm0.1, 2, 2);
    build_farm_plot(&mut islands[main.index].world, &main.heightmap, farm1.0, farm1.1, 2, 2);

    // Faroles en postes a mitad de camino de cada radio del pueblo -- antes
    // de pintar los caminos, para que `build_path_lights` todavia vea pasto
    // debajo (si se llamara despues, el propio camino ya habria pisado esa
    // columna y la condicion de pasto/arena no pasaria).
    let village_lights = [
        lerp_point(village_center, house0, 0.55),
        lerp_point(village_center, house1, 0.55),
        lerp_point(village_center, house2, 0.55),
        lerp_point(village_center, farm0, 0.5),
        lerp_point(village_center, farm1, 0.5),
    ];
    build_path_lights(&mut islands[main.index].world, &main.heightmap, &village_lights);

    let village_spokes = [house0, house1, house2, well_pos, farm0, farm1];
    for &(ex, ez) in &village_spokes {
        build_ground_path(&mut islands[main.index].world, &main.heightmap, village_center.0, village_center.1, ex, ez, 2);
    }

    // Camino troncal que une muelle - faro - casita - torre del pueblo, y de
    // ahi ramales cortos hacia el extremo real de cada puente (cada uno sale
    // del punto mas cercano, no todos desde la torre, para no cruzar por
    // encima del lago).
    build_ground_path(&mut islands[main.index].world, &main.heightmap, kx, kz, lx, lz, 2);
    build_ground_path(&mut islands[main.index].world, &main.heightmap, lx, lz, hx, hz, 2);
    build_ground_path(&mut islands[main.index].world, &main.heightmap, hx, hz, village_center.0, village_center.1, 2);

    let bridge_spurs = [(250.0f32, lx, lz), (70.0, kx, kz), (160.0, hx, hz), (320.0, lx, lz)];
    for (angle, fx, fz) in bridge_spurs {
        let (ex, ez, _) = find_edge(&main.heightmap, cx, cz, angle.to_radians(), r + 80.0);
        build_ground_path(&mut islands[main.index].world, &main.heightmap, fx, fz, ex, ez, 2);
    }

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
    let monolith_center_world = island_world_center(&islands, &sat_a);
    let wood_bridge = BridgeStyle { deck_id: block::OAK_PLANKS, rail_id: block::OAK_LOG, support_id: block::OAK_LOG, lamp_id: block::GLOWSTONE, arch: false, curve_amount: 3.0 };
    build_bridge(&mut islands, main.index, &main.heightmap, &main.params, sat_a.index, &sat_a.heightmap, &sat_a.params, water_level_world + 2, &wood_bridge);

    // Isla satelite B: jardin con fuente.
    let b_offset_xz = (main_center_world.x + (70f32.to_radians().cos() * (r + sat_gap + sat_r)), main_center_world.z + (70f32.to_radians().sin() * (r + sat_gap + sat_r)));
    let params_b = IslandParams::new(sat_r, 3);
    let offset_b = ((b_offset_xz.0 as i32) - params_b.center_x, 0, (b_offset_xz.1 as i32) - params_b.center_z);
    let sat_b = spawn_island(&mut islands, seed.wrapping_add(202), params_b, offset_b);
    clear_trees_near(&mut islands[sat_b.index].world, sat_b.params.center_x, sat_b.params.center_z, 7);
    build_garden(&mut islands[sat_b.index].world, &sat_b.heightmap, sat_b.params.center_x, sat_b.params.center_z, seed);
    build_bridge(&mut islands, main.index, &main.heightmap, &main.params, sat_b.index, &sat_b.heightmap, &sat_b.params, water_level_world + 2, &wood_bridge);

    // Isla del Nether: a un costado y mas abajo que la principal (offset.y
    // negativo), perfil de ruido "ridged" (generate_rugged_island) en vez
    // del fBm suave de las demas. Union por un puente de nether_bricks.
    let nether_r = 15.0f32;
    let nether_gap = 8.0;
    let nether_angle = 320.0f32;
    let nether_xz = (
        main_center_world.x + nether_angle.to_radians().cos() * (r + nether_gap + nether_r),
        main_center_world.z + nether_angle.to_radians().sin() * (r + nether_gap + nether_r),
    );
    let params_n = IslandParams::new(nether_r, 0);
    let offset_n = ((nether_xz.0 as i32) - params_n.center_x, -13, (nether_xz.1 as i32) - params_n.center_z);
    let (nether_world, nether_hm) = generate_rugged_island(seed.wrapping_add(303), &params_n);
    let nether_idx = islands.len();
    islands.push(Island::new(nether_world, offset_n));
    let nether = BuiltIsland { index: nether_idx, heightmap: nether_hm, params: params_n };

    let (ncx, ncz, nr) = (nether.params.center_x, nether.params.center_z, nether.params.radius);
    let nseed = seed.wrapping_add(303);
    let lava_level = nether.params.top_y - 5;

    let (plx, plz) = polar(ncx, ncz, 195.0, nr * 0.42);
    build_nether_portal(&mut islands[nether.index].world, &nether.heightmap, plx, plz);

    let (llx, llz) = polar(ncx, ncz, 330.0, nr * 0.3);
    build_lava_lake_and_falls(&mut islands[nether.index].world, &nether.heightmap, llx, llz, (nr * 0.28) as i32, lava_level, 330.0);

    // Arbol hongo carmesi grande, con nylium carmesi debajo.
    let (ctx, ctz) = polar(ncx, ncz, 20.0, nr * 0.4);
    build_nylium_patch(&mut islands[nether.index].world, &nether.heightmap, ctx, ctz, 3, block::CRIMSON_NYLIUM);
    build_fungus_tree(&mut islands[nether.index].world, &nether.heightmap, ctx, ctz, block::CRIMSON_STEM, block::NETHER_WART_BLOCK, block::SHROOMLIGHT, 9, nseed);

    // Arbol hongo distorsionado grande, con nylium turquesa debajo.
    let (wtx, wtz) = polar(ncx, ncz, 160.0, nr * 0.4);
    build_nylium_patch(&mut islands[nether.index].world, &nether.heightmap, wtx, wtz, 3, block::WARPED_NYLIUM);
    build_fungus_tree(&mut islands[nether.index].world, &nether.heightmap, wtx, wtz, block::WARPED_STEM, block::WARPED_WART_BLOCK, block::SHROOMLIGHT, 8, nseed.wrapping_add(7));

    // Formacion de blackstone/basalto con fuente de lava y charco al pie.
    let (bfx, bfz) = polar(ncx, ncz, 255.0, nr * 0.4);
    build_blackstone_formation(&mut islands[nether.index].world, &nether.heightmap, bfx, bfz, nseed);

    // Fuegos naranjas dispersos y un parche de soul_sand con fuego de alma.
    scatter_fire(&mut islands[nether.index].world, &nether.heightmap, ncx, ncz, nr * 0.75, 7, nseed);
    let (sfx, sfz) = polar(ncx, ncz, 90.0, nr * 0.5);
    build_soul_fire_patch(&mut islands[nether.index].world, &nether.heightmap, sfx, sfz, 2, nseed);

    // Camino en zig-zag de blackstone colgando por debajo del borde.
    build_hanging_zigzag_path(&mut islands[nether.index].world, &nether.heightmap, ncx, ncz, 230.0, 4);

    hang_glowstone_edge(&mut islands[nether.index].world, &nether.heightmap, ncx, ncz, nr, 5, nseed);

    let nether_bridge = BridgeStyle { deck_id: block::NETHER_BRICKS, rail_id: block::OBSIDIAN, support_id: block::NETHER_BRICKS, lamp_id: block::GLOWSTONE, arch: true, curve_amount: 2.5 };
    build_bridge(&mut islands, main.index, &main.heightmap, &main.params, nether.index, &nether.heightmap, &nether.params, offset_n.1 + 4, &nether_bridge);
    let nether_center_world = island_world_center(&islands, &nether);

    // Isla del End: al otro costado, mas alta y mas lejos que la principal
    // (offset.y positivo, mayor separacion), perfil de ruido suave y
    // redondeado (generate_soft_island). Union por un camino de bloques de
    // end_stone flotantes (no un puente solido).
    let end_r = 22.0f32;
    let end_gap = 10.0;
    let end_angle = 160.0f32;
    let end_xz = (
        main_center_world.x + end_angle.to_radians().cos() * (r + end_gap + end_r),
        main_center_world.z + end_angle.to_radians().sin() * (r + end_gap + end_r),
    );
    let params_e = IslandParams::new(end_r, 0);
    // Antes flotaba a +32 (32 bloques arriba del top_y de la isla principal);
    // ahora solo un poco por encima, para que el recorrido de la camara y el
    // puente se lean bien en vez de subir en vertical casi todo el tramo.
    let offset_e = ((end_xz.0 as i32) - params_e.center_x, 9, (end_xz.1 as i32) - params_e.center_z);
    let (end_world, end_hm) = generate_soft_island(seed.wrapping_add(404), &params_e);
    let end_idx = islands.len();
    islands.push(Island::new(end_world, offset_e));
    let end_island = BuiltIsland { index: end_idx, heightmap: end_hm, params: params_e };

    let (ecx, ecz, er) = (end_island.params.center_x, end_island.params.center_z, end_island.params.radius);
    let eseed = seed.wrapping_add(404);

    // Pilares de obsidiana + end_crystal, ahora en el borde de la isla para
    // no taparle el frente a la ciudad.
    build_end_pillars(&mut islands[end_island.index].world, &end_island.heightmap, ecx, ecz, er, eseed);

    // Ciudad de torres: torre central delgada, dos torres secundarias con
    // pisos apilados unidas por escaleras diagonales.
    // Torres separadas 180 grados (antes 150) y un poco mas lejos del centro:
    // con la isla mas grande hay lugar de sobra y asi no se pisan con los
    // bosques de chorus que van mas al borde.
    let (t1x, t1z) = polar(ecx, ecz, 30.0, er * 0.38);
    let (t2x, t2z) = polar(ecx, ecz, 210.0, er * 0.38);
    let central_base_y = build_central_tower(&mut islands[end_island.index].world, &end_island.heightmap, ecx, ecz, 18);
    let tower1_base_y = build_city_tower(&mut islands[end_island.index].world, &end_island.heightmap, t1x, t1z, 2, false);
    let tower2_base_y = build_city_tower(&mut islands[end_island.index].world, &end_island.heightmap, t2x, t2z, 3, true);
    build_diagonal_stair(&mut islands[end_island.index].world, ecx, ecz, central_base_y + 2, t1x, t1z, tower1_base_y + 1);
    build_diagonal_stair(&mut islands[end_island.index].world, ecx, ecz, central_base_y + 2, t2x, t2z, tower2_base_y + 1);

    // Barco chico de purpur flotando cerca de la ciudad, lejos de las torres
    // y de los bosques de chorus.
    let (shx, shz) = polar(ecx, ecz, 120.0, er * 0.6);
    build_purpur_ship(&mut islands[end_island.index].world, &end_island.heightmap, shx, shz);

    // Bosquecitos de chorus alrededor de la ciudad, mas hacia el borde y
    // repartidos lejos de las torres/barco.
    for angle in [280.0, 330.0, 60.0] {
        let (chx, chz) = polar(ecx, ecz, angle, er * 0.72);
        build_chorus_grove(&mut islands[end_island.index].world, &end_island.heightmap, chx, chz, 6, eseed ^ (angle as u32));
    }

    let end_bridge = BridgeStyle { deck_id: block::END_STONE_BRICKS, rail_id: block::PURPUR, support_id: block::PURPUR, lamp_id: block::END_ROD, arch: true, curve_amount: 2.5 };
    build_bridge(&mut islands, main.index, &main.heightmap, &main.params, end_island.index, &end_island.heightmap, &end_island.params, offset_e.1 - 2, &end_bridge);
    let end_center_world = island_world_center(&islands, &end_island);

    // 2-3 mini-islas de end_stone flotando alrededor de la isla principal del End.
    build_mini_end_islands(&mut islands, end_center_world, er, eseed);

    SceneIslands { islands, main_center: main_center_world, main_radius: r, nether_center: nether_center_world, end_center: end_center_world, monolith_center: monolith_center_world, village_center: village_center_world }
}
