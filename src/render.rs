use crate::color::Color;
use crate::framebuffer::Framebuffer;
use crate::light::Light;
use crate::ray_intersect::{Intersect, RayIntersect};
use crate::vec3::Vec3;

const SKY_HORIZON: Color = Color::from_rgb(200, 225, 255);
const SKY_ZENITH: Color = Color::from_rgb(60, 120, 215);

// Luz minima que llega a todos lados, para que lo que no ve la luz no quede negro
const AMBIENT: f32 = 0.15;

fn sky_color(direction: &Vec3) -> Color {
    let t = 0.5 * (direction.y + 1.0);
    SKY_HORIZON.lerp(&SKY_ZENITH, t)
}

// Difusa (Lambert) + especular (Phong) de una luz en el punto de impacto
fn shade(intersect: &Intersect, view_direction: &Vec3, light: &Light) -> Color {
    let material = &intersect.material;
    let light_direction = (light.position - intersect.point).normalize();
    let light_color = light.color * light.intensity;

    // Lambert: la cara recibe mas luz mientras mas de frente la mire
    let diffuse_intensity = intersect.normal.dot(&light_direction).max(0.0);
    let diffuse = material.diffuse * light_color * (diffuse_intensity * material.albedo[0]);

    // Phong: que tan alineado esta el reflejo de la luz con el ojo
    let reflect_direction = (-light_direction).reflect(&intersect.normal);
    let specular_intensity = reflect_direction
        .dot(view_direction)
        .max(0.0)
        .powf(material.specular);
    let specular = light_color * (specular_intensity * material.albedo[1]);

    diffuse + specular
}

pub fn cast_ray(
    ray_origin: &Vec3,
    ray_direction: &Vec3,
    objects: &[Box<dyn RayIntersect>],
    lights: &[Light],
) -> Color {
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

    // Del punto de impacto hacia la camara
    let view_direction = -*ray_direction;

    let mut color = intersect.material.diffuse * AMBIENT;
    for light in lights {
        color = color + shade(&intersect, &view_direction, light);
    }
    color
}

pub fn render(framebuffer: &mut Framebuffer, objects: &[Box<dyn RayIntersect>], lights: &[Light]) {
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

            let pixel_color = cast_ray(&camera_position, &ray_direction, objects, lights);
            framebuffer.set_pixel(x, y, pixel_color.to_bytes());
        }
    }
}
