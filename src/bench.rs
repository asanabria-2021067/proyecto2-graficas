use std::time::Instant;

use crate::camera::Camera;
use crate::framebuffer::Framebuffer;
use crate::math::Vec3;
use crate::render::{default_thread_count, render_frame, Tracer};

const VIEWS: [(f32, f32, f32); 3] = [(35.0, 25.0, 1.0), (120.0, 55.0, 0.75), (250.0, 15.0, 1.5)];
const FRAMES_PER_VIEW: usize = 10;

pub fn run(tracer: &dyn Tracer, width: u32, height: u32, center: Vec3, base_dist: f32) {
    let threads = default_thread_count();
    let mut fb = Framebuffer::new(width, height);
    let mut total_secs = 0.0f64;
    let mut count = 0usize;

    for &(yaw, pitch, dist_scale) in VIEWS.iter() {
        let dist = base_dist * dist_scale;
        let cam = Camera::new(center, yaw, pitch, dist, dist * 0.1, dist * 10.0, 50.0);
        for _ in 0..FRAMES_PER_VIEW {
            let t0 = Instant::now();
            render_frame(&mut fb, &cam, tracer, threads);
            total_secs += t0.elapsed().as_secs_f64();
            count += 1;
        }
    }

    let avg_ms = total_secs * 1000.0 / count as f64;
    let fps = 1000.0 / avg_ms;
    println!("--bench: {count} frames, {width}x{height}, {threads} threads");
    println!("avg: {avg_ms:.3} ms/frame  ({fps:.1} fps equivalente)");
}
