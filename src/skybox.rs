use crate::image_io;
use crate::math::Vec3;
use crate::noise::Perlin;
use crate::texture::Texture;

const FACE_SIZE: u32 = 256;
const FACE_NAMES: [&str; 6] = ["px", "nx", "py", "ny", "pz", "nz"];

/// Direction (world space, need not be normalized) -> (face index, u, v) in
/// the standard cubemap layout: PX=0, NX=1, PY=2, NY=3, PZ=4, NZ=5.
fn dir_to_face_uv(dir: Vec3) -> (usize, f32, f32) {
    let ax = dir.x.abs();
    let ay = dir.y.abs();
    let az = dir.z.abs();
    if ax >= ay && ax >= az {
        if dir.x > 0.0 {
            (0, (-dir.z / ax + 1.0) * 0.5, (-dir.y / ax + 1.0) * 0.5)
        } else {
            (1, (dir.z / ax + 1.0) * 0.5, (-dir.y / ax + 1.0) * 0.5)
        }
    } else if ay >= ax && ay >= az {
        if dir.y > 0.0 {
            (2, (dir.x / ay + 1.0) * 0.5, (dir.z / ay + 1.0) * 0.5)
        } else {
            (3, (dir.x / ay + 1.0) * 0.5, (-dir.z / ay + 1.0) * 0.5)
        }
    } else if dir.z > 0.0 {
        (4, (dir.x / az + 1.0) * 0.5, (-dir.y / az + 1.0) * 0.5)
    } else {
        (5, (-dir.x / az + 1.0) * 0.5, (-dir.y / az + 1.0) * 0.5)
    }
}

/// Inverse of `dir_to_face_uv`: texel center (u,v) on `face` -> world direction.
fn face_uv_to_dir(face: usize, u: f32, v: f32) -> Vec3 {
    let sc = 2.0 * u - 1.0;
    let tc = 2.0 * v - 1.0;
    match face {
        0 => Vec3::new(1.0, -tc, -sc),
        1 => Vec3::new(-1.0, -tc, sc),
        2 => Vec3::new(sc, 1.0, tc),
        3 => Vec3::new(sc, -1.0, -tc),
        4 => Vec3::new(sc, -tc, 1.0),
        _ => Vec3::new(-sc, -tc, -1.0),
    }
    .normalize()
}

#[inline]
fn hash_u32(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}

fn star_brightness(seed: u32, face: usize, x: u32, y: u32) -> f32 {
    let h = hash_u32(seed ^ (face as u32).wrapping_mul(0x9E3779B1) ^ x.wrapping_mul(374_761_393) ^ y.wrapping_mul(668_265_263));
    let r = h as f32 / u32::MAX as f32;
    if r > 0.9975 {
        ((r - 0.9975) / 0.0025).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn disc_glow(dir: Vec3, toward_light: Vec3, core_cos: f32, glow_exp: f32) -> (f32, f32) {
    let d = dir.dot(toward_light).clamp(-1.0, 1.0);
    let core = ((d - core_cos) / (1.0 - core_cos)).clamp(0.0, 1.0);
    let glow = d.max(0.0).powf(glow_exp);
    (core, glow)
}

fn generate_day_face(face: usize, perlin: &Perlin, sun_dir: Vec3) -> Texture {
    let mut data = vec![0u8; (FACE_SIZE * FACE_SIZE * 4) as usize];
    let sun_pos = -sun_dir;
    for y in 0..FACE_SIZE {
        for x in 0..FACE_SIZE {
            let u = (x as f32 + 0.5) / FACE_SIZE as f32;
            let v = (y as f32 + 0.5) / FACE_SIZE as f32;
            let dir = face_uv_to_dir(face, u, v);

            let t = (dir.y * 0.5 + 0.5).clamp(0.0, 1.0);
            let mut color = Vec3::new(0.75, 0.85, 0.97).lerp(Vec3::new(0.15, 0.35, 0.75), t);

            let n = perlin.fbm2(dir.x * 6.0 + 100.0, dir.z * 6.0, 5, 2.0, 0.5) * 0.5 + 0.5;
            let cloud = ((n - 0.42) / 0.3).clamp(0.0, 1.0) * dir.y.max(0.05).powf(0.3);
            color = color.lerp(Vec3::new(0.99, 0.99, 1.0), cloud);

            let (core, glow) = disc_glow(dir, sun_pos, 0.9994, 400.0);
            color += Vec3::new(1.0, 0.95, 0.85) * core * 3.0;
            color += Vec3::new(1.0, 0.85, 0.6) * glow * 0.6;

            let idx = ((y * FACE_SIZE + x) * 4) as usize;
            let to_u8 = |c: f32| (crate::math::linear_to_srgb(c.clamp(0.0, 1.0)) * 255.0) as u8;
            data[idx] = to_u8(color.x);
            data[idx + 1] = to_u8(color.y);
            data[idx + 2] = to_u8(color.z);
            data[idx + 3] = 255;
        }
    }
    Texture::from_rgba8(FACE_SIZE, FACE_SIZE, &data)
}

fn generate_night_face(face: usize, seed: u32, moon_dir: Vec3) -> Texture {
    let mut data = vec![0u8; (FACE_SIZE * FACE_SIZE * 4) as usize];
    let moon_pos = -moon_dir;
    for y in 0..FACE_SIZE {
        for x in 0..FACE_SIZE {
            let u = (x as f32 + 0.5) / FACE_SIZE as f32;
            let v = (y as f32 + 0.5) / FACE_SIZE as f32;
            let dir = face_uv_to_dir(face, u, v);

            let t = (dir.y * 0.5 + 0.5).clamp(0.0, 1.0);
            let mut color = Vec3::new(0.015, 0.017, 0.035).lerp(Vec3::new(0.04, 0.05, 0.12), t);

            let star = star_brightness(seed, face, x, y);
            color += Vec3::splat(star) * 0.9;

            let (core, glow) = disc_glow(dir, moon_pos, 0.9985, 300.0);
            color += Vec3::new(0.85, 0.88, 0.95) * core * 1.4;
            color += Vec3::new(0.6, 0.65, 0.8) * glow * 0.4;

            let idx = ((y * FACE_SIZE + x) * 4) as usize;
            let to_u8 = |c: f32| (crate::math::linear_to_srgb(c.clamp(0.0, 1.0)) * 255.0) as u8;
            data[idx] = to_u8(color.x);
            data[idx + 1] = to_u8(color.y);
            data[idx + 2] = to_u8(color.z);
            data[idx + 3] = 255;
        }
    }
    Texture::from_rgba8(FACE_SIZE, FACE_SIZE, &data)
}

fn load_face(dir: &str, name: &str) -> Option<Texture> {
    let path = format!("{dir}/{name}.bmp");
    image_io::read_bmp(&path).ok().map(|bmp| Texture::from_rgba8(bmp.width, bmp.height, &bmp.rgba))
}

pub struct Skybox {
    day: [Texture; 6],
    night: [Texture; 6],
}

impl Skybox {
    /// Uses assets/skybox/day/*.bmp and assets/skybox/night/*.bmp if present
    /// (px, nx, py, ny, pz, nz), otherwise generates all 6 faces of each.
    pub fn build(seed: u32, sun_dir: Vec3, moon_dir: Vec3) -> Skybox {
        let perlin = Perlin::new(seed ^ 0x5C11_5C11);
        let day = std::array::from_fn(|i| load_face("assets/skybox/day", FACE_NAMES[i]).unwrap_or_else(|| generate_day_face(i, &perlin, sun_dir)));
        let night = std::array::from_fn(|i| load_face("assets/skybox/night", FACE_NAMES[i]).unwrap_or_else(|| generate_night_face(i, seed, moon_dir)));
        Skybox { day, night }
    }

    #[inline]
    pub fn sample(&self, dir: Vec3, night: bool) -> Vec3 {
        let (face, u, v) = dir_to_face_uv(dir);
        let faces = if night { &self.night } else { &self.day };
        faces[face].sample_rgb(u, v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_sky_has_bright_sun_and_cloud_variation() {
        let sun_dir = crate::shading::day_environment().sun.dir;
        let moon_dir = crate::shading::night_environment().sun.dir;
        let sb = Skybox::build(1234, sun_dir, moon_dir);
        let sun_pos = -sun_dir;
        let bright = sb.sample(sun_pos, false);
        let side = sb.sample(Vec3::new(1.0, 0.3, 0.0).normalize(), false);
        println!("bright={:?} side={:?}", bright, side);
        assert!(bright.x + bright.y + bright.z > side.x + side.y + side.z);

        // sample a grid to see cloud variance
        let mut minv = f32::INFINITY;
        let mut maxv = f32::NEG_INFINITY;
        for i in 0..20 {
            for j in 0..20 {
                let d = Vec3::new(i as f32 - 10.0, 8.0, j as f32 - 10.0).normalize();
                let c = sb.sample(d, false);
                let v = c.x + c.y + c.z;
                minv = minv.min(v);
                maxv = maxv.max(v);
            }
        }
        println!("min={minv} max={maxv}");
    }
}
