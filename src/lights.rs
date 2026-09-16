use crate::islands::Island;
use crate::material::MaterialTable;
use crate::math::Vec3;

/// Upper bound on how many lights `query_nearby` can be asked for at once
/// (backs its allocation-free scratch arrays).
const MAX_QUERY_LIGHTS: usize = 8;

pub struct PointLight {
    pub pos: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub radius: f32,
}

/// Uniform spatial grid over the world bounds so shading only has to check
/// the lights near a given point instead of every emissive block in the
/// scene. Global in world-space coordinates: it doesn't matter which island
/// a light physically belongs to, a light on one island can still light (and
/// cast shadows across) another island or a bridge.
pub struct LightGrid {
    pub lights: Vec<PointLight>,
    origin: Vec3,
    cell_size: f32,
    nx: i32,
    ny: i32,
    nz: i32,
    cells: Vec<Vec<u16>>,
}

impl LightGrid {
    fn cell_index(&self, cx: i32, cy: i32, cz: i32) -> Option<usize> {
        if cx < 0 || cy < 0 || cz < 0 || cx >= self.nx || cy >= self.ny || cz >= self.nz {
            return None;
        }
        Some(((cx * self.ny + cy) * self.nz + cz) as usize)
    }

    fn cell_of(&self, p: Vec3) -> (i32, i32, i32) {
        let rel = p - self.origin;
        ((rel.x / self.cell_size).floor() as i32, (rel.y / self.cell_size).floor() as i32, (rel.z / self.cell_size).floor() as i32)
    }

    /// Fills `out` with up to `out.len()` lights that can plausibly reach
    /// `point`, nearest first, and returns how many were written. Runs a
    /// bounded insertion sort over the cell's lights instead of allocating a
    /// scratch vector, so it costs nothing on the heap per ray.
    pub fn query_nearby(&self, point: Vec3, out: &mut [u16]) -> usize {
        let max_n = out.len();
        let mut dist2 = [f32::INFINITY; MAX_QUERY_LIGHTS];
        debug_assert!(max_n <= MAX_QUERY_LIGHTS);

        let (cx, cy, cz) = self.cell_of(point);
        let Some(idx) = self.cell_index(cx, cy, cz) else { return 0 };

        let mut count = 0usize;
        for &light_idx in &self.cells[idx] {
            let d = (self.lights[light_idx as usize].pos - point).length_squared();
            let insert_at = if count < max_n {
                let pos = count;
                count += 1;
                Some(pos)
            } else if d < dist2[max_n - 1] {
                Some(max_n - 1)
            } else {
                None
            };
            if let Some(mut pos) = insert_at {
                while pos > 0 && dist2[pos - 1] > d {
                    dist2[pos] = dist2[pos - 1];
                    out[pos] = out[pos - 1];
                    pos -= 1;
                }
                dist2[pos] = d;
                out[pos] = light_idx;
            }
        }
        count
    }
}

/// Scans every island for emissive blocks and registers each one as a point
/// light in world-space, so a lamp on one island can illuminate (and cast
/// shadows across) a neighbouring island or the bridge between them.
pub fn build_light_grid(islands: &[Island], materials: &MaterialTable, cell_size: f32) -> LightGrid {
    let mut lights = Vec::new();
    let mut global_min = Vec3::splat(f32::INFINITY);
    let mut global_max = Vec3::splat(f32::NEG_INFINITY);

    for island in islands {
        let (lo, hi) = island.world_aabb();
        global_min = Vec3::new(global_min.x.min(lo.x), global_min.y.min(lo.y), global_min.z.min(lo.z));
        global_max = Vec3::new(global_max.x.max(hi.x), global_max.y.max(hi.y), global_max.z.max(hi.z));

        let w = &island.world;
        for y in 0..w.ny {
            for z in 0..w.nz {
                for x in 0..w.nx {
                    let id = w.get(x, y, z);
                    if id == 0 {
                        continue;
                    }
                    let Some(mat) = materials.get(id) else { continue };
                    let intensity = mat.emission.max_component();
                    if intensity <= 0.001 {
                        continue;
                    }
                    let radius = 4.0 + intensity * 6.0;
                    let local = Vec3::new(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5);
                    lights.push(PointLight {
                        pos: island.to_world_point(local),
                        color: (mat.emission / intensity).clamp01(),
                        intensity,
                        radius,
                    });
                }
            }
        }
    }

    let origin = global_min;
    let span = global_max - global_min;
    let nx = ((span.x / cell_size).ceil() as i32).max(1);
    let ny = ((span.y / cell_size).ceil() as i32).max(1);
    let nz = ((span.z / cell_size).ceil() as i32).max(1);
    let mut cells: Vec<Vec<u16>> = (0..(nx * ny * nz)).map(|_| Vec::new()).collect();

    let mut grid = LightGrid { lights, origin, cell_size, nx, ny, nz, cells: Vec::new() };

    for (i, light) in grid.lights.iter().enumerate() {
        let lo = light.pos - Vec3::splat(light.radius) - origin;
        let hi = light.pos + Vec3::splat(light.radius) - origin;
        let cx0 = (lo.x / cell_size).floor() as i32;
        let cy0 = (lo.y / cell_size).floor() as i32;
        let cz0 = (lo.z / cell_size).floor() as i32;
        let cx1 = (hi.x / cell_size).floor() as i32;
        let cy1 = (hi.y / cell_size).floor() as i32;
        let cz1 = (hi.z / cell_size).floor() as i32;
        for cx in cx0.max(0)..=cx1.min(nx - 1) {
            for cy in cy0.max(0)..=cy1.min(ny - 1) {
                for cz in cz0.max(0)..=cz1.min(nz - 1) {
                    let idx = ((cx * ny + cy) * nz + cz) as usize;
                    cells[idx].push(i as u16);
                }
            }
        }
    }

    grid.cells = cells;
    grid
}
