mod camera;
mod color;
mod cube;
mod framebuffer;
mod light;
mod noise;
mod material;
mod ray_intersect;
mod render;
mod skybox;
mod structures;
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
use render::{RenderSettings, Scene, render};
use skybox::Skybox;
use vec3::Vec3;

const WINDOW_WIDTH: i32 = 800;
const WINDOW_HEIGHT: i32 = 600;
const TITLE: &str = "Diorama Minecraft - Raytracer";

// Velocidades por segundo, para que no dependan de los FPS
const ROTATION_SPEED: f32 = 1.5;
// Fraccion de la distancia actual que se acerca o aleja por segundo
const ZOOM_SPEED: f32 = 0.8;

// Mientras la camara se mueve se lanza un rayo por cada bloque de 2x2 pixeles
// (4 veces menos rayos); al soltar las teclas se traza a resolucion completa
const MOVING_SCALE: usize = 2;

// Semilla del archipielago al abrir el programa
const INITIAL_SEED: u64 = 2026;

// Direccion hacia el sol: bajo en el horizonte, como al atardecer. La usan el
// skybox (donde se dibuja el sol) y la luz (de donde vienen las sombras)
const SUN_DIRECTION: Vec3 = Vec3::new(-0.83, 0.3, 0.56);
// De noche la luna sale del otro lado, mas alta
const MOON_DIRECTION: Vec3 = Vec3::new(0.55, 0.5, -0.65);
const SKY_LIGHT_DISTANCE: f32 = 300.0;

// Luz ambiente: al atardecer la penumbra es morada; de noche casi negra, para que
// resalten el portal, el magma y los faroles
const SUNSET_AMBIENT: Color = Color::new(0.3, 0.22, 0.34);
const NIGHT_AMBIENT: Color = Color::new(0.06, 0.07, 0.13);

// Genera el archipielago y arma la lista de objetos y de luces de la escena. Todo el
// mundo es un solo objeto: el DDA ya sabe donde esta cada bloque
fn build_island(seed: u64, sky_light: Light) -> (Vec<Box<dyn RayIntersect>>, Vec<Light>) {
    println!("Archipielago generado con la semilla {seed}");
    let island = terrain::generate(seed);
    // La primera luz siempre es la del cielo (sol o luna)
    let mut lights = vec![sky_light];
    lights.extend(island.lights);
    (vec![Box::new(island.world)], lights)
}

// La luz del cielo: de dia el sol del atardecer (bajo y naranja, con sombras largas)
// y de noche la luna (tenue y azulada)
fn sky_light(night: bool) -> Light {
    if night {
        let position = MOON_DIRECTION.normalize() * SKY_LIGHT_DISTANCE;
        Light::sun(position, Color::new(0.55, 0.62, 0.9), 0.35)
    } else {
        let position = SUN_DIRECTION.normalize() * SKY_LIGHT_DISTANCE;
        Light::sun(position, Color::new(1.0, 0.66, 0.42), 1.25)
    }
}

// Vista general: el centro del archipielago, lo bastante lejos para ver las 5 islas
const OVERVIEW_CENTER: Vec3 = Vec3::new(0.0, 4.0, 0.0);
const OVERVIEW_RADIUS: f32 = 95.0;
// Cuanto se puede acercar la camara: en la vista general hasta la isla del portal y
// en cada isla casi hasta su centro (sin colisiones: muy cerca puede quedar adentro
// de un arbol o una casa)
const OVERVIEW_MIN_RADIUS: f32 = 12.0;
const FOCUS_MIN_RADIUS: f32 = 4.0;

// Teclas que se muestran en la barra de arriba de la ventana
const CONTROLS: [&str; 2] = [
    "Flechas: girar/inclinar   W/S: zoom   0: vista general   1-5: islas",
    "L: dia/noche   G: otras islas   N: relieve   T: 1 hilo/todos",
];
const MAX_RADIUS: f32 = 170.0;

// 0 enfoca todo el archipielago; 1 a 5 enfocan cada isla. Devuelve si cambio algo
fn handle_focus(window: &RaylibHandle, camera: &mut Camera) -> bool {
    let keys = [
        KeyboardKey::KEY_ONE,
        KeyboardKey::KEY_TWO,
        KeyboardKey::KEY_THREE,
        KeyboardKey::KEY_FOUR,
        KeyboardKey::KEY_FIVE,
    ];

    if window.is_key_pressed(KeyboardKey::KEY_ZERO) {
        camera.center = OVERVIEW_CENTER;
        camera.radius = OVERVIEW_RADIUS;
        camera.min_radius = OVERVIEW_MIN_RADIUS;
        println!("Vista general");
        return true;
    }

    for (key, focus) in keys.iter().zip(terrain::FOCUS_POINTS.iter()) {
        if window.is_key_pressed(*key) {
            camera.center = focus.center;
            camera.radius = focus.radius;
            camera.min_radius = FOCUS_MIN_RADIUS;
            println!("{}", focus.name);
            return true;
        }
    }
    false
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
    // El zoom es proporcional a la distancia: igual de comodo de cerca que de lejos
    if key(KeyboardKey::KEY_W) {
        delta_zoom -= ZOOM_SPEED * camera.radius * delta_time;
    }
    if key(KeyboardKey::KEY_S) {
        delta_zoom += ZOOM_SPEED * camera.radius * delta_time;
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

    // El archipielago se genera a partir de una semilla; con G se genera otro
    let mut seed = INITIAL_SEED;
    let mut night = false;

    let (objects, lights) = build_island(seed, sky_light(night));

    let mut scene = Scene {
        objects,
        // El sol y las luces de los bloques emisivos (lamparas y portal)
        lights,
        skybox: Skybox::sunset(SUN_DIRECTION),
        ambient: SUNSET_AMBIENT,
        normal_maps: true,
    };

    // El cielo que no se esta usando; con L se intercambia con el de la escena
    let mut other_skybox = Skybox::night(MOON_DIRECTION);

    // Arranca viendo todo el archipielago
    let mut camera = Camera {
        center: OVERVIEW_CENTER,
        // Mirando entre la isla de sakura y la del desierto, con el portal al centro
        yaw: 0.0,
        pitch: 0.42,
        radius: OVERVIEW_RADIUS,
        min_radius: OVERVIEW_MIN_RADIUS,
        max_radius: MAX_RADIUS,
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
            (scene.objects, scene.lights) = build_island(seed, sky_light(night));
            needs_full_render = true;
        }

        // L cambia entre atardecer y noche
        if window.is_key_pressed(KeyboardKey::KEY_L) {
            night = !night;
            std::mem::swap(&mut scene.skybox, &mut other_skybox);
            scene.lights[0] = sky_light(night);
            scene.ambient = if night { NIGHT_AMBIENT } else { SUNSET_AMBIENT };
            println!("{}", if night { "Noche" } else { "Atardecer" });
            needs_full_render = true;
        }

        // N prende y apaga los mapas normales, para comparar el relieve
        if window.is_key_pressed(KeyboardKey::KEY_N) {
            scene.normal_maps = !scene.normal_maps;
            let state = if scene.normal_maps { "prendidos" } else { "apagados" };
            println!("Mapas normales {state}");
            needs_full_render = true;
        }

        // T alterna entre 1 hilo y todos, para comparar tiempos
        if window.is_key_pressed(KeyboardKey::KEY_T) {
            use_all_threads = !use_all_threads;
            needs_full_render = true;
        }

        let moved = handle_focus(&window, &mut camera) | handle_input(&window, &mut camera);

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
                &scene,
                &camera,
                &settings,
            );
            let elapsed_ms = start.elapsed().as_secs_f32() * 1000.0;

            // Solo el render completo se reporta en la terminal, para comparar hilos
            if scale == 1 {
                println!("Render completo: {} hilo(s), {elapsed_ms:.1} ms", settings.threads);
            }
        }

        framebuffer.present(&mut window, &raylib_thread, &CONTROLS);
    }
}
