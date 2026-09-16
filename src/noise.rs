//! PRNG (PCG32) y ruido Perlin 2D/3D con fBm, escritos a mano con std.
//! Usado por skybox.rs (nubes/estrellas) y terrain.rs (heightmap/cono de la isla).
//! noise3/fbm3/range_* se consumen a partir de fase 8 (terreno).
#![allow(dead_code)]

/// PCG32, PRNG propio de periodo largo y buena distribucion, mas simple que
/// Mersenne Twister y suficiente para ruido/generacion procedural.
pub struct Pcg32 {
    state: u64,
    inc: u64,
}

impl Pcg32 {
    pub fn new(seed: u64, seq: u64) -> Self {
        let mut rng = Pcg32 { state: 0, inc: (seq << 1) | 1 };
        rng.next_u32();
        rng.state = rng.state.wrapping_add(seed);
        rng.next_u32();
        rng
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        (xorshifted >> rot) | (xorshifted << ((32u32.wrapping_sub(rot)) & 31))
    }

    #[inline]
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() as f32) / (u32::MAX as f32 + 1.0)
    }

    #[inline]
    pub fn range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next_f32()
    }

    #[inline]
    pub fn range_i32(&mut self, lo: i32, hi_inclusive: i32) -> i32 {
        let span = (hi_inclusive - lo + 1).max(1) as u32;
        lo + (self.next_u32() % span) as i32
    }
}

#[inline]
fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[inline]
fn lerp(t: f32, a: f32, b: f32) -> f32 {
    a + t * (b - a)
}

#[inline]
fn grad2(hash: u8, x: f32, y: f32) -> f32 {
    match hash & 3 {
        0 => x + y,
        1 => -x + y,
        2 => x - y,
        _ => -x - y,
    }
}

#[inline]
fn grad3(hash: u8, x: f32, y: f32, z: f32) -> f32 {
    let h = hash & 15;
    let u = if h < 8 { x } else { y };
    let v = if h < 4 {
        y
    } else if h == 12 || h == 14 {
        x
    } else {
        z
    };
    (if h & 1 == 0 { u } else { -u }) + (if h & 2 == 0 { v } else { -v })
}

/// Ruido Perlin clasico (tabla de permutacion desde una semilla), 2D y 3D.
pub struct Perlin {
    perm: [u8; 512],
}

impl Perlin {
    pub fn new(seed: u32) -> Self {
        let mut p = [0u8; 256];
        for (i, slot) in p.iter_mut().enumerate() {
            *slot = i as u8;
        }
        let mut rng = Pcg32::new(seed as u64, 0xda3e_39cb_94b9_5bdbu64);
        for i in (1..256).rev() {
            let j = (rng.next_u32() as usize) % (i + 1);
            p.swap(i, j);
        }
        let mut perm = [0u8; 512];
        for (i, slot) in perm.iter_mut().enumerate() {
            *slot = p[i & 255];
        }
        Perlin { perm }
    }

    pub fn noise2(&self, x: f32, y: f32) -> f32 {
        let xi = (x.floor() as i32).rem_euclid(256) as usize;
        let yi = (y.floor() as i32).rem_euclid(256) as usize;
        let xf = x - x.floor();
        let yf = y - y.floor();
        let u = fade(xf);
        let v = fade(yf);

        let p = &self.perm;
        let aa = p[p[xi] as usize + yi];
        let ab = p[p[xi] as usize + yi + 1];
        let ba = p[p[xi + 1] as usize + yi];
        let bb = p[p[xi + 1] as usize + yi + 1];

        lerp(
            v,
            lerp(u, grad2(aa, xf, yf), grad2(ba, xf - 1.0, yf)),
            lerp(u, grad2(ab, xf, yf - 1.0), grad2(bb, xf - 1.0, yf - 1.0)),
        )
    }

    pub fn noise3(&self, x: f32, y: f32, z: f32) -> f32 {
        let xi = (x.floor() as i32).rem_euclid(256) as usize;
        let yi = (y.floor() as i32).rem_euclid(256) as usize;
        let zi = (z.floor() as i32).rem_euclid(256) as usize;
        let xf = x - x.floor();
        let yf = y - y.floor();
        let zf = z - z.floor();
        let u = fade(xf);
        let v = fade(yf);
        let w = fade(zf);

        let p = &self.perm;
        let a = p[xi] as usize + yi;
        let aa = p[a] as usize + zi;
        let ab = p[a + 1] as usize + zi;
        let b = p[xi + 1] as usize + yi;
        let ba = p[b] as usize + zi;
        let bb = p[b + 1] as usize + zi;

        lerp(
            w,
            lerp(
                v,
                lerp(u, grad3(p[aa], xf, yf, zf), grad3(p[ba], xf - 1.0, yf, zf)),
                lerp(u, grad3(p[ab], xf, yf - 1.0, zf), grad3(p[bb], xf - 1.0, yf - 1.0, zf)),
            ),
            lerp(
                v,
                lerp(u, grad3(p[aa + 1], xf, yf, zf - 1.0), grad3(p[ba + 1], xf - 1.0, yf, zf - 1.0)),
                lerp(u, grad3(p[ab + 1], xf, yf - 1.0, zf - 1.0), grad3(p[bb + 1], xf - 1.0, yf - 1.0, zf - 1.0)),
            ),
        )
    }

    /// Fractional Brownian motion: suma octavas de ruido 2D con lacunarity/gain.
    pub fn fbm2(&self, x: f32, y: f32, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
        let mut sum = 0.0;
        let mut amp = 0.5;
        let mut freq = 1.0;
        let mut norm = 0.0;
        for _ in 0..octaves {
            sum += amp * self.noise2(x * freq, y * freq);
            norm += amp;
            amp *= gain;
            freq *= lacunarity;
        }
        if norm > 1e-6 {
            sum / norm
        } else {
            0.0
        }
    }

    pub fn fbm3(&self, x: f32, y: f32, z: f32, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
        let mut sum = 0.0;
        let mut amp = 0.5;
        let mut freq = 1.0;
        let mut norm = 0.0;
        for _ in 0..octaves {
            sum += amp * self.noise3(x * freq, y * freq, z * freq);
            norm += amp;
            amp *= gain;
            freq *= lacunarity;
        }
        if norm > 1e-6 {
            sum / norm
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perlin_is_deterministic_and_bounded() {
        let p = Perlin::new(42);
        for i in 0..50 {
            let x = i as f32 * 0.37;
            let y = i as f32 * 0.71;
            let n = p.noise2(x, y);
            assert!((-1.5..=1.5).contains(&n));
            assert_eq!(n, p.noise2(x, y));
        }
    }

    #[test]
    fn pcg32_varies() {
        let mut rng = Pcg32::new(7, 1);
        let a = rng.next_u32();
        let b = rng.next_u32();
        assert_ne!(a, b);
    }
}
