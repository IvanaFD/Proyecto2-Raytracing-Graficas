use crate::color::Color;
use crate::framebuffer::Framebuffer;
use crate::ray_intersect::{Intersect, RayIntersect};
use crate::vec3::Vec3;

const SKY_HORIZON: Color = Color::from_rgb(200, 225, 255);
const SKY_ZENITH: Color = Color::from_rgb(60, 120, 215);

fn sky_color(direction: &Vec3) -> Color {
    let t = 0.5 * (direction.y + 1.0);
    SKY_HORIZON.lerp(&SKY_ZENITH, t)
}

// Sombreado fijo por cara como el Minecraft clasico (arriba 1.0, lados 0.8
// y 0.7, abajo 0.6) para distinguir las caras antes de tener luces.
// Se reemplaza por la iluminacion de verdad en la fase 6
fn face_shade(normal: &Vec3) -> f32 {
    0.8 + 0.2 * normal.y - 0.1 * normal.x.abs()
}

pub fn cast_ray(ray_origin: &Vec3, ray_direction: &Vec3, objects: &[Box<dyn RayIntersect>]) -> Color {
    let mut intersect = Intersect::empty();
    // Distancia del impacto mas cercano encontrado hasta ahora
    let mut zbuffer = f32::INFINITY;

    for object in objects {
        let hit = object.ray_intersect(ray_origin, ray_direction);
        if hit.is_intersecting && hit.distance < zbuffer {
            zbuffer = hit.distance;
            intersect = hit;
        }
    }

    if !intersect.is_intersecting {
        return sky_color(ray_direction);
    }

    intersect.material.diffuse * face_shade(&intersect.normal)
}

pub fn render(framebuffer: &mut Framebuffer, objects: &[Box<dyn RayIntersect>]) {
    let width = framebuffer.width as f32;
    let height = framebuffer.height as f32;
    let aspect_ratio = width / height;
    let camera_position = Vec3::new(0.0, 0.0, 0.0);

    for y in 0..framebuffer.height {
        for x in 0..framebuffer.width {
            let screen_x = (2.0 * x as f32) / width - 1.0;
            let screen_y = -(2.0 * y as f32) / height + 1.0;
            let screen_x = screen_x * aspect_ratio;

            let ray_direction = Vec3::new(screen_x, screen_y, -1.0).normalize();

            let pixel_color = cast_ray(&camera_position, &ray_direction, objects);
            framebuffer.set_pixel(x, y, pixel_color.to_bytes());
        }
    }
}
