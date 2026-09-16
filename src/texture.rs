// sample_rgb se usa a partir de fase 6 (muestreo de normal maps sin alpha).
#![allow(dead_code)]

use crate::image_io;
use crate::math::{srgb_to_linear, Vec3};
use crate::texgen;

pub struct Texture {
    pub width: u32,
    pub height: u32,
    rgb: Vec<Vec3>,
    alpha: Vec<f32>,
}

impl Texture {
    pub fn from_rgba8(width: u32, height: u32, data: &[u8]) -> Texture {
        let n = (width * height) as usize;
        let mut rgb = Vec::with_capacity(n);
        let mut alpha = Vec::with_capacity(n);
        for i in 0..n {
            let r = data[i * 4] as f32 / 255.0;
            let g = data[i * 4 + 1] as f32 / 255.0;
            let b = data[i * 4 + 2] as f32 / 255.0;
            let a = data[i * 4 + 3] as f32 / 255.0;
            rgb.push(Vec3::new(srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b)));
            alpha.push(a);
        }
        Texture { width, height, rgb, alpha }
    }

    /// Like `from_rgba8` but without the sRGB->linear decode: normal maps
    /// encode direction vectors, not gamma-corrected color.
    pub fn from_rgba8_raw(width: u32, height: u32, data: &[u8]) -> Texture {
        let n = (width * height) as usize;
        let mut rgb = Vec::with_capacity(n);
        let mut alpha = Vec::with_capacity(n);
        for i in 0..n {
            rgb.push(Vec3::new(data[i * 4] as f32 / 255.0, data[i * 4 + 1] as f32 / 255.0, data[i * 4 + 2] as f32 / 255.0));
            alpha.push(data[i * 4 + 3] as f32 / 255.0);
        }
        Texture { width, height, rgb, alpha }
    }

    #[inline]
    fn texel_index(&self, x: i32, y: i32) -> usize {
        let xu = x.rem_euclid(self.width as i32) as u32;
        let yu = y.rem_euclid(self.height as i32) as u32;
        (yu * self.width + xu) as usize
    }

    /// Flat "no bump" normal map: tangent-space (0,0,1) everywhere, encoded as (0.5,0.5,1.0).
    pub fn flat_normal(width: u32, height: u32) -> Texture {
        let n = (width * height) as usize;
        Texture {
            width,
            height,
            rgb: vec![Vec3::new(0.5, 0.5, 1.0); n],
            alpha: vec![1.0; n],
        }
    }

    #[inline]
    fn texel(&self, u: f32, v: f32) -> usize {
        let mut uu = u - u.floor();
        let mut vv = v - v.floor();
        if !(0.0..1.0).contains(&uu) {
            uu = 0.0;
        }
        if !(0.0..1.0).contains(&vv) {
            vv = 0.0;
        }
        let x = ((uu * self.width as f32) as u32).min(self.width - 1);
        let y = ((vv * self.height as f32) as u32).min(self.height - 1);
        (y * self.width + x) as usize
    }

    #[inline]
    pub fn sample(&self, u: f32, v: f32) -> (Vec3, f32) {
        let idx = self.texel(u, v);
        (self.rgb[idx], self.alpha[idx])
    }

    #[inline]
    pub fn sample_rgb(&self, u: f32, v: f32) -> Vec3 {
        self.rgb[self.texel(u, v)]
    }

    #[inline]
    fn luminance_at(&self, x: i32, y: i32) -> f32 {
        let c = self.rgb[self.texel_index(x, y)];
        0.299 * c.x + 0.587 * c.y + 0.114 * c.z
    }
}

/// Derives a tangent-space normal map from an albedo texture's luminance
/// using a Sobel edge filter, for materials without a hand-painted `_n.bmp`.
pub fn generate_normal_from_albedo(albedo: &Texture) -> Texture {
    const EDGE_SCALE: f32 = 3.0;
    let w = albedo.width;
    let h = albedo.height;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let tl = albedo.luminance_at(x - 1, y - 1);
            let t = albedo.luminance_at(x, y - 1);
            let tr = albedo.luminance_at(x + 1, y - 1);
            let l = albedo.luminance_at(x - 1, y);
            let r = albedo.luminance_at(x + 1, y);
            let bl = albedo.luminance_at(x - 1, y + 1);
            let b = albedo.luminance_at(x, y + 1);
            let br = albedo.luminance_at(x + 1, y + 1);

            let gx = (tr + 2.0 * r + br) - (tl + 2.0 * l + bl);
            let gy = (bl + 2.0 * b + br) - (tl + 2.0 * t + tr);

            let n = Vec3::new(-gx * EDGE_SCALE, -gy * EDGE_SCALE, 1.0).normalize();
            let idx = ((y as u32 * w + x as u32) * 4) as usize;
            data[idx] = ((n.x * 0.5 + 0.5) * 255.0) as u8;
            data[idx + 1] = ((n.y * 0.5 + 0.5) * 255.0) as u8;
            data[idx + 2] = ((n.z * 0.5 + 0.5) * 255.0) as u8;
            data[idx + 3] = 255;
        }
    }
    Texture::from_rgba8_raw(w, h, &data)
}

/// Loads `assets/textures/<name>.bmp` if present, otherwise generates a
/// procedural 16x16 pixel-art texture with the same name.
pub fn load_or_generate(name: &str, seed: u32) -> Texture {
    let path = format!("assets/textures/{name}.bmp");
    if let Ok(bmp) = image_io::read_bmp(&path) {
        Texture::from_rgba8(bmp.width, bmp.height, &bmp.rgba)
    } else {
        let (w, h, data) = texgen::generate(name, seed);
        Texture::from_rgba8(w, h, &data)
    }
}

/// Loads `assets/textures/<name>_n.bmp` if present, otherwise derives a
/// normal map from `albedo`'s luminance via a Sobel filter.
pub fn load_or_generate_normal(name: &str, albedo: &Texture) -> Texture {
    let path = format!("assets/textures/{name}_n.bmp");
    if let Ok(bmp) = image_io::read_bmp(&path) {
        Texture::from_rgba8_raw(bmp.width, bmp.height, &bmp.rgba)
    } else {
        generate_normal_from_albedo(albedo)
    }
}
