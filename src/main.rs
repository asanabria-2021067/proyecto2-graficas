mod bench;
mod camera;
mod cli;
mod framebuffer;
mod image_io;
mod intersect;
mod lights;
mod material;
mod math;
mod noise;
mod render;
mod scene;
mod shading;
mod skybox;
mod terrain;
mod texgen;
mod texture;
mod world;

use raylib::prelude::*;

use camera::Camera;
use cli::Args;
use framebuffer::Framebuffer;
use lights::{build_light_grid, LightGrid};
use material::MaterialTable;
use math::Vec3;
use render::{default_thread_count, render_frame};
use scene::{max_depth_for_quality, Scene};
use shading::{day_environment, night_environment, Environment};
use skybox::Skybox;
use terrain::{generate_island, Heightmap, IslandParams};
use world::World;

const INTERNAL_W: u32 = 480;
const INTERNAL_H: u32 = 270;
const WORLD_NX: i32 = 100;
const WORLD_NY: i32 = 56;
const WORLD_NZ: i32 = 100;

/// Fase 8 test scene: isla flotante procedural (heightmap fBm + mascara radial
/// deformada + base conica irregular + arboles). La escena final del faro
/// (fase 9) se construye encima de este terreno.
#[allow(dead_code)] // heightmap se usa en fase 9 para ubicar estructuras
struct WorldData {
    world: World,
    materials: MaterialTable,
    lights: LightGrid,
    heightmap: Heightmap,
    island: IslandParams,
    gen_ms: f64,
}

fn build_world_data(seed: u32) -> WorldData {
    let t0 = std::time::Instant::now();
    let mut world = World::new(WORLD_NX, WORLD_NY, WORLD_NZ);
    let island = IslandParams::main_island(&world);
    let heightmap = generate_island(&mut world, seed, &island);
    let materials = material::build_material_table(seed);
    let lights = build_light_grid(&world, &materials, 8.0);
    let gen_ms = t0.elapsed().as_secs_f64() * 1000.0;
    WorldData { world, materials, lights, heightmap, island, gen_ms }
}

fn build_camera(args: &Args, wd: &WorldData) -> Camera {
    let center = Vec3::new(wd.island.center_x as f32, wd.island.top_y as f32, wd.island.center_z as f32);
    let radius = wd.island.radius;
    Camera::new(center, args.yaw, args.pitch, args.dist, radius * 0.6, radius * 8.0, 50.0)
}

fn environment_for(night: bool) -> Environment {
    if night {
        night_environment()
    } else {
        day_environment()
    }
}

fn build_skybox(seed: u32) -> Skybox {
    Skybox::build(seed, day_environment().sun.dir, night_environment().sun.dir)
}

fn run_render_mode(args: &Args, path: &str) {
    let wd = build_world_data(args.seed);
    println!("terreno generado en {:.2} ms (semilla {})", wd.gen_ms, args.seed);
    let skybox = build_skybox(args.seed);
    let scene = Scene {
        world: &wd.world,
        materials: &wd.materials,
        lights: &wd.lights,
        skybox: &skybox,
        env: environment_for(args.night),
        night: args.night,
        max_depth: max_depth_for_quality(3),
        normalmaps: !args.no_normalmaps,
    };
    let cam = build_camera(args, &wd);
    let mut fb = Framebuffer::new(args.width, args.height);
    render_frame(&mut fb, &cam, &scene, default_thread_count());
    image_io::write_framebuffer_png(path, &fb).expect("no se pudo escribir el PNG");
    println!("render escrito en {path} ({}x{})", args.width, args.height);
}

fn run_bench_mode(args: &Args) {
    let wd = build_world_data(args.seed);
    let skybox = build_skybox(args.seed);
    let scene = Scene {
        world: &wd.world,
        materials: &wd.materials,
        lights: &wd.lights,
        skybox: &skybox,
        env: environment_for(args.night),
        night: args.night,
        max_depth: max_depth_for_quality(2),
        normalmaps: !args.no_normalmaps,
    };
    let center = Vec3::new(wd.island.center_x as f32, wd.island.top_y as f32, wd.island.center_z as f32);
    bench::run(&scene, args.width, args.height, center, wd.island.radius * 2.0);
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

    let mut wd = build_world_data(args.seed);
    let mut gen_ms = wd.gen_ms;
    let mut cam = build_camera(args, &wd);
    let skybox = build_skybox(args.seed);
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
            wd = build_world_data(seed);
            gen_ms = wd.gen_ms;
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
            let scene = Scene {
                world: &wd.world,
                materials: &wd.materials,
                lights: &wd.lights,
                skybox: &skybox,
                env: environment_for(night),
                night,
                max_depth: max_depth_for_quality(quality),
                normalmaps,
            };
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
            fb.draw_text(4, 28, &format!("NORMALMAPS:{nm} GEN:{gen_ms:.1}MS"), 0x00FFFFFF, 1);

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
