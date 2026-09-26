mod camera;
mod color;
mod cube;
mod framebuffer;
mod light;
mod noise;
mod material;
mod ray_intersect;
mod render;
mod terrain;
mod texture;
mod vec3;
mod world;

use std::thread;
use std::time::Instant;

use raylib::prelude::{KeyboardKey, RaylibHandle, TraceLogLevel};

use camera::Camera;

use color::Color;
use framebuffer::Framebuffer;
use light::Light;
use ray_intersect::RayIntersect;
use render::{RenderSettings, render};
use vec3::Vec3;

const WINDOW_WIDTH: i32 = 800;
const WINDOW_HEIGHT: i32 = 600;
const TITLE: &str = "Diorama Minecraft - Raytracer";

// Velocidades por segundo, para que no dependan de los FPS
const ROTATION_SPEED: f32 = 1.5;
const ZOOM_SPEED: f32 = 8.0;

// Mientras la camara se mueve se lanza un rayo por cada bloque de 2x2 pixeles
// (4 veces menos rayos); al soltar las teclas se traza a resolucion completa
const MOVING_SCALE: usize = 2;

// Semilla de la isla al abrir el programa
const INITIAL_SEED: u64 = 2026;

// Todo el mundo es un solo objeto: el DDA ya sabe donde esta cada bloque
fn build_scene(seed: u64) -> Vec<Box<dyn RayIntersect>> {
    println!("Isla generada con la semilla {seed}");
    vec![Box::new(terrain::generate(seed))]
}

// Mueve la camara segun las teclas presionadas. Devuelve si cambio algo
fn handle_input(window: &RaylibHandle, camera: &mut Camera) -> bool {
    let delta_time = window.get_frame_time();
    let key = |key: KeyboardKey| window.is_key_down(key);

    let mut delta_yaw = 0.0;
    let mut delta_pitch = 0.0;
    let mut delta_zoom = 0.0;

    if key(KeyboardKey::KEY_LEFT) {
        delta_yaw -= ROTATION_SPEED * delta_time;
    }
    if key(KeyboardKey::KEY_RIGHT) {
        delta_yaw += ROTATION_SPEED * delta_time;
    }
    if key(KeyboardKey::KEY_UP) {
        delta_pitch += ROTATION_SPEED * delta_time;
    }
    if key(KeyboardKey::KEY_DOWN) {
        delta_pitch -= ROTATION_SPEED * delta_time;
    }
    if key(KeyboardKey::KEY_W) {
        delta_zoom -= ZOOM_SPEED * delta_time;
    }
    if key(KeyboardKey::KEY_S) {
        delta_zoom += ZOOM_SPEED * delta_time;
    }

    if delta_yaw == 0.0 && delta_pitch == 0.0 && delta_zoom == 0.0 {
        return false;
    }

    camera.orbit(delta_yaw, delta_pitch);
    camera.zoom(delta_zoom);
    true
}

fn main() {
    let (mut window, raylib_thread) = raylib::init()
        .size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .title(TITLE)
        .log_level(TraceLogLevel::LOG_WARNING)
        .build();

    window.set_target_fps(60);

    let mut framebuffer = Framebuffer::new(
        &mut window,
        &raylib_thread,
        WINDOW_WIDTH as usize,
        WINDOW_HEIGHT as usize,
    );

    // La isla se genera a partir de una semilla; con G se genera otra
    let mut seed = INITIAL_SEED;
    let mut objects = build_scene(seed);

    // El sol: lejos, arriba y a un lado de la isla, para que las sombras se vean largas
    let lights = [Light::new(
        Vec3::new(-40.0, 60.0, 30.0),
        Color::new(1.0, 1.0, 1.0),
        1.0,
    )];

    let mut camera = Camera {
        // A media altura entre la parte de abajo de la isla y la copa del arbol
        center: Vec3::new(0.0, 4.0, 0.0),
        yaw: 0.6,
        pitch: 0.35,
        radius: 48.0,
        // Suficiente para no meterse dentro de la isla ni de la copa
        min_radius: 22.0,
        max_radius: 90.0,
        fov: 60.0_f32.to_radians(),
    };

    let all_threads = thread::available_parallelism().map_or(1, |count| count.get());
    let mut use_all_threads = true;
    // Al abrir la ventana todavia no hay nada trazado
    let mut needs_full_render = true;

    while !window.window_should_close() {
        // G genera otra isla con la siguiente semilla
        if window.is_key_pressed(KeyboardKey::KEY_G) {
            seed += 1;
            objects = build_scene(seed);
            needs_full_render = true;
        }

        // T alterna entre 1 hilo y todos, para comparar tiempos
        if window.is_key_pressed(KeyboardKey::KEY_T) {
            use_all_threads = !use_all_threads;
            needs_full_render = true;
        }

        let moved = handle_input(&window, &mut camera);

        // Solo se vuelve a trazar si algo cambio
        let scale = if moved {
            needs_full_render = true;
            Some(MOVING_SCALE)
        } else if needs_full_render {
            needs_full_render = false;
            Some(1)
        } else {
            None
        };

        if let Some(scale) = scale {
            let settings = RenderSettings {
                threads: if use_all_threads { all_threads } else { 1 },
                scale,
            };

            let (width, height) = (framebuffer.width, framebuffer.height);
            let start = Instant::now();
            render(
                framebuffer.pixels_mut(),
                width,
                height,
                &objects,
                &lights,
                &camera,
                &settings,
            );
            let elapsed_ms = start.elapsed().as_secs_f32() * 1000.0;

            // Solo el render completo se reporta en la terminal, para comparar hilos
            if scale == 1 {
                println!("Render completo: {} hilo(s), {elapsed_ms:.1} ms", settings.threads);
            }
        }

        framebuffer.present(&mut window, &raylib_thread);
    }
}
