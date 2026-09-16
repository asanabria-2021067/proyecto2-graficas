use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use crate::camera::Camera;
use crate::framebuffer::{linear_to_u32, Framebuffer};
use crate::math::{Ray, Vec3};

pub const TILE: u32 = 16;

/// 4-tap regular grid, used for media-quality settled-frame supersampling.
pub const SAMPLES_2X2: [(f32, f32); 4] = [(-0.25, -0.25), (0.25, -0.25), (-0.25, 0.25), (0.25, 0.25)];

/// 9-tap rotated-grid supersampling (RGSS-style): a regular 3x3 lattice
/// rotated by atan(1/3) so samples don't line up on the pixel's horizontal/
/// vertical axes, which reduces the "staircase" look plain grid supersampling
/// leaves on near-axis-aligned edges. Used for alta-quality settled frames.
pub fn rotated_grid_3x3() -> [(f32, f32); 9] {
    let theta = (1.0f32 / 3.0).atan();
    let (s, c) = theta.sin_cos();
    let spacing = 1.0 / 3.0;
    let mut out = [(0.0f32, 0.0f32); 9];
    let mut i = 0;
    for gy in -1..=1 {
        for gx in -1..=1 {
            let bx = gx as f32 * spacing;
            let by = gy as f32 * spacing;
            out[i] = (bx * c - by * s, bx * s + by * c);
            i += 1;
        }
    }
    out
}

pub fn default_thread_count() -> usize {
    thread::available_parallelism().map(|n| n.get()).unwrap_or(4)
}

/// Anything that can turn a camera ray into a linear-space color.
/// Implementations must be safe to call concurrently from many threads.
pub trait Tracer: Sync {
    fn trace(&self, ray: Ray) -> Vec3;
}

struct Tile {
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
}

fn build_tiles(width: u32, height: u32) -> Vec<Tile> {
    let mut tiles = Vec::new();
    let mut y = 0;
    while y < height {
        let mut x = 0;
        let y1 = (y + TILE).min(height);
        while x < width {
            let x1 = (x + TILE).min(width);
            tiles.push(Tile { x0: x, y0: y, x1, y1 });
            x += TILE;
        }
        y += TILE;
    }
    tiles
}

/// Renders one frame into `fb` using all available CPU cores, splitting work
/// into small tiles pulled from a shared atomic counter so threads stay
/// balanced across cheap and expensive regions of the image. One ray per
/// pixel, through the pixel center.
pub fn render_frame(fb: &mut Framebuffer, cam: &Camera, tracer: &dyn Tracer, threads: usize) {
    render_frame_ms(fb, cam, tracer, threads, &[(0.0, 0.0)]);
}

/// Luminance-based contrast threshold above which a pixel gets extra
/// adaptive samples in `Progressive::step`'s refinement passes.
const AA_CONTRAST_THRESHOLD: f32 = 0.08;

#[inline]
fn luminance(c: Vec3) -> f32 {
    0.2126 * c.x + 0.7152 * c.y + 0.0722 * c.z
}

/// Progressive, adaptively-supersampled settled-frame render: pass 0 traces
/// every pixel once (so something sharp-ish shows up immediately instead of
/// blocking for the whole multi-sample budget), then each subsequent call to
/// `step` adds ONE more sample tap but only to pixels whose contrast with
/// their 4-neighbourhood (computed once, after pass 0) exceeds
/// `AA_CONTRAST_THRESHOLD` -- flat regions stay at 1 sample, edges and
/// busy textures keep refining. The caller drives one `step` per UI frame
/// (see `main::run_window_mode`), so input stays responsive between passes;
/// dropping the whole `Progressive` (e.g. because the camera moved) cancels
/// whatever refinement was in flight.
pub struct Progressive {
    width: u32,
    height: u32,
    accum: Vec<Vec3>,
    counts: Vec<u16>,
    mask: Option<Vec<bool>>,
    samples: Vec<(f32, f32)>,
    pub pass: usize,
}

struct SendMutPtr(*mut Vec3);
unsafe impl Sync for SendMutPtr {}
struct SendMutPtrU16(*mut u16);
unsafe impl Sync for SendMutPtrU16 {}

impl Progressive {
    pub fn new(width: u32, height: u32, samples: Vec<(f32, f32)>) -> Self {
        let n = (width * height) as usize;
        Progressive { width, height, accum: vec![Vec3::zero(); n], counts: vec![0u16; n], mask: None, samples, pass: 0 }
    }

    pub fn total_passes(&self) -> usize {
        self.samples.len()
    }

    pub fn is_done(&self) -> bool {
        self.pass >= self.samples.len()
    }

    /// Runs exactly one pass and resolves the accumulated result into `fb`.
    /// No-op if already done.
    pub fn step(&mut self, fb: &mut Framebuffer, cam: &Camera, tracer: &dyn Tracer, threads: usize) {
        if self.is_done() {
            return;
        }
        let (sx, sy) = self.samples[self.pass];
        let frame = cam.frame(self.width, self.height);
        let tiles = build_tiles(self.width, self.height);
        let cursor = AtomicUsize::new(0);
        let width = self.width;
        let accum_ptr = SendMutPtr(self.accum.as_mut_ptr());
        let counts_ptr = SendMutPtrU16(self.counts.as_mut_ptr());
        let mask_ref = self.mask.as_deref();

        thread::scope(|scope| {
            for _ in 0..threads.max(1) {
                let cursor = &cursor;
                let tiles = &tiles;
                let frame = &frame;
                let accum_ptr = &accum_ptr;
                let counts_ptr = &counts_ptr;
                scope.spawn(move || loop {
                    let idx = cursor.fetch_add(1, Ordering::Relaxed);
                    if idx >= tiles.len() {
                        break;
                    }
                    let tile = &tiles[idx];
                    for y in tile.y0..tile.y1 {
                        for x in tile.x0..tile.x1 {
                            let i = (y * width + x) as usize;
                            if let Some(m) = mask_ref {
                                if !m[i] {
                                    continue;
                                }
                            }
                            let ray = frame.ray_for_pixel(x as f32, y as f32, sx, sy);
                            let c = tracer.trace(ray);
                            unsafe {
                                let p = accum_ptr.0.add(i);
                                *p += c;
                                *counts_ptr.0.add(i) += 1;
                            }
                        }
                    }
                });
            }
        });

        if self.pass == 0 {
            self.mask = Some(self.compute_mask());
        }
        self.resolve_into(fb);
        self.pass += 1;
    }

    fn compute_mask(&self) -> Vec<bool> {
        let width = self.width;
        let height = self.height;
        let lum_at = |x: i32, y: i32| -> f32 {
            let i = (y as u32 * width + x as u32) as usize;
            let n = self.counts[i].max(1) as f32;
            luminance(self.accum[i] * (1.0 / n))
        };
        let mut mask = vec![false; (width * height) as usize];
        for y in 0..height as i32 {
            for x in 0..width as i32 {
                let l0 = lum_at(x, y);
                let mut maxdiff = 0.0f32;
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                        continue;
                    }
                    maxdiff = maxdiff.max((l0 - lum_at(nx, ny)).abs());
                }
                mask[(y as u32 * width + x as u32) as usize] = maxdiff > AA_CONTRAST_THRESHOLD;
            }
        }
        mask
    }

    fn resolve_into(&self, fb: &mut Framebuffer) {
        for i in 0..self.accum.len() {
            let n = self.counts[i].max(1) as f32;
            fb.pixels[i] = linear_to_u32(self.accum[i] * (1.0 / n));
        }
    }
}

/// Same as `render_frame`, but traces `samples.len()` rays per pixel (each
/// offset by a sub-pixel `(sx, sy)` tap) and averages them. Used for the
/// settled-frame supersampling pass; `samples` of length 1 is exactly
/// `render_frame`.
pub fn render_frame_ms(fb: &mut Framebuffer, cam: &Camera, tracer: &dyn Tracer, threads: usize, samples: &[(f32, f32)]) {
    let width = fb.width;
    let height = fb.height;
    let frame = cam.frame(width, height);
    let tiles = build_tiles(width, height);
    let cursor = AtomicUsize::new(0);
    let pixels = &fb.pixels;
    let raw_ptr = pixels.as_ptr() as *mut u32;
    // Safety: each tile owns a disjoint rectangle of pixels, so concurrent
    // writes from different threads never alias the same index.
    struct SendPtr(*mut u32);
    unsafe impl Sync for SendPtr {}
    let out = SendPtr(raw_ptr);
    let inv_n = 1.0 / samples.len() as f32;

    thread::scope(|scope| {
        for _ in 0..threads.max(1) {
            let cursor = &cursor;
            let tiles = &tiles;
            let out_ref = &out;
            let frame = &frame;
            scope.spawn(move || loop {
                let idx = cursor.fetch_add(1, Ordering::Relaxed);
                if idx >= tiles.len() {
                    break;
                }
                let tile = &tiles[idx];
                for y in tile.y0..tile.y1 {
                    for x in tile.x0..tile.x1 {
                        let mut color = Vec3::zero();
                        for &(sx, sy) in samples {
                            let ray = frame.ray_for_pixel(x as f32, y as f32, sx, sy);
                            color += tracer.trace(ray);
                        }
                        let packed = linear_to_u32(color * inv_n);
                        unsafe {
                            *out_ref.0.add((y * width + x) as usize) = packed;
                        }
                    }
                }
            });
        }
    });
}
