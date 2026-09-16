// hollow_box se usa a partir de fase 9 (estructuras de la escena final).
#![allow(dead_code)]

use crate::math::Vec3;

/// Dense voxel grid. Block id 0 is always air. Voxel (x,y,z) occupies the
/// unit cube [x,x+1) x [y,y+1) x [z,z+1) in world space.
pub struct World {
    pub nx: i32,
    pub ny: i32,
    pub nz: i32,
    blocks: Vec<u8>,
}

impl World {
    pub fn new(nx: i32, ny: i32, nz: i32) -> Self {
        World {
            nx,
            ny,
            nz,
            blocks: vec![0u8; (nx * ny * nz) as usize],
        }
    }

    #[inline]
    fn in_bounds(&self, x: i32, y: i32, z: i32) -> bool {
        x >= 0 && y >= 0 && z >= 0 && x < self.nx && y < self.ny && z < self.nz
    }

    #[inline]
    fn idx(&self, x: i32, y: i32, z: i32) -> usize {
        ((y * self.nz + z) * self.nx + x) as usize
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32, z: i32) -> u8 {
        if self.in_bounds(x, y, z) {
            self.blocks[self.idx(x, y, z)]
        } else {
            0
        }
    }

    #[inline]
    pub fn set(&mut self, x: i32, y: i32, z: i32, id: u8) {
        if self.in_bounds(x, y, z) {
            let i = self.idx(x, y, z);
            self.blocks[i] = id;
        }
    }

    /// Fills the inclusive box [min,max] with `id`.
    pub fn fill_box(&mut self, min: (i32, i32, i32), max: (i32, i32, i32), id: u8) {
        for y in min.1.min(max.1)..=min.1.max(max.1) {
            for z in min.2.min(max.2)..=min.2.max(max.2) {
                for x in min.0.min(max.0)..=min.0.max(max.0) {
                    self.set(x, y, z, id);
                }
            }
        }
    }

    /// Fills only the shell of the inclusive box [min,max] with `id`, leaving the interior untouched.
    pub fn hollow_box(&mut self, min: (i32, i32, i32), max: (i32, i32, i32), id: u8) {
        let (x0, x1) = (min.0.min(max.0), min.0.max(max.0));
        let (y0, y1) = (min.1.min(max.1), min.1.max(max.1));
        let (z0, z1) = (min.2.min(max.2), min.2.max(max.2));
        for y in y0..=y1 {
            for z in z0..=z1 {
                for x in x0..=x1 {
                    let on_shell = x == x0 || x == x1 || y == y0 || y == y1 || z == z0 || z == z1;
                    if on_shell {
                        self.set(x, y, z, id);
                    }
                }
            }
        }
    }

    #[inline]
    pub fn aabb(&self) -> (Vec3, Vec3) {
        (Vec3::zero(), Vec3::new(self.nx as f32, self.ny as f32, self.nz as f32))
    }
}
