//! Genera texturas pixel-art 16x16 por codigo, con un PRNG por hash propio
//! (splitmix-style), sin dependencias externas.

pub const SIZE: u32 = 16;

#[inline]
fn hash_u32(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}

#[inline]
fn rand01(seed: u32, x: u32, y: u32) -> f32 {
    let h = hash_u32(seed ^ x.wrapping_mul(374_761_393) ^ y.wrapping_mul(668_265_263));
    (h as f32) / (u32::MAX as f32)
}

#[inline]
fn put(buf: &mut [u8], x: u32, y: u32, rgba: [u8; 4]) {
    let i = ((y * SIZE + x) * 4) as usize;
    buf[i] = rgba[0];
    buf[i + 1] = rgba[1];
    buf[i + 2] = rgba[2];
    buf[i + 3] = rgba[3];
}

#[inline]
fn vary(base: u8, amount: i32, n: f32) -> u8 {
    let delta = ((n - 0.5) * 2.0 * amount as f32) as i32;
    (base as i32 + delta).clamp(0, 255) as u8
}

fn blank() -> Vec<u8> {
    vec![0u8; (SIZE * SIZE * 4) as usize]
}

fn grass_top(seed: u32) -> Vec<u8> {
    let mut buf = blank();
    for y in 0..SIZE {
        for x in 0..SIZE {
            let n = rand01(seed, x, y);
            let r = vary(78, 18, n);
            let g = vary(148, 22, n);
            let b = vary(58, 16, n);
            put(&mut buf, x, y, [r, g, b, 255]);
        }
    }
    buf
}

fn grass_side(seed: u32) -> Vec<u8> {
    let mut buf = blank();
    for x in 0..SIZE {
        let edge = 3 + (rand01(seed ^ 0xA1, x, 0) * 2.5) as u32;
        for y in 0..SIZE {
            let n = rand01(seed, x, y);
            if y < edge {
                let r = vary(80, 18, n);
                let g = vary(146, 20, n);
                let b = vary(58, 16, n);
                put(&mut buf, x, y, [r, g, b, 255]);
            } else {
                let r = vary(121, 16, n);
                let g = vary(87, 14, n);
                let b = vary(58, 12, n);
                put(&mut buf, x, y, [r, g, b, 255]);
            }
        }
    }
    buf
}

fn dirt(seed: u32) -> Vec<u8> {
    let mut buf = blank();
    for y in 0..SIZE {
        for x in 0..SIZE {
            let n = rand01(seed, x, y);
            let r = vary(121, 18, n);
            let g = vary(87, 15, n);
            let b = vary(58, 12, n);
            put(&mut buf, x, y, [r, g, b, 255]);
        }
    }
    buf
}

fn sand(seed: u32) -> Vec<u8> {
    let mut buf = blank();
    for y in 0..SIZE {
        for x in 0..SIZE {
            let n = rand01(seed, x, y);
            let r = vary(214, 10, n);
            let g = vary(198, 10, n);
            let b = vary(150, 10, n);
            put(&mut buf, x, y, [r, g, b, 255]);
        }
    }
    buf
}

fn stone_bricks(seed: u32) -> Vec<u8> {
    let mut buf = blank();
    let brick_w = 8u32;
    let brick_h = 4u32;
    for y in 0..SIZE {
        let row = y / brick_h;
        let offset = if row.is_multiple_of(2) { 0 } else { brick_w / 2 };
        for x in 0..SIZE {
            let bx = (x + offset) % SIZE;
            let mortar = bx.is_multiple_of(brick_w) || y % brick_h == 0;
            let n = rand01(seed, x, y);
            if mortar {
                let v = vary(90, 10, n);
                put(&mut buf, x, y, [v, v, v, 255]);
            } else {
                let v = vary(160, 14, n);
                put(&mut buf, x, y, [v, v, v.saturating_add(2), 255]);
            }
        }
    }
    buf
}

fn oak_log_side(seed: u32) -> Vec<u8> {
    let mut buf = blank();
    for y in 0..SIZE {
        for x in 0..SIZE {
            let n = rand01(seed, x, y);
            let streak = ((x as f32 * 1.7).sin() * 0.5 + 0.5) * 0.3;
            let r = vary(101, 12, n) as f32 - streak * 25.0;
            let g = vary(72, 10, n) as f32 - streak * 18.0;
            let b = vary(45, 8, n) as f32 - streak * 10.0;
            put(&mut buf, x, y, [r.clamp(0.0, 255.0) as u8, g.clamp(0.0, 255.0) as u8, b.clamp(0.0, 255.0) as u8, 255]);
        }
    }
    buf
}

fn oak_log_top(seed: u32) -> Vec<u8> {
    let mut buf = blank();
    let c = (SIZE as f32 - 1.0) / 2.0;
    for y in 0..SIZE {
        for x in 0..SIZE {
            let n = rand01(seed, x, y);
            let dx = x as f32 - c;
            let dy = y as f32 - c;
            let dist = (dx * dx + dy * dy).sqrt();
            let ring = (dist * 1.3).sin() * 0.5 + 0.5;
            let r = vary(150, 12, n) as f32 - ring * 35.0;
            let g = vary(107, 10, n) as f32 - ring * 25.0;
            let b = vary(66, 8, n) as f32 - ring * 15.0;
            put(&mut buf, x, y, [r.clamp(0.0, 255.0) as u8, g.clamp(0.0, 255.0) as u8, b.clamp(0.0, 255.0) as u8, 255]);
        }
    }
    buf
}

fn oak_planks(seed: u32) -> Vec<u8> {
    let mut buf = blank();
    for y in 0..SIZE {
        let joint = y % 4 == 0;
        for x in 0..SIZE {
            let n = rand01(seed, x, y);
            let grain = ((x as f32 * 2.3 + y as f32 * 0.4).sin() * 0.5 + 0.5) * 0.25;
            let mut r = vary(176, 14, n) as f32 - grain * 30.0;
            let mut g = vary(139, 12, n) as f32 - grain * 24.0;
            let mut b = vary(92, 10, n) as f32 - grain * 16.0;
            if joint {
                r -= 25.0;
                g -= 20.0;
                b -= 15.0;
            }
            put(&mut buf, x, y, [r.clamp(0.0, 255.0) as u8, g.clamp(0.0, 255.0) as u8, b.clamp(0.0, 255.0) as u8, 255]);
        }
    }
    buf
}

fn leaves(seed: u32) -> Vec<u8> {
    let mut buf = blank();
    for y in 0..SIZE {
        for x in 0..SIZE {
            let n = rand01(seed, x, y);
            let hole = rand01(seed ^ 0x5EA, x, y) < 0.22;
            let r = vary(46, 20, n);
            let g = vary(110, 28, n);
            let b = vary(34, 16, n);
            let a = if hole { 0 } else { 255 };
            put(&mut buf, x, y, [r, g, b, a]);
        }
    }
    buf
}

fn water(seed: u32) -> Vec<u8> {
    let mut buf = blank();
    for y in 0..SIZE {
        for x in 0..SIZE {
            let wave = ((x as f32 * 0.9 + y as f32 * 0.5).sin() * 0.5 + (y as f32 * 0.7).cos() * 0.5) * 0.5 + 0.5;
            let n = rand01(seed, x, y);
            let r = vary(28, 6, n) as f32;
            let g = (vary(94, 10, n) as f32) + wave * 24.0;
            let b = (vary(158, 12, n) as f32) + wave * 30.0;
            put(&mut buf, x, y, [r.clamp(0.0, 255.0) as u8, g.clamp(0.0, 255.0) as u8, b.clamp(0.0, 255.0) as u8, 178]);
        }
    }
    buf
}

fn glass(seed: u32) -> Vec<u8> {
    let mut buf = blank();
    for y in 0..SIZE {
        for x in 0..SIZE {
            let border = x == 0 || y == 0 || x == SIZE - 1 || y == SIZE - 1;
            let n = rand01(seed, x, y);
            if border {
                let v = vary(225, 10, n);
                put(&mut buf, x, y, [v, v, v.saturating_add(5), 235]);
            } else {
                let v = vary(220, 8, n);
                put(&mut buf, x, y, [v, v, v, 40]);
            }
        }
    }
    buf
}

fn glowstone(seed: u32) -> Vec<u8> {
    let mut buf = blank();
    for y in 0..SIZE {
        for x in 0..SIZE {
            let n = rand01(seed, x, y);
            let blotch = rand01(seed ^ 0x9AA, x / 3, y / 3);
            let bright = blotch > 0.45;
            if bright {
                let r = vary(255, 6, n);
                let g = vary(214, 12, n);
                let b = vary(120, 16, n);
                put(&mut buf, x, y, [r, g, b, 255]);
            } else {
                let r = vary(150, 10, n);
                let g = vary(120, 10, n);
                let b = vary(60, 8, n);
                put(&mut buf, x, y, [r, g, b, 255]);
            }
        }
    }
    buf
}

fn iron_block(seed: u32) -> Vec<u8> {
    let mut buf = blank();
    for y in 0..SIZE {
        for x in 0..SIZE {
            let border = x == 0 || y == 0 || x == SIZE - 1 || y == SIZE - 1 || x == SIZE / 2 || y == SIZE / 2;
            let n = rand01(seed, x, y);
            let base = if border { 205 } else { 224 };
            let v = vary(base, 10, n);
            put(&mut buf, x, y, [v, v, (v as i32 + 4).clamp(0, 255) as u8, 255]);
        }
    }
    buf
}

fn lamp_frame(seed: u32) -> Vec<u8> {
    let mut buf = blank();
    for y in 0..SIZE {
        for x in 0..SIZE {
            let border = x < 2 || y < 2 || x >= SIZE - 2 || y >= SIZE - 2;
            let n = rand01(seed, x, y);
            if border {
                let v = vary(60, 10, n);
                put(&mut buf, x, y, [v, v, v.saturating_add(4), 255]);
            } else {
                put(&mut buf, x, y, [0, 0, 0, 0]);
            }
        }
    }
    buf
}

/// Generates a 16x16 RGBA8 texture by name. Unknown names fall back to a
/// magenta/black checker so a typo is obvious instead of silently blank.
pub fn generate(name: &str, seed: u32) -> (u32, u32, Vec<u8>) {
    let buf = match name {
        "grass_top" => grass_top(seed),
        "grass_side" => grass_side(seed),
        "dirt" => dirt(seed),
        "sand" => sand(seed),
        "stone_bricks" => stone_bricks(seed),
        "oak_log_side" => oak_log_side(seed),
        "oak_log_top" => oak_log_top(seed),
        "oak_planks" => oak_planks(seed),
        "leaves" => leaves(seed),
        "water" => water(seed),
        "glass" => glass(seed),
        "glowstone" => glowstone(seed),
        "iron_block" => iron_block(seed),
        "lamp_frame" => lamp_frame(seed),
        _ => {
            let mut buf = blank();
            for y in 0..SIZE {
                for x in 0..SIZE {
                    let c = if (x + y) % 2 == 0 { [255, 0, 255, 255] } else { [0, 0, 0, 255] };
                    put(&mut buf, x, y, c);
                }
            }
            buf
        }
    };
    (SIZE, SIZE, buf)
}
