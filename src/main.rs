mod color;
mod cube;
mod framebuffer;
mod light;
mod material;
mod ray_intersect;
mod render;
mod sphere;
mod vec3;

use raylib::prelude::TraceLogLevel;

use color::Color;
use cube::Cube;
use framebuffer::Framebuffer;
use light::Light;
use material::Material;
use ray_intersect::RayIntersect;
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

    // Material::new(color, exponente especular, [difuso, especular, reflectividad, transparencia], IOR)
    let grass = Material::new(Color::from_rgb(95, 159, 53), 10.0, [0.9, 0.1, 0.0, 0.0], 1.0);
    let dirt = Material::new(Color::from_rgb(134, 96, 67), 5.0, [0.95, 0.05, 0.0, 0.0], 1.0);
    let stone = Material::new(Color::from_rgb(125, 125, 125), 30.0, [0.8, 0.25, 0.0, 0.0], 1.0);
    let sand = Material::new(Color::from_rgb(219, 207, 163), 10.0, [0.9, 0.1, 0.0, 0.0], 1.0);
    let wood = Material::new(Color::from_rgb(162, 130, 78), 15.0, [0.85, 0.15, 0.0, 0.0], 1.0);
    let ivory = Material::new(Color::from_rgb(230, 225, 200), 50.0, [0.6, 0.4, 0.0, 0.0], 1.0);

    // Bloques por debajo y a los lados de la camara para que se vean
    // la cara de arriba, el frente y los costados
    let objects: Vec<Box<dyn RayIntersect>> = vec![
        Box::new(Cube::block(Vec3::new(-2.5, -2.0, -6.0), grass)),
        Box::new(Cube::block(Vec3::new(-1.5, -2.0, -6.0), dirt)),
        Box::new(Cube::block(Vec3::new(-0.5, -2.0, -6.0), stone)),
        Box::new(Cube::block(Vec3::new(0.5, -2.0, -6.0), sand)),
        Box::new(Cube::block(Vec3::new(-1.5, -2.0, -7.0), stone)),
        Box::new(Cube::block(Vec3::new(-1.5, -1.0, -7.0), wood)),
        // Cubo que tapa parcialmente a otro: prueba del zbuffer entre cubos
        Box::new(Cube::block(Vec3::new(1.2, -1.6, -5.0), wood)),
        // La esfera sigue funcionando porque tambien implementa RayIntersect
        Box::new(Sphere {
            center: Vec3::new(2.3, -1.2, -7.0),
            radius: 0.8,
            material: ivory,
        }),
    ];

    // Arriba, a la izquierda y detras de la camara, para iluminar el frente y la cara de arriba
    let lights = [Light::new(
        Vec3::new(-3.0, 5.0, 3.0),
        Color::new(1.0, 1.0, 1.0),
        1.0,
    )];

    render(&mut framebuffer, &objects, &lights);

    while !window.window_should_close() {
        framebuffer.present(&mut window, &raylib_thread);
    }
}
