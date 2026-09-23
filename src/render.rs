use crate::color::Color;
use crate::framebuffer::Framebuffer;
use crate::light::Light;
use crate::ray_intersect::{Intersect, RayIntersect};
use crate::vec3::Vec3;

const SKY_HORIZON: Color = Color::from_rgb(200, 225, 255);
const SKY_ZENITH: Color = Color::from_rgb(60, 120, 215);

// Luz minima que llega a todos lados, para que lo que no ve la luz no quede negro
const AMBIENT: f32 = 0.15;

// Cuanto se despega el origen del shadow ray de la superficie, para que no
// choque con la misma cara de la que sale ("shadow acne")
const SHADOW_BIAS: f32 = 1e-3;

// Ancho del contorno de cada bloque: 1 texel de una textura de 16x16
const BLOCK_EDGE: f32 = 1.0 / 16.0;
const BLOCK_EDGE_SHADE: f32 = 0.7;

// Color de la superficie en el punto: el del material con el borde de la cara
// oscurecido, para que se note donde termina un bloque y empieza el otro.
// En la fase 10 esto se reemplaza por la textura del bloque
fn surface_color(intersect: &Intersect) -> Color {
    let (u, v) = intersect.uv;
    let on_edge = u < BLOCK_EDGE || u > 1.0 - BLOCK_EDGE || v < BLOCK_EDGE || v > 1.0 - BLOCK_EDGE;

    if on_edge {
        intersect.material.diffuse * BLOCK_EDGE_SHADE
    } else {
        intersect.material.diffuse
    }
}

fn sky_color(direction: &Vec3) -> Color {
    let t = 0.5 * (direction.y + 1.0);
    SKY_HORIZON.lerp(&SKY_ZENITH, t)
}

// Un punto esta en sombra si algo se atraviesa entre el y la luz
fn in_shadow(
    intersect: &Intersect,
    light_direction: &Vec3,
    light_distance: f32,
    objects: &[Box<dyn RayIntersect>],
) -> bool {
    let shadow_origin = intersect.point + intersect.normal * SHADOW_BIAS;

    objects.iter().any(|object| {
        let hit = object.ray_intersect(&shadow_origin, light_direction);
        // Lo que esta detras de la luz no le hace sombra al punto
        hit.is_intersecting && hit.distance < light_distance
    })
}

// Difusa (Lambert) + especular (Phong) de una luz en el punto de impacto
fn shade(
    intersect: &Intersect,
    surface: &Color,
    view_direction: &Vec3,
    light: &Light,
    objects: &[Box<dyn RayIntersect>],
) -> Color {
    let material = &intersect.material;
    let to_light = light.position - intersect.point;
    let light_distance = to_light.length();
    let light_direction = to_light / light_distance;

    // En sombra solo queda la luz ambiente
    if in_shadow(intersect, &light_direction, light_distance, objects) {
        return Color::default();
    }
    let light_color = light.color * light.intensity;

    // Lambert: la cara recibe mas luz mientras mas de frente la mire
    let diffuse_intensity = intersect.normal.dot(&light_direction).max(0.0);
    let diffuse = *surface * light_color * (diffuse_intensity * material.albedo[0]);

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

    let surface = surface_color(&intersect);

    let mut color = surface * AMBIENT;
    for light in lights {
        color = color + shade(&intersect, &surface, &view_direction, light, objects);
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
