//! Fase "parte 2": el mundo deja de ser una sola grilla gigante mayormente
//! vacia y pasa a ser una lista de islas, cada una con su propia grilla de
//! voxeles chica (`World`) y un offset entero que la ubica en el espacio de
//! mundo. Un rayo primero se prueba contra el AABB de cada isla (pocas,
//! se ordenan por t de entrada) y solo hace DDA dentro de las que
//! realmente toca, parando en el primer hit valido -- asi sombras,
//! reflexion y refraccion funcionan igual de bien entre islas distintas
//! (una isla puede iluminar/reflejar lo que hay en otra) sin pagar memoria
//! por el espacio vacio entre ellas.

use crate::intersect::{intersect_aabb, traverse, Face, HitInfo};
use crate::math::{Ray, Vec3};
use crate::world::World;

pub struct Island {
    pub world: World,
    pub offset: (i32, i32, i32),
}

impl Island {
    pub fn new(world: World, offset: (i32, i32, i32)) -> Self {
        Island { world, offset }
    }

    #[inline]
    fn offset_vec(&self) -> Vec3 {
        Vec3::new(self.offset.0 as f32, self.offset.1 as f32, self.offset.2 as f32)
    }

    /// AABB de esta isla en coordenadas de mundo.
    pub fn world_aabb(&self) -> (Vec3, Vec3) {
        let (lo, hi) = self.world.aabb();
        let o = self.offset_vec();
        (lo + o, hi + o)
    }

    /// Convierte un punto de coordenadas de mundo a locales de esta isla.
    pub fn to_local_point(&self, world_point: Vec3) -> Vec3 {
        world_point - self.offset_vec()
    }

    /// Convierte un punto local de esta isla a coordenadas de mundo.
    pub fn to_world_point(&self, local_point: Vec3) -> Vec3 {
        local_point + self.offset_vec()
    }

    #[inline]
    fn to_local_ray(&self, ray: Ray) -> Ray {
        Ray::new(ray.origin - self.offset_vec(), ray.dir)
    }
}

/// Igual que `intersect::traverse`, pero contra una lista de islas: prueba
/// el AABB de cada una (las ordena por t de entrada), y solo entra al DDA de
/// las que el rayo realmente puede tocar, cortando apenas encuentra un hit
/// mas cercano que el resto de los candidatos pendientes. El `HitInfo` que
/// devuelve ya esta en coordenadas de mundo (listo para shading/sombras/
/// luces sin que a nadie mas le importe de que isla vino).
pub fn traverse_islands(islands: &[Island], ray: Ray, max_t: f32, accept: impl Fn(u8, Face, (f32, f32)) -> bool) -> Option<HitInfo> {
    // Candidatos: (t_enter, indice de isla), solo los que el rayo puede
    // llegar a tocar dentro de max_t. Arreglo fijo en el stack (no Vec): son
    // pocas islas y esto corre por cada rayo, primario o secundario.
    const MAX_ISLANDS: usize = 32;
    debug_assert!(islands.len() <= MAX_ISLANDS, "subir MAX_ISLANDS si hay mas islas que esto");
    let mut cand_t = [0.0f32; MAX_ISLANDS];
    let mut cand_idx = [0usize; MAX_ISLANDS];
    let mut n = 0usize;
    for (i, island) in islands.iter().enumerate().take(MAX_ISLANDS) {
        let (lo, hi) = island.world_aabb();
        if let Some((t_enter, t_exit)) = intersect_aabb(ray, lo, hi) {
            if t_exit >= 0.0 && t_enter <= max_t {
                cand_t[n] = t_enter.max(0.0);
                cand_idx[n] = i;
                n += 1;
            }
        }
    }
    // Insertion sort: n es chico (unas pocas islas), no hace falta nada mas.
    for i in 1..n {
        let (t, idx) = (cand_t[i], cand_idx[i]);
        let mut j = i;
        while j > 0 && cand_t[j - 1] > t {
            cand_t[j] = cand_t[j - 1];
            cand_idx[j] = cand_idx[j - 1];
            j -= 1;
        }
        cand_t[j] = t;
        cand_idx[j] = idx;
    }

    let mut best: Option<HitInfo> = None;
    let mut best_t = max_t;
    for i in 0..n {
        if cand_t[i] > best_t {
            break; // ninguna isla mas lejos puede dar un hit mas cercano
        }
        let island = &islands[cand_idx[i]];
        let local_ray = island.to_local_ray(ray);
        if let Some(hit) = traverse(&island.world, local_ray, best_t, &accept) {
            if hit.t < best_t {
                best_t = hit.t;
                let world_point = island.to_world_point(hit.point);
                best = Some(HitInfo { point: world_point, ..hit });
            }
        }
    }
    best
}
