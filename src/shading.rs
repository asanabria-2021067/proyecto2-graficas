use crate::intersect::{traverse, Face, HitInfo};
use crate::lights::LightGrid;
use crate::material::{FaceTex, Material, MaterialTable};
use crate::math::{Ray, Vec3};
use crate::world::World;

pub const MAX_LIGHTS_PER_POINT: usize = 4;
const SHADOW_MAX_DIST: f32 = 256.0;
const SHADOW_EPS: f32 = 1e-3;
const MAX_TRANSPARENT_STEPS: u32 = 8;

pub struct Sun {
    pub dir: Vec3, // direction the light travels (from sun towards the scene)
    pub color: Vec3,
    pub intensity: f32,
}

pub struct Environment {
    pub sun: Sun,
    pub ambient: Vec3,
}

pub fn day_environment() -> Environment {
    Environment {
        sun: Sun { dir: Vec3::new(-0.45, -0.82, -0.35).normalize(), color: Vec3::new(1.0, 0.96, 0.88), intensity: 1.5 },
        ambient: Vec3::new(0.22, 0.26, 0.32),
    }
}

pub fn night_environment() -> Environment {
    Environment {
        sun: Sun { dir: Vec3::new(0.3, -0.9, 0.25).normalize(), color: Vec3::new(0.55, 0.65, 0.95), intensity: 0.1 },
        ambient: Vec3::new(0.02, 0.022, 0.045),
    }
}

#[inline]
pub fn face_tex(mat: &Material, face: Face) -> &FaceTex {
    match face {
        Face::PY => &mat.top,
        Face::NY => &mat.bottom,
        _ => &mat.side,
    }
}

/// Shared DDA-traversal predicate: stops on any solid block, except that
/// alpha-cutout materials (leaves, lamp frames) let the ray pass through
/// their transparent texels. Used for primary rays and shadow rays alike.
#[inline]
pub fn is_visible(materials: &MaterialTable, id: u8, face: Face, uv: (f32, f32)) -> bool {
    match materials.get(id) {
        Some(mat) if mat.alpha_cutout => {
            let (_, a) = face_tex(mat, face).albedo.sample(uv.0, uv.1);
            a > 0.5
        }
        Some(_) => true,
        None => true,
    }
}

/// Casts a shadow ray with the same DDA used for primary rays, but instead of
/// stopping at the first hit it walks through transparent surfaces (water,
/// glass) attenuating the light by their transparency and color; leaves use
/// their alpha cutout directly via the traverse predicate. Fully opaque
/// surfaces block the light outright.
fn shadow_transmittance(world: &World, materials: &MaterialTable, origin: Vec3, dir: Vec3, max_dist: f32) -> Vec3 {
    let mut transmittance = Vec3::splat(1.0);
    let mut traveled = 0.0f32;
    let mut current = origin;

    for _ in 0..MAX_TRANSPARENT_STEPS {
        let remaining = max_dist - traveled;
        if remaining <= SHADOW_EPS {
            break;
        }
        let ray = Ray::new(current, dir);
        let hit = traverse(world, ray, remaining, |id, face, uv| is_visible(materials, id, face, uv));
        let Some(hit) = hit else { break };
        let Some(mat) = materials.get(hit.block) else {
            return Vec3::zero();
        };
        if mat.transparency <= 0.001 {
            return Vec3::zero();
        }
        let (albedo, _a) = face_tex(mat, hit.face).albedo.sample(hit.uv.0, hit.uv.1);
        transmittance = transmittance.mul_v(albedo * mat.transparency);
        traveled = hit.t + SHADOW_EPS;
        current = hit.point + dir * SHADOW_EPS;
    }

    transmittance
}

/// Lambert diffuse + Blinn-Phong specular from the sun and nearby emissive
/// point lights, both shadowed (with transparent attenuation), plus the
/// material's own emission (never shadowed). Returns raw HDR linear color;
/// tone mapping happens once at the top of the recursive trace.
#[allow(clippy::too_many_arguments)]
pub fn shade_surface(hit: &HitInfo, normal: Vec3, view_dir: Vec3, world: &World, materials: &MaterialTable, lights: &LightGrid, env: &Environment) -> Vec3 {
    let Some(mat) = materials.get(hit.block) else {
        return Vec3::new(1.0, 0.0, 1.0);
    };
    let tex = face_tex(mat, hit.face);
    let (albedo_raw, _alpha) = tex.albedo.sample(hit.uv.0, hit.uv.1);
    let albedo = albedo_raw.mul_v(mat.tint);

    let mut color = albedo.mul_v(env.ambient);

    let l_sun = -env.sun.dir;
    let ndotl = normal.dot(l_sun).max(0.0);
    if ndotl > 0.0 {
        let origin = hit.point + normal * SHADOW_EPS;
        let trans = shadow_transmittance(world, materials, origin, l_sun, SHADOW_MAX_DIST);
        if trans.max_component() > 0.001 {
            let diffuse = albedo * ndotl;
            let half = (l_sun + view_dir).normalize();
            let spec = mat.specular_coef * normal.dot(half).max(0.0).powf(mat.specular_exp);
            color += (diffuse + Vec3::splat(spec)).mul_v(env.sun.color).mul_v(trans) * env.sun.intensity;
        }
    }

    let mut nearby = Vec::with_capacity(MAX_LIGHTS_PER_POINT);
    lights.query_nearby(hit.point, MAX_LIGHTS_PER_POINT, &mut nearby);
    for &idx in &nearby {
        let light = &lights.lights[idx as usize];
        let to_light = light.pos - hit.point;
        let dist = to_light.length();
        if dist < 1e-4 || dist > light.radius {
            continue;
        }
        let l = to_light / dist;
        let ndotl = normal.dot(l).max(0.0);
        if ndotl <= 0.0 {
            continue;
        }
        let falloff = (1.0 - dist / light.radius).max(0.0);
        let atten = falloff * falloff / (1.0 + dist * dist * 0.06);
        if atten <= 0.001 {
            continue;
        }
        let origin = hit.point + normal * SHADOW_EPS;
        let trans = shadow_transmittance(world, materials, origin, l, dist - SHADOW_EPS);
        if trans.max_component() <= 0.001 {
            continue;
        }
        let diffuse = albedo * ndotl;
        let half = (l + view_dir).normalize();
        let spec = mat.specular_coef * normal.dot(half).max(0.0).powf(mat.specular_exp);
        color += (diffuse + Vec3::splat(spec)).mul_v(light.color).mul_v(trans) * (light.intensity * atten);
    }

    color + mat.emission
}

/// Beer-Lambert attenuation for light travelling `distance` through a medium
/// with the given per-channel absorption coefficient.
#[inline]
pub fn beer_lambert(absorption: Vec3, distance: f32) -> Vec3 {
    Vec3::new((-absorption.x * distance).exp(), (-absorption.y * distance).exp(), (-absorption.z * distance).exp())
}

#[inline]
fn aces_channel(x: f32) -> f32 {
    let (a, b, c, d, e) = (2.51, 0.03, 2.43, 0.59, 0.14);
    ((x * (a * x + b)) / (x * (c * x + d) + e)).clamp(0.0, 1.0)
}

/// Approximate ACES filmic tone mapping (Narkowicz fit), applied once to the
/// final linear radiance of a pixel before the framebuffer's sRGB gamma.
#[inline]
pub fn aces_tonemap(c: Vec3) -> Vec3 {
    Vec3::new(aces_channel(c.x), aces_channel(c.y), aces_channel(c.z))
}
