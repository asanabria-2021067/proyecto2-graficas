// specular/transparency/reflectivity/ior/normal se consumen en fase 4 (shading),
// fase 5 (reflexion/refraccion) y fase 6 (normal maps). AIR/COUNT son parte de la
// API publica del modulo de bloques, se usan en fase 8+ al generar terreno.
#![allow(dead_code)]

use crate::math::Vec3;
use crate::texture::{load_or_generate, Texture};

pub mod block {
    pub const AIR: u8 = 0;
    pub const GRASS: u8 = 1;
    pub const DIRT: u8 = 2;
    pub const SAND: u8 = 3;
    pub const STONE_BRICKS: u8 = 4;
    pub const OAK_LOG: u8 = 5;
    pub const OAK_PLANKS: u8 = 6;
    pub const LEAVES: u8 = 7;
    pub const WATER: u8 = 8;
    pub const GLASS: u8 = 9;
    pub const GLOWSTONE: u8 = 10;
    pub const IRON_BLOCK: u8 = 11;
    pub const LAMP_FRAME: u8 = 12;
    pub const COUNT: usize = 13;
}

pub struct FaceTex {
    pub albedo: Texture,
    pub normal: Texture,
}

impl FaceTex {
    fn new(albedo: Texture) -> Self {
        let normal = Texture::flat_normal(albedo.width, albedo.height);
        FaceTex { albedo, normal }
    }
}

pub struct Material {
    pub top: FaceTex,
    pub side: FaceTex,
    pub bottom: FaceTex,
    pub tint: Vec3,
    pub specular_coef: f32,
    pub specular_exp: f32,
    pub transparency: f32,
    pub reflectivity: f32,
    pub ior: f32,
    pub emission: Vec3,
    pub normal_strength: f32,
    pub alpha_cutout: bool,
    pub absorption: Vec3,
}

struct MatParams {
    specular_coef: f32,
    specular_exp: f32,
    transparency: f32,
    reflectivity: f32,
    ior: f32,
    emission: Vec3,
}

impl Default for MatParams {
    fn default() -> Self {
        MatParams { specular_coef: 0.0, specular_exp: 1.0, transparency: 0.0, reflectivity: 0.0, ior: 1.0, emission: Vec3::zero() }
    }
}

impl Material {
    /// Same procedurally-generated texture (regenerated per face, they're tiny
    /// 16x16 images so this is cheap) used for top/side/bottom.
    fn uniform(name: &str, seed: u32, p: MatParams) -> Self {
        Material {
            top: FaceTex::new(load_or_generate(name, seed)),
            side: FaceTex::new(load_or_generate(name, seed)),
            bottom: FaceTex::new(load_or_generate(name, seed)),
            tint: Vec3::splat(1.0),
            specular_coef: p.specular_coef,
            specular_exp: p.specular_exp,
            transparency: p.transparency,
            reflectivity: p.reflectivity,
            ior: p.ior,
            emission: p.emission,
            normal_strength: 1.0,
            alpha_cutout: false,
            absorption: Vec3::zero(),
        }
    }
}

pub struct MaterialTable {
    entries: Vec<Option<Material>>,
}

impl MaterialTable {
    #[inline]
    pub fn get(&self, id: u8) -> Option<&Material> {
        self.entries.get(id as usize).and_then(|m| m.as_ref())
    }
}

pub fn build_material_table(seed: u32) -> MaterialTable {
    let mut entries: Vec<Option<Material>> = (0..256).map(|_| None).collect();

    let mut grass = Material::uniform("grass_top", seed, MatParams { specular_coef: 0.05, specular_exp: 8.0, ..Default::default() });
    grass.side = FaceTex::new(load_or_generate("grass_side", seed));
    grass.bottom = FaceTex::new(load_or_generate("dirt", seed));
    entries[block::GRASS as usize] = Some(grass);

    entries[block::DIRT as usize] = Some(Material::uniform("dirt", seed, MatParams { specular_coef: 0.03, specular_exp: 6.0, ..Default::default() }));
    entries[block::SAND as usize] = Some(Material::uniform("sand", seed, MatParams { specular_coef: 0.05, specular_exp: 10.0, ..Default::default() }));

    let mut stone_bricks = Material::uniform("stone_bricks", seed, MatParams { specular_coef: 0.15, specular_exp: 24.0, reflectivity: 0.05, ..Default::default() });
    stone_bricks.normal_strength = 1.2;
    entries[block::STONE_BRICKS as usize] = Some(stone_bricks);

    let mut oak_log = Material::uniform("oak_log_top", seed, MatParams { specular_coef: 0.05, specular_exp: 8.0, ..Default::default() });
    oak_log.side = FaceTex::new(load_or_generate("oak_log_side", seed));
    oak_log.bottom = FaceTex::new(load_or_generate("oak_log_top", seed));
    entries[block::OAK_LOG as usize] = Some(oak_log);

    let mut oak_planks = Material::uniform("oak_planks", seed, MatParams { specular_coef: 0.08, specular_exp: 12.0, ..Default::default() });
    oak_planks.normal_strength = 0.8;
    entries[block::OAK_PLANKS as usize] = Some(oak_planks);

    let mut leaves = Material::uniform("leaves", seed, MatParams { specular_coef: 0.02, specular_exp: 4.0, ..Default::default() });
    leaves.alpha_cutout = true;
    entries[block::LEAVES as usize] = Some(leaves);

    let mut water = Material::uniform("water", seed, MatParams { specular_coef: 0.6, specular_exp: 90.0, transparency: 0.85, reflectivity: 0.08, ior: 1.33, emission: Vec3::zero() });
    water.absorption = Vec3::new(0.35, 0.12, 0.08);
    entries[block::WATER as usize] = Some(water);

    let glass = Material::uniform("glass", seed, MatParams { specular_coef: 0.6, specular_exp: 120.0, transparency: 0.92, reflectivity: 0.06, ior: 1.5, emission: Vec3::zero() });
    entries[block::GLASS as usize] = Some(glass);

    let glowstone = Material::uniform("glowstone", seed, MatParams { emission: Vec3::new(1.0, 0.85, 0.5) * 2.5, ..Default::default() });
    entries[block::GLOWSTONE as usize] = Some(glowstone);

    let mut iron_block = Material::uniform("iron_block", seed, MatParams { specular_coef: 0.4, specular_exp: 60.0, reflectivity: 0.55, ..Default::default() });
    iron_block.normal_strength = 0.6;
    entries[block::IRON_BLOCK as usize] = Some(iron_block);

    let mut lamp_frame = Material::uniform("lamp_frame", seed, MatParams { specular_coef: 0.1, specular_exp: 20.0, ..Default::default() });
    lamp_frame.alpha_cutout = true;
    entries[block::LAMP_FRAME as usize] = Some(lamp_frame);

    MaterialTable { entries }
}
