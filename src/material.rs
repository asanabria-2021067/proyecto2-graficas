// specular/transparency/reflectivity/ior/normal se consumen en fase 4 (shading),
// fase 5 (reflexion/refraccion) y fase 6 (normal maps). AIR/COUNT son parte de la
// API publica del modulo de bloques, se usan en fase 8+ al generar terreno.
#![allow(dead_code)]

use crate::math::Vec3;
use crate::texture::{load_or_generate, load_or_generate_normal, load_or_generate_normal_named, Texture};

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
    pub const NETHERRACK: u8 = 13;
    pub const NETHER_BRICKS: u8 = 14;
    pub const LAVA: u8 = 15;
    pub const OBSIDIAN: u8 = 16;
    pub const PORTAL: u8 = 17;
    pub const MAGMA: u8 = 18;
    pub const BASALT: u8 = 19;
    pub const END_STONE: u8 = 20;
    pub const PURPUR: u8 = 21;
    pub const END_CRYSTAL: u8 = 22;
    pub const END_ROD: u8 = 23;
    pub const CHORUS: u8 = 24;
    pub const END_STONE_BRICKS: u8 = 25;
    pub const COUNT: usize = 26;
}

pub struct FaceTex {
    pub albedo: Texture,
    pub normal: Texture,
    /// Si esta presente, la emision se muestrea aca en vez de usar el valor
    /// plano `Material::emission` -- solo el magma la usa (para que unicamente
    /// las grietas brillen, no todo el bloque).
    pub emission_map: Option<Texture>,
}

impl FaceTex {
    fn new(albedo: Texture) -> Self {
        let normal = Texture::flat_normal(albedo.width, albedo.height);
        FaceTex { albedo, normal, emission_map: None }
    }

    /// Uses assets/textures/<name>_n.bmp if present, otherwise derives a
    /// normal map from the albedo's own texture detail (Sobel).
    fn bumped(name: &str, seed: u32) -> Self {
        let albedo = load_or_generate(name, seed);
        let normal = load_or_generate_normal(name, &albedo);
        FaceTex { albedo, normal, emission_map: None }
    }

    /// Como `bumped`, pero el normal map NO se deriva del albedo (Sobel):
    /// se genera/carga con su propio nombre (`<name>_normal`). Lo usa el
    /// portal para su remolino "magico".
    fn bumped_named_normal(name: &str, normal_name: &str, seed: u32) -> Self {
        let albedo = load_or_generate(name, seed);
        let normal = load_or_generate_normal_named(normal_name, seed);
        FaceTex { albedo, normal, emission_map: None }
    }

    /// Como `new`, pero con un mapa de emision separado (mismo nombre +
    /// sufijo `_emission`) que solo el magma usa.
    fn with_emission_map(name: &str, seed: u32) -> Self {
        let albedo = load_or_generate(name, seed);
        let normal = Texture::flat_normal(albedo.width, albedo.height);
        let emission_map = Some(load_or_generate(&format!("{name}_emission"), seed));
        FaceTex { albedo, normal, emission_map }
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

    /// Same as `uniform` but with a real (loaded or Sobel-derived) normal map
    /// on every face instead of the flat placeholder.
    fn uniform_bumped(name: &str, seed: u32, p: MatParams) -> Self {
        Material {
            top: FaceTex::bumped(name, seed),
            side: FaceTex::bumped(name, seed),
            bottom: FaceTex::bumped(name, seed),
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

    let mut stone_bricks = Material::uniform_bumped("stone_bricks", seed, MatParams { specular_coef: 0.15, specular_exp: 24.0, reflectivity: 0.05, ..Default::default() });
    stone_bricks.normal_strength = 1.2;
    entries[block::STONE_BRICKS as usize] = Some(stone_bricks);

    let mut oak_log = Material::uniform("oak_log_top", seed, MatParams { specular_coef: 0.05, specular_exp: 8.0, ..Default::default() });
    oak_log.top = FaceTex::bumped("oak_log_top", seed);
    oak_log.side = FaceTex::bumped("oak_log_side", seed);
    oak_log.bottom = FaceTex::bumped("oak_log_top", seed);
    oak_log.normal_strength = 1.0;
    entries[block::OAK_LOG as usize] = Some(oak_log);

    let mut oak_planks = Material::uniform_bumped("oak_planks", seed, MatParams { specular_coef: 0.08, specular_exp: 12.0, ..Default::default() });
    oak_planks.normal_strength = 0.8;
    entries[block::OAK_PLANKS as usize] = Some(oak_planks);

    let mut leaves = Material::uniform("leaves", seed, MatParams { specular_coef: 0.02, specular_exp: 4.0, ..Default::default() });
    leaves.alpha_cutout = true;
    entries[block::LEAVES as usize] = Some(leaves);

    let mut water = Material::uniform("water", seed, MatParams { specular_coef: 0.6, specular_exp: 90.0, transparency: 0.85, reflectivity: 0.30, ior: 1.33, emission: Vec3::zero() });
    water.absorption = Vec3::new(0.14, 0.045, 0.025);
    entries[block::WATER as usize] = Some(water);

    let glass = Material::uniform("glass", seed, MatParams { specular_coef: 0.6, specular_exp: 120.0, transparency: 0.92, reflectivity: 0.06, ior: 1.5, emission: Vec3::zero() });
    entries[block::GLASS as usize] = Some(glass);

    let glowstone = Material::uniform("glowstone", seed, MatParams { emission: Vec3::new(1.0, 0.85, 0.5) * 2.5, ..Default::default() });
    entries[block::GLOWSTONE as usize] = Some(glowstone);

    let mut iron_block = Material::uniform_bumped("iron_block", seed, MatParams { specular_coef: 0.4, specular_exp: 60.0, reflectivity: 0.55, ..Default::default() });
    iron_block.normal_strength = 0.6;
    entries[block::IRON_BLOCK as usize] = Some(iron_block);

    let mut lamp_frame = Material::uniform("lamp_frame", seed, MatParams { specular_coef: 0.1, specular_exp: 20.0, ..Default::default() });
    lamp_frame.alpha_cutout = true;
    entries[block::LAMP_FRAME as usize] = Some(lamp_frame);

    // ---------- Nether ----------

    let mut netherrack = Material::uniform_bumped("netherrack", seed, MatParams { specular_coef: 0.04, specular_exp: 6.0, ..Default::default() });
    netherrack.normal_strength = 1.4; // rugosa, se nota bien con luz rasante
    entries[block::NETHERRACK as usize] = Some(netherrack);

    let mut nether_bricks = Material::uniform_bumped("nether_bricks", seed, MatParams { specular_coef: 0.12, specular_exp: 20.0, reflectivity: 0.03, ..Default::default() });
    nether_bricks.normal_strength = 1.1;
    entries[block::NETHER_BRICKS as usize] = Some(nether_bricks);

    // Lava: emisiva intensa, opaca (nada de transparencia/refraccion), se
    // registra sola como luz puntual (lights.rs mira cualquier material con
    // emission > 0). No hace falta logica especial de "sin sombra": la
    // emision ya se suma sin pasar por el rayo de sombra en shading.rs.
    let lava = Material::uniform("lava", seed, MatParams { specular_coef: 0.3, specular_exp: 40.0, emission: Vec3::new(1.0, 0.45, 0.08) * 3.2, ..Default::default() });
    entries[block::LAVA as usize] = Some(lava);

    let mut obsidian = Material::uniform("obsidian", seed, MatParams { specular_coef: 0.5, specular_exp: 80.0, reflectivity: 0.35, ..Default::default() });
    obsidian.normal_strength = 0.3;
    entries[block::OBSIDIAN as usize] = Some(obsidian);

    // Portal: translucido morado con ior bajo (efecto "liviano") y un normal
    // map en remolino (no derivado del albedo) para que la refraccion se
    // vea distorsionada, magica.
    let portal = Material {
        top: FaceTex::bumped_named_normal("portal", "portal_normal", seed),
        side: FaceTex::bumped_named_normal("portal", "portal_normal", seed),
        bottom: FaceTex::bumped_named_normal("portal", "portal_normal", seed),
        tint: Vec3::splat(1.0),
        specular_coef: 0.4,
        specular_exp: 30.0,
        transparency: 0.85,
        reflectivity: 0.05,
        ior: 1.1,
        emission: Vec3::new(0.55, 0.15, 0.85) * 0.6,
        normal_strength: 2.2,
        alpha_cutout: false,
        absorption: Vec3::new(0.05, 0.1, 0.02),
    };
    entries[block::PORTAL as usize] = Some(portal);

    // Magma: la emision sale de un mapa (magma_emission), asi que solo las
    // grietas brillan en vez de todo el bloque.
    let magma = Material {
        top: FaceTex::with_emission_map("magma", seed),
        side: FaceTex::with_emission_map("magma", seed),
        bottom: FaceTex::with_emission_map("magma", seed),
        tint: Vec3::splat(1.0),
        specular_coef: 0.1,
        specular_exp: 10.0,
        transparency: 0.0,
        reflectivity: 0.0,
        ior: 1.0,
        emission: Vec3::new(1.0, 0.5, 0.1) * 2.0, // usado solo si emission_map llegara a faltar
        normal_strength: 1.0,
        alpha_cutout: false,
        absorption: Vec3::zero(),
    };
    entries[block::MAGMA as usize] = Some(magma);

    let mut basalt = Material::uniform("basalt", seed, MatParams { specular_coef: 0.1, specular_exp: 14.0, ..Default::default() });
    basalt.normal_strength = 0.5;
    entries[block::BASALT as usize] = Some(basalt);

    // ---------- End ----------

    let mut end_stone = Material::uniform_bumped("end_stone", seed, MatParams { specular_coef: 0.06, specular_exp: 10.0, ..Default::default() });
    end_stone.normal_strength = 0.7;
    entries[block::END_STONE as usize] = Some(end_stone);

    let mut purpur = Material::uniform_bumped("purpur", seed, MatParams { specular_coef: 0.25, specular_exp: 30.0, ..Default::default() });
    purpur.normal_strength = 0.9;
    entries[block::PURPUR as usize] = Some(purpur);

    let mut end_stone_bricks = Material::uniform_bumped("end_stone_bricks", seed, MatParams { specular_coef: 0.1, specular_exp: 16.0, ..Default::default() });
    end_stone_bricks.normal_strength = 1.0;
    entries[block::END_STONE_BRICKS as usize] = Some(end_stone_bricks);

    // Cristal del End: emisivo (se registra como luz puntual), refractivo
    // (ior alto, como vidrio grueso) y muy reflectivo.
    let end_crystal = Material::uniform("end_crystal", seed, MatParams {
        specular_coef: 0.7,
        specular_exp: 100.0,
        transparency: 0.6,
        reflectivity: 0.5,
        ior: 1.6,
        emission: Vec3::new(0.9, 0.55, 1.0) * 1.8,
    });
    entries[block::END_CRYSTAL as usize] = Some(end_crystal);

    // Barra fina blanca emisiva: bloque entero por simplicidad, pero se lee
    // como lampara por la emision fuerte y pareja.
    let end_rod = Material::uniform("end_rod", seed, MatParams { specular_coef: 0.3, specular_exp: 40.0, emission: Vec3::new(0.9, 0.95, 1.0) * 2.0, ..Default::default() });
    entries[block::END_ROD as usize] = Some(end_rod);

    let mut chorus = Material::uniform("chorus", seed, MatParams { specular_coef: 0.05, specular_exp: 6.0, ..Default::default() });
    chorus.alpha_cutout = true;
    entries[block::CHORUS as usize] = Some(chorus);

    MaterialTable { entries }
}
