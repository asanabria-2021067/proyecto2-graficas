use crate::intersect::{traverse, HitInfo};
use crate::lights::LightGrid;
use crate::material::{Material, MaterialTable};
use crate::math::{Ray, Vec3};
use crate::render::Tracer;
use crate::shading::{aces_tonemap, beer_lambert, face_tex, is_visible, perturb_normal, shade_surface, Environment};
use crate::world::World;

const EPS: f32 = 1e-3;
const MIN_CONTRIB: f32 = 0.01;

pub fn max_depth_for_quality(quality: u8) -> u32 {
    match quality {
        1 => 2,
        3 => 6,
        _ => 4,
    }
}

/// Ties a voxel world, its materials, lights and environment together into a
/// recursive path tracer: local shading (fase 4) plus reflection/refraction
/// with Fresnel-Schlick mixing and Beer-Lambert absorption inside transparent
/// volumes (fase 5). This is "the scene" in the fase-9 sense, just built
/// incrementally from fase 4/5 test worlds until the lighthouse island lands.
pub struct Scene<'a> {
    pub world: &'a World,
    pub materials: &'a MaterialTable,
    pub lights: &'a LightGrid,
    pub env: Environment,
    pub night: bool,
    pub max_depth: u32,
    pub normalmaps: bool,
}

impl Tracer for Scene<'_> {
    fn trace(&self, ray: Ray) -> Vec3 {
        aces_tonemap(self.trace_recursive(ray, 0, 0))
    }
}

impl Scene<'_> {
    fn sky(&self, dir: Vec3) -> Vec3 {
        crate::sky_color(dir, self.night)
    }

    /// `current_medium` is the block id of the transparent volume the ray
    /// currently originates inside of (0 = air/vacuum). Voxels of that same
    /// id are skipped by the DDA predicate so a multi-voxel body of water or
    /// glass reads as one continuous volume instead of stopping at every
    /// internal face between same-type neighbours.
    fn trace_recursive(&self, ray: Ray, depth: u32, current_medium: u8) -> Vec3 {
        if depth > self.max_depth {
            return Vec3::zero();
        }
        let materials = self.materials;
        let hit = traverse(self.world, ray, f32::INFINITY, |id, face, uv| id != current_medium && (id == 0 || is_visible(materials, id, face, uv)));
        let Some(hit) = hit else {
            return self.sky(ray.dir);
        };

        // Cuando el bloque golpeado es aire (0), en realidad estamos saliendo
        // de `current_medium` hacia afuera: no hay textura propia que sombrear,
        // solo la reflexion/refraccion de la superficie del medio que dejamos.
        let surface_mat = if hit.block == 0 { materials.get(current_medium) } else { materials.get(hit.block) };

        let shading_normal = match surface_mat {
            Some(mat) => perturb_normal(&hit, face_tex(mat, hit.face), mat.normal_strength, self.normalmaps),
            None => hit.normal,
        };

        let local = if hit.block == 0 {
            Vec3::zero()
        } else {
            shade_surface(&hit, shading_normal, -ray.dir, self.world, self.materials, self.lights, &self.env)
        };

        let mut result = match surface_mat {
            Some(mat) => self.specular_bounce(&hit, shading_normal, ray, mat, current_medium, depth, local),
            None => local,
        };

        if current_medium != 0 {
            if let Some(medium_mat) = materials.get(current_medium) {
                result = result.mul_v(beer_lambert(medium_mat.absorption, hit.t.max(0.0)));
            }
        }
        result
    }

    /// Mixes in reflection and (for transparent materials) refraction using
    /// Fresnel-Schlick to split energy between the two, with total internal
    /// reflection redirecting the refracted share back into reflection.
    #[allow(clippy::too_many_arguments)]
    fn specular_bounce(&self, hit: &HitInfo, shading_normal: Vec3, ray: Ray, mat: &Material, current_medium: u8, depth: u32, local: Vec3) -> Vec3 {
        if depth >= self.max_depth {
            return local;
        }
        // La normal perturbada por el normal map gobierna la direccion de los
        // rebotes; la normal geometrica (hit.normal) sigue offseteando el
        // origen del rayo para no perforar el voxel por acne.
        let n = shading_normal;
        let geo_n = hit.normal;

        if mat.transparency > 0.001 {
            let current_ior = if current_medium == 0 { 1.0 } else { self.materials.get(current_medium).map(|m| m.ior).unwrap_or(1.0) };
            let target_ior = if hit.block == 0 { 1.0 } else { mat.ior };
            let cos_i = (-ray.dir).dot(n).clamp(0.0, 1.0);
            let f0r = (current_ior - target_ior) / (current_ior + target_ior);
            let f0 = f0r * f0r;
            let fresnel = (f0 + (1.0 - f0) * (1.0 - cos_i).powi(5)).clamp(0.0, 1.0);

            let eta = current_ior / target_ior;
            let refract_dir = ray.dir.refract(n, eta);

            let mut reflect_amt = (mat.reflectivity + fresnel).clamp(0.0, 1.0);
            let mut refract_amt = ((1.0 - reflect_amt) * mat.transparency).clamp(0.0, 1.0);
            if refract_dir.is_none() {
                // Reflexion interna total: toda la energia que iba a refractar rebota.
                reflect_amt = (reflect_amt + refract_amt).min(1.0);
                refract_amt = 0.0;
            }
            let local_amt = (1.0 - reflect_amt - refract_amt).max(0.0);
            let mut out = local * local_amt;

            if reflect_amt > MIN_CONTRIB {
                let rdir = ray.dir.reflect(n);
                let rorigin = hit.point + geo_n * EPS;
                out += self.trace_recursive(Ray::new(rorigin, rdir), depth + 1, current_medium) * reflect_amt;
            }
            if let Some(rdir) = refract_dir {
                if refract_amt > MIN_CONTRIB {
                    let rorigin = hit.point - geo_n * EPS;
                    let target_medium = if hit.block == 0 { 0 } else { hit.block };
                    out += self.trace_recursive(Ray::new(rorigin, rdir), depth + 1, target_medium) * refract_amt;
                }
            }
            out
        } else if mat.reflectivity > 0.001 {
            let rdir = ray.dir.reflect(n);
            let rorigin = hit.point + geo_n * EPS;
            let refl = self.trace_recursive(Ray::new(rorigin, rdir), depth + 1, current_medium);
            local * (1.0 - mat.reflectivity) + refl * mat.reflectivity
        } else {
            local
        }
    }
}
