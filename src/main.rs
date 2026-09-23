mod color;
mod framebuffer;
mod material;
mod ray_intersect;
mod render;
mod sphere;
mod vec3;

use raylib::prelude::TraceLogLevel;

use color::Color;
use framebuffer::Framebuffer;
use material::Material;
use render::render;
use sphere::Sphere;
use vec3::Vec3;

const WINDOW_WIDTH: i32 = 800;
const WINDOW_HEIGHT: i32 = 600;
const TITLE: &str = "Diorama Minecraft - Raytracer";

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

    let ivory = Material::new(Color::from_rgb(230, 225, 200));
    let rubber = Material::new(Color::from_rgb(160, 30, 30));

    // La esfera lejana va primero a proposito: sin zbuffer se pintaria encima
    // de la cercana solo por el orden de la lista
    let objects = [
        Sphere {
            center: Vec3::new(1.5, 0.0, -6.0),
            radius: 1.0,
            material: rubber,
        },
        Sphere {
            center: Vec3::new(0.5, 0.0, -4.0),
            radius: 1.0,
            material: ivory,
        },
    ];

    render(&mut framebuffer, &objects);

    while !window.window_should_close() {
        framebuffer.present(&mut window, &raylib_thread);
    }
}
