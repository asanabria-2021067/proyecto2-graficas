mod bench;
mod camera;
mod cli;
mod framebuffer;
mod image_io;
mod math;
mod render;

use raylib::prelude::*;

use camera::Camera;
use cli::Args;
use framebuffer::Framebuffer;
use math::{Ray, Vec3};
use render::{default_thread_count, render_frame, Tracer};

const INTERNAL_W: u32 = 480;
const INTERNAL_H: u32 = 270;

/// Temporary placeholder scene: gradient sky + a single test cube.
/// Replaced in fase 2 by the voxel world + DDA intersector.
struct DemoScene {
    night: bool,
}

impl Tracer for DemoScene {
    fn trace(&self, ray: Ray) -> Vec3 {
        match intersect_test_cube(ray) {
            Some((_t, normal)) => shade_face(normal, self.night),
            None => sky_color(ray.dir, self.night),
        }
    }
}

fn intersect_test_cube(ray: Ray) -> Option<(f32, Vec3)> {
    let bmin = Vec3::new(-1.0, 0.0, -1.0);
    let bmax = Vec3::new(1.0, 2.0, 1.0);
    let o = [ray.origin.x, ray.origin.y, ray.origin.z];
    let d = [ray.dir.x, ray.dir.y, ray.dir.z];
    let lo = [bmin.x, bmin.y, bmin.z];
    let hi = [bmax.x, bmax.y, bmax.z];

    let mut tmin = 0.0001f32;
    let mut tmax = f32::INFINITY;
    let mut hit_axis = 0usize;
    let mut hit_sign = -1.0f32;

    for axis in 0..3 {
        let inv_d = 1.0 / d[axis];
        let mut t0 = (lo[axis] - o[axis]) * inv_d;
        let mut t1 = (hi[axis] - o[axis]) * inv_d;
        let sign = if inv_d < 0.0 { 1.0 } else { -1.0 };
        if inv_d < 0.0 {
            std::mem::swap(&mut t0, &mut t1);
        }
        if t0 > tmin {
            tmin = t0;
            hit_axis = axis;
            hit_sign = sign;
        }
        if t1 < tmax {
            tmax = t1;
        }
        if tmin > tmax {
            return None;
        }
    }

    let mut normal = Vec3::zero();
    match hit_axis {
        0 => normal.x = hit_sign,
        1 => normal.y = hit_sign,
        _ => normal.z = hit_sign,
    }
    Some((tmin, normal))
}

fn sky_color(dir: Vec3, night: bool) -> Vec3 {
    let t = (dir.y * 0.5 + 0.5).clamp(0.0, 1.0);
    if night {
        Vec3::new(0.02, 0.02, 0.05).lerp(Vec3::new(0.05, 0.06, 0.16), t)
    } else {
        Vec3::new(0.75, 0.85, 0.95).lerp(Vec3::new(0.15, 0.35, 0.75), t)
    }
}

fn shade_face(normal: Vec3, night: bool) -> Vec3 {
    let light_dir = Vec3::new(0.4, 0.8, 0.3).normalize();
    let ndotl = normal.dot(light_dir).max(0.05);
    let base = if normal.x > 0.5 {
        Vec3::new(0.85, 0.25, 0.25)
    } else if normal.x < -0.5 {
        Vec3::new(0.25, 0.55, 0.85)
    } else if normal.y > 0.5 {
        Vec3::new(0.25, 0.85, 0.25)
    } else if normal.y < -0.5 {
        Vec3::new(0.85, 0.85, 0.25)
    } else if normal.z > 0.5 {
        Vec3::new(0.85, 0.25, 0.85)
    } else {
        Vec3::new(0.25, 0.85, 0.85)
    };
    let ambient = if night { 0.12 } else { 0.22 };
    base * (ambient + (1.0 - ambient) * ndotl)
}

fn build_camera(args: &Args) -> Camera {
    Camera::new(Vec3::new(0.0, 1.0, 0.0), args.yaw, args.pitch, args.dist, 5.0, 300.0, 50.0)
}

fn run_render_mode(args: &Args, path: &str) {
    let scene = DemoScene { night: args.night };
    let cam = build_camera(args);
    let mut fb = Framebuffer::new(args.width, args.height);
    render_frame(&mut fb, &cam, &scene, default_thread_count());
    image_io::write_framebuffer_png(path, &fb).expect("no se pudo escribir el PNG");
    println!("render escrito en {path} ({}x{})", args.width, args.height);
}

fn run_bench_mode(args: &Args) {
    let scene = DemoScene { night: args.night };
    bench::run(&scene, args.width, args.height);
}

#[derive(PartialEq, Clone, Copy)]
struct FrameState {
    yaw_bits: u32,
    pitch_bits: u32,
    dist_bits: u32,
    night: bool,
    normalmaps: bool,
    quality: u8,
    seed: u32,
}

fn run_window_mode(args: &Args) {
    let (mut rl, thread) = raylib::init()
        .size(args.width as i32, args.height as i32)
        .title("Diorama Raytracer - Proyecto 2 Graficas")
        .build();
    rl.set_target_fps(60);

    let mut cam = build_camera(args);
    let mut night = args.night;
    let mut normalmaps = !args.no_normalmaps;
    let mut quality: u8 = 2;
    let mut seed = args.seed;
    let mut auto_rotate = false;

    let mut fb = Framebuffer::new(INTERNAL_W, INTERNAL_H);
    let mut rgba = vec![0u8; (INTERNAL_W * INTERNAL_H * 4) as usize];

    let image = Image::gen_image_color(INTERNAL_W as i32, INTERNAL_H as i32, Color::BLACK);
    let mut texture = rl
        .load_texture_from_image(&thread, &image)
        .expect("no se pudo crear la textura del framebuffer");

    let mut last_state: Option<FrameState> = None;

    while !rl.window_should_close() {
        let dt = rl.get_frame_time();
        let rot_speed = 1.4_f32; // rad/s
        let zoom_speed = 30.0_f32; // units/s

        if rl.is_key_down(KeyboardKey::KEY_A) || rl.is_key_down(KeyboardKey::KEY_LEFT) {
            cam.orbit(-rot_speed * dt, 0.0);
        }
        if rl.is_key_down(KeyboardKey::KEY_D) || rl.is_key_down(KeyboardKey::KEY_RIGHT) {
            cam.orbit(rot_speed * dt, 0.0);
        }
        if rl.is_key_down(KeyboardKey::KEY_W) || rl.is_key_down(KeyboardKey::KEY_UP) {
            cam.orbit(0.0, rot_speed * dt);
        }
        if rl.is_key_down(KeyboardKey::KEY_S) || rl.is_key_down(KeyboardKey::KEY_DOWN) {
            cam.orbit(0.0, -rot_speed * dt);
        }
        if rl.is_key_down(KeyboardKey::KEY_Q) {
            cam.zoom(-zoom_speed * dt);
        }
        if rl.is_key_down(KeyboardKey::KEY_E) {
            cam.zoom(zoom_speed * dt);
        }
        let wheel = rl.get_mouse_wheel_move();
        if wheel != 0.0 {
            cam.zoom(-wheel * 4.0);
        }
        if rl.is_key_pressed(KeyboardKey::KEY_R) {
            auto_rotate = !auto_rotate;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_T) {
            night = !night;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_N) {
            normalmaps = !normalmaps;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_G) {
            seed = seed.wrapping_add(1);
        }
        if rl.is_key_pressed(KeyboardKey::KEY_ONE) {
            quality = 1;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_TWO) {
            quality = 2;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_THREE) {
            quality = 3;
        }
        if auto_rotate {
            cam.orbit(0.5 * dt, 0.0);
        }

        let state = FrameState {
            yaw_bits: cam.yaw.to_bits(),
            pitch_bits: cam.pitch.to_bits(),
            dist_bits: cam.dist.to_bits(),
            night,
            normalmaps,
            quality,
            seed,
        };
        let dirty = last_state != Some(state);

        if dirty {
            let scene = DemoScene { night };
            let t0 = std::time::Instant::now();
            render_frame(&mut fb, &cam, &scene, default_thread_count());
            let last_ms = t0.elapsed().as_secs_f64() * 1000.0;

            let fps_est = if last_ms > 0.0 { 1000.0 / last_ms } else { 0.0 };
            let quality_name = match quality {
                1 => "BAJA",
                3 => "ALTA",
                _ => "MEDIA",
            };
            let mode = if night { "NOCHE" } else { "DIA" };
            let nm = if normalmaps { "ON" } else { "OFF" };
            fb.draw_text(4, 4, &format!("FPS:{fps_est:.0} MS:{last_ms:.1}"), 0x00FFFFFF, 1);
            fb.draw_text(4, 12, &format!("RES:{INTERNAL_W}X{INTERNAL_H}"), 0x00FFFFFF, 1);
            fb.draw_text(4, 20, &format!("SEED:{seed} {mode} Q:{quality_name}"), 0x00FFFFFF, 1);
            fb.draw_text(4, 28, &format!("NORMALMAPS:{nm}"), 0x00FFFFFF, 1);

            rgba.clear();
            for &p in &fb.pixels {
                let (r, g, b) = framebuffer::u32_to_rgb(p);
                rgba.push(r);
                rgba.push(g);
                rgba.push(b);
                rgba.push(255);
            }
            texture.update_texture(&rgba).expect("no se pudo actualizar la textura");
            last_state = Some(state);
        }

        let scale_x = args.width as f32 / INTERNAL_W as f32;
        let scale_y = args.height as f32 / INTERNAL_H as f32;
        let scale = scale_x.min(scale_y);

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::BLACK);
        d.draw_texture_ex(&texture, Vector2::new(0.0, 0.0), 0.0, scale, Color::WHITE);
    }
}

fn main() {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let args = cli::parse(&raw);

    if let Some(path) = args.render.clone() {
        run_render_mode(&args, &path);
        return;
    }
    if args.bench {
        run_bench_mode(&args);
        return;
    }
    run_window_mode(&args);
}
