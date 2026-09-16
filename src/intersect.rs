// point/voxel/t de HitInfo se usan a partir de fase 4+ (rayos de sombra, reflexion,
// refraccion, luces puntuales); shade_hit de fase 2 solo necesita normal/face/uv/block.
#![allow(dead_code)]

use crate::math::{Ray, Vec3};
use crate::world::World;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    PX,
    NX,
    PY,
    NY,
    PZ,
    NZ,
}

impl Face {
    #[inline]
    pub fn normal(self) -> Vec3 {
        match self {
            Face::PX => Vec3::new(1.0, 0.0, 0.0),
            Face::NX => Vec3::new(-1.0, 0.0, 0.0),
            Face::PY => Vec3::new(0.0, 1.0, 0.0),
            Face::NY => Vec3::new(0.0, -1.0, 0.0),
            Face::PZ => Vec3::new(0.0, 0.0, 1.0),
            Face::NZ => Vec3::new(0.0, 0.0, -1.0),
        }
    }

    #[inline]
    fn from_axis_sign(axis: usize, positive_normal: bool) -> Face {
        match (axis, positive_normal) {
            (0, true) => Face::PX,
            (0, false) => Face::NX,
            (1, true) => Face::PY,
            (1, false) => Face::NY,
            (2, true) => Face::PZ,
            _ => Face::NZ,
        }
    }
}

pub struct HitInfo {
    pub t: f32,
    pub point: Vec3,
    pub normal: Vec3,
    pub face: Face,
    pub uv: (f32, f32),
    pub block: u8,
    pub voxel: (i32, i32, i32),
}

/// Ray vs world AABB, slab method. Returns (t_enter, t_exit) clamped so t_enter >= 0.
fn intersect_aabb(ray: Ray, bmin: Vec3, bmax: Vec3) -> Option<(f32, f32)> {
    let o = [ray.origin.x, ray.origin.y, ray.origin.z];
    let d = [ray.dir.x, ray.dir.y, ray.dir.z];
    let lo = [bmin.x, bmin.y, bmin.z];
    let hi = [bmax.x, bmax.y, bmax.z];

    let mut tmin = 0.0f32;
    let mut tmax = f32::INFINITY;
    for axis in 0..3 {
        let inv_d = 1.0 / d[axis];
        let mut t0 = (lo[axis] - o[axis]) * inv_d;
        let mut t1 = (hi[axis] - o[axis]) * inv_d;
        if inv_d < 0.0 {
            std::mem::swap(&mut t0, &mut t1);
        }
        tmin = tmin.max(t0);
        tmax = tmax.min(t1);
        if tmin > tmax {
            return None;
        }
    }
    Some((tmin, tmax))
}

#[inline]
fn face_uv(face: Face, local: Vec3) -> (f32, f32) {
    match face {
        Face::PX => (local.z, 1.0 - local.y),
        Face::NX => (1.0 - local.z, 1.0 - local.y),
        Face::PZ => (1.0 - local.x, 1.0 - local.y),
        Face::NZ => (local.x, 1.0 - local.y),
        Face::PY => (local.x, local.z),
        Face::NY => (local.x, 1.0 - local.z),
    }
}

#[inline]
fn frac0(v: f32) -> f32 {
    let f = v - v.floor();
    if f < 0.0 {
        0.0
    } else if f >= 1.0 {
        0.999_999_9
    } else {
        f
    }
}

/// Casts `ray` through `world` using 3D DDA (Amanatides & Woo). `accept` decides
/// whether a voxel's block id should stop the ray (used later for transparent
/// materials, leaves cutout, shadow attenuation, etc). `max_t` limits the ray
/// length (use f32::INFINITY for primary rays).
pub fn traverse(world: &World, ray: Ray, max_t: f32, accept: impl Fn(u8, Face, (f32, f32)) -> bool) -> Option<HitInfo> {
    let (bmin, bmax) = world.aabb();
    let (t_enter, t_exit) = intersect_aabb(ray, bmin, bmax)?;
    if t_exit < 0.0 || t_enter > max_t {
        return None;
    }
    let t_enter = t_enter.max(0.0);

    let start = ray.at(t_enter + 1e-4);
    let mut vx = (start.x.floor() as i32).clamp(0, world.nx - 1);
    let mut vy = (start.y.floor() as i32).clamp(0, world.ny - 1);
    let mut vz = (start.z.floor() as i32).clamp(0, world.nz - 1);

    let d = [ray.dir.x, ray.dir.y, ray.dir.z];
    let mut voxel = [vx, vy, vz];

    let mut step = [0i32; 3];
    let mut t_max = [f32::INFINITY; 3];
    let mut t_delta = [f32::INFINITY; 3];
    let origin = [ray.origin.x, ray.origin.y, ray.origin.z];

    for axis in 0..3 {
        if d[axis] > 1e-12 {
            step[axis] = 1;
            let boundary = (voxel[axis] + 1) as f32;
            t_max[axis] = (boundary - origin[axis]) / d[axis];
            t_delta[axis] = 1.0 / d[axis];
        } else if d[axis] < -1e-12 {
            step[axis] = -1;
            let boundary = voxel[axis] as f32;
            t_max[axis] = (boundary - origin[axis]) / d[axis];
            t_delta[axis] = -1.0 / d[axis];
        }
    }

    // The entry face into the grid, used only if the very first voxel is solid.
    let mut entry_face = {
        // Determine which AABB face t_enter belongs to by re-checking each axis.
        let mut face = Face::PY;
        let mut best = f32::INFINITY;
        for axis in 0..3 {
            let inv_d = 1.0 / d[axis];
            let lo = [bmin.x, bmin.y, bmin.z][axis];
            let hi = [bmax.x, bmax.y, bmax.z][axis];
            let o = origin[axis];
            let t_lo = (lo - o) * inv_d;
            let t_hi = (hi - o) * inv_d;
            let (t_near, positive) = if inv_d < 0.0 { (t_hi, true) } else { (t_lo, false) };
            if (t_near - t_enter).abs() < best {
                best = (t_near - t_enter).abs();
                face = Face::from_axis_sign(axis, positive);
            }
        }
        face
    };

    let mut entry_t = t_enter;
    loop {
        vx = voxel[0];
        vy = voxel[1];
        vz = voxel[2];
        let block = world.get(vx, vy, vz);
        if block != 0 {
            let point = ray.at(entry_t.max(0.0));
            let local = Vec3::new(frac0(point.x), frac0(point.y), frac0(point.z));
            let uv = face_uv(entry_face, local);
            if accept(block, entry_face, uv) {
                return Some(HitInfo {
                    t: entry_t,
                    point,
                    normal: entry_face.normal(),
                    face: entry_face,
                    uv,
                    block,
                    voxel: (vx, vy, vz),
                });
            }
        }

        // Step to the next voxel: advance along the axis with the smallest t_max.
        let axis = if t_max[0] < t_max[1] {
            if t_max[0] < t_max[2] {
                0
            } else {
                2
            }
        } else if t_max[1] < t_max[2] {
            1
        } else {
            2
        };

        if t_max[axis] > max_t {
            return None;
        }

        entry_t = t_max[axis];
        voxel[axis] += step[axis];
        entry_face = Face::from_axis_sign(axis, step[axis] < 0);
        t_max[axis] += t_delta[axis];

        if voxel[0] < 0 || voxel[0] >= world.nx || voxel[1] < 0 || voxel[1] >= world.ny || voxel[2] < 0 || voxel[2] >= world.nz {
            return None;
        }
    }
}
