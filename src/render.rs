use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use crate::camera::Camera;
use crate::framebuffer::{linear_to_u32, Framebuffer};
use crate::math::{Ray, Vec3};

pub const TILE: u32 = 16;

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
/// balanced across cheap and expensive regions of the image.
pub fn render_frame(fb: &mut Framebuffer, cam: &Camera, tracer: &dyn Tracer, threads: usize) {
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
                        let ray = frame.ray_for_pixel(x as f32, y as f32);
                        let color = tracer.trace(ray);
                        let packed = linear_to_u32(color);
                        unsafe {
                            *out_ref.0.add((y * width + x) as usize) = packed;
                        }
                    }
                }
            });
        }
    });
}
