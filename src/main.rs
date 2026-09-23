mod color;
mod framebuffer;
mod ray_intersect;
mod render;
mod sphere;
mod vec3;

use raylib::prelude::TraceLogLevel;

use framebuffer::Framebuffer;
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

    let objects = [Sphere {
        center: Vec3::new(0.0, 0.0, -5.0),
        radius: 1.5,
    }];

    render(&mut framebuffer, &objects);

    while !window.window_should_close() {
        framebuffer.present(&mut window, &raylib_thread);
    }
}
