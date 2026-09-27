use std::sync::Mutex;
use std::thread;

use crate::camera::Camera;
use crate::color::Color;
use crate::cube::face_tangents;
use crate::framebuffer::CHANNELS;
use crate::light::Light;
use crate::ray_intersect::{Intersect, RayIntersect};
use crate::skybox::Skybox;
use crate::vec3::Vec3;

// Cuanto se despega el origen del shadow ray de la superficie, para que no
// choque con la misma cara de la que sale ("shadow acne")
const SHADOW_BIAS: f32 = 1e-3;

// Cuantas veces puede rebotar un rayo. Sin limite, dos superficies que se
// reflejan una a la otra harian una recursion infinita
const MAX_DEPTH: u32 = 3;

// Cuanto tine un material transparente la luz que lo atraviesa (0 = nada, 1 = su color)
const TRANSPARENT_TINT: f32 = 0.35;

// Mas alla de esta distancia una luz de bloque (lampara, portal) ya casi no aporta
// y se salta, para no lanzar shadow rays de mas
const POINT_LIGHT_RANGE: f32 = 14.0;

// Cuantas superficies transparentes puede cruzar un shadow ray antes de rendirse
const MAX_SHADOW_LAYERS: u32 = 4;

// Fresnel con la aproximacion de Schlick: que fraccion de la luz se refleja en una
// superficie transparente segun el angulo. De frente casi toda pasa (el agua refleja
// ~2%); de lado casi toda se refleja
fn fresnel(incident: &Vec3, normal: &Vec3, refractive_index: f32) -> f32 {
    let cos_incident = incident.dot(normal).clamp(-1.0, 1.0);
    let (n1, n2) = if cos_incident < 0.0 {
        (1.0, refractive_index)
    } else {
        (refractive_index, 1.0)
    };

    // Saliendo del material: si Snell no tiene solucion es reflexion interna total
    let sin_transmitted = n1 / n2 * (1.0 - cos_incident * cos_incident).max(0.0).sqrt();
    if sin_transmitted >= 1.0 {
        return 1.0;
    }

    let r0 = ((n1 - n2) / (n1 + n2)).powi(2);
    let cos = if n1 <= n2 {
        cos_incident.abs()
    } else {
        // Saliendo se usa el angulo del lado del aire
        (1.0 - sin_transmitted * sin_transmitted).sqrt()
    };
    r0 + (1.0 - r0) * (1.0 - cos).powi(5)
}

// Origen de un rayo secundario despegado de la superficie, del lado hacia donde va
fn offset_origin(intersect: &Intersect, direction: &Vec3) -> Vec3 {
    if direction.dot(&intersect.normal) < 0.0 {
        intersect.point - intersect.normal * SHADOW_BIAS
    } else {
        intersect.point + intersect.normal * SHADOW_BIAS
    }
}

// Filas de bloques de pixeles que toma un hilo cada vez. Bandas pequenas
// reparten mejor el trabajo: una banda con mucho cielo termina rapido y el
// hilo toma otra en lugar de quedarse esperando
const BAND_BLOCK_ROWS: usize = 4;

// Color de la superficie en el punto: el pixel de la textura de esa cara,
// o el color base si el material no tiene textura
fn surface_color(intersect: &Intersect) -> Color {
    match intersect.material.textures {
        Some(textures) => textures.sample(&intersect.normal, intersect.uv),
        None => intersect.material.diffuse,
    }
}

// Todo lo que se necesita para trazar un rayo
pub struct Scene {
    pub objects: Vec<Box<dyn RayIntersect>>,
    pub lights: Vec<Light>,
    pub skybox: Skybox,
    // Luz minima que llega a todos lados, para que lo que no ve ninguna luz no
    // quede negro. Tiene color: al atardecer la sombra se ve morada
    pub ambient: Color,
    // Si se usan los mapas normales (se apagan con N para comparar)
    pub normal_maps: bool,
}

// Mapa normal: cambia la normal pixel a pixel sin cambiar la geometria. La normal
// del mapa esta en espacio tangente y se pasa al mundo con la base TBN de la cara:
// N' = T * n.x + B * n.y + N * n.z
fn apply_normal_map(intersect: &mut Intersect) {
    let normal_map = intersect
        .material
        .textures
        .and_then(|textures| textures.normal_map.as_ref());
    let Some(normal_map) = normal_map else {
        return;
    };

    let tangent_normal = normal_map.sample(intersect.uv.0, intersect.uv.1);
    let (tangent, bitangent) = face_tangents(&intersect.normal);
    intersect.normal = (tangent * tangent_normal.x
        + bitangent * tangent_normal.y
        + intersect.normal * tangent_normal.z)
        .normalize();
}

// Cuanta luz llega del foco al punto (1 = toda, 0 = sombra completa). Un bloque
// opaco la bloquea; uno transparente (agua) deja pasar la parte transparente
fn light_visibility(
    intersect: &Intersect,
    light_direction: &Vec3,
    light_distance: f32,
    emitter_radius: f32,
    objects: &[Box<dyn RayIntersect>],
) -> f32 {
    let mut origin = intersect.point + intersect.normal * SHADOW_BIAS;
    let mut remaining = light_distance;
    let mut visibility = 1.0;

    for _ in 0..MAX_SHADOW_LAYERS {
        let mut nearest = Intersect::empty();
        for object in objects {
            let hit = object.ray_intersect(&origin, light_direction);
            if hit.is_intersecting && hit.distance < nearest.distance {
                nearest = hit;
            }
        }

        // Lo que esta detras de la luz no le hace sombra al punto, y tampoco el
        // bloque que emite la luz (lo que se choque a menos de `emitter_radius` de ella)
        if !nearest.is_intersecting || nearest.distance >= remaining - emitter_radius {
            return visibility;
        }

        let transparency = nearest.material.albedo[3];
        if transparency <= 0.0 {
            return 0.0;
        }

        // Atraviesa la superficie transparente y sigue buscando hacia la luz
        visibility *= transparency;
        remaining -= nearest.distance;
        origin = offset_origin(&nearest, light_direction);
    }
    visibility
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

    // Lambert: la cara recibe mas luz mientras mas de frente la mire
    let diffuse_intensity = intersect.normal.dot(&light_direction);

    // Sin shadow ray si la cara le da la espalda a la luz o si es una luz de bloque
    // demasiado lejana para notarse
    let too_far = light.falloff > 0.0 && light_distance > POINT_LIGHT_RANGE;
    if diffuse_intensity <= 0.0 || too_far {
        return Color::default();
    }

    // En sombra solo queda la luz ambiente
    let visibility = light_visibility(
        intersect,
        &light_direction,
        light_distance,
        light.radius,
        objects,
    );
    if visibility <= 0.0 {
        return Color::default();
    }
    let light_color = light.color * (light.intensity_at(light_distance) * visibility);

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

// `depth` cuenta los rebotes: 0 para el rayo que sale de la camara
pub fn cast_ray(ray_origin: &Vec3, ray_direction: &Vec3, scene: &Scene, depth: u32) -> Color {
    let objects = &scene.objects;
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
        return scene.skybox.sample(ray_direction);
    }

    // La normal de la cara antes del mapa normal
    let face_normal = intersect.normal;
    if scene.normal_maps {
        apply_normal_map(&mut intersect);
    }

    // Del punto de impacto hacia la camara
    let view_direction = -*ray_direction;

    let surface = surface_color(&intersect);

    // Las grietas y juntas del mapa normal miran de lado y reciben menos luz ambiente,
    // como una cavidad real. En una cara plana el factor es 1
    let cavity = intersect.normal.dot(&face_normal).max(0.0).powi(2);
    let mut color = surface * scene.ambient * cavity;
    for light in &scene.lights {
        color = color + shade(&intersect, &surface, &view_direction, light, objects);
    }

    // Emision: la luz propia del bloque se suma sin depender de ninguna luz ni sombra.
    // Se multiplica por la textura para que se siga viendo su dibujo
    color = color + surface * intersect.material.emission;

    if depth >= MAX_DEPTH {
        return color;
    }

    let material = &intersect.material;
    let reflectivity = material.albedo[2];
    let transparency = material.albedo[3];

    // Reflexion: se lanza otro rayo en la direccion del reflejo, R = D - 2(D.N)N
    let reflect_direction = ray_direction.reflect(&intersect.normal).normalize();
    let reflected = |scene: &Scene| {
        let origin = offset_origin(&intersect, &reflect_direction);
        cast_ray(&origin, &reflect_direction, scene, depth + 1)
    };

    if transparency > 0.0 {
        // Refraccion: el rayo atraviesa la superficie y se dobla (ley de Snell).
        // Fresnel reparte la luz transmitida entre reflejo y refraccion segun el angulo
        let ior = material.refractive_index;
        let kr = fresnel(ray_direction, &intersect.normal, ior);
        let reflected_color = reflected(scene);

        let refracted_color = match ray_direction.refract(&intersect.normal, ior) {
            Some(refract_direction) => {
                let refract_direction = refract_direction.normalize();
                let origin = offset_origin(&intersect, &refract_direction);
                let behind = cast_ray(&origin, &refract_direction, scene, depth + 1);
                // La luz que atraviesa el agua se tine un poco de su color
                behind * Color::new(1.0, 1.0, 1.0).lerp(&surface, TRANSPARENT_TINT)
            }
            // Reflexion interna total: todo se refleja
            None => reflected_color,
        };

        // Pesos: reflejo fijo del material + la parte de Fresnel, luz que pasa, y
        // lo que queda para el color propio de la superficie
        let reflect_weight = (reflectivity + transparency * kr).min(1.0);
        let refract_weight = (transparency * (1.0 - kr)).min(1.0 - reflect_weight);
        let local_weight = 1.0 - reflect_weight - refract_weight;

        color = color * local_weight + reflected_color * reflect_weight + refracted_color * refract_weight;
    } else if reflectivity > 0.0 {
        // Superficie opaca que refleja: mezcla su color con el del reflejo
        color = color * (1.0 - reflectivity) + reflected(scene) * reflectivity;
    }

    color
}

// Como trazar un frame: cuantos hilos usar y cada cuantos pixeles lanzar un rayo.
// Con scale = 2 se lanza un rayo por cada bloque de 2x2 pixeles (4 veces menos rayos)
pub struct RenderSettings {
    pub threads: usize,
    pub scale: usize,
}

// Traza la escena sobre `pixels` (RGBA, fila por fila, width x height)
pub fn render(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    scene: &Scene,
    camera: &Camera,
    settings: &RenderSettings,
) {
    let aspect_ratio = width as f32 / height as f32;
    // Con el fov se decide que tan abierta es la vista: a 90 grados la escala es 1
    let fov_scale = (camera.fov / 2.0).tan();
    let frame = camera.frame();
    let scale = settings.scale.max(1);

    // Color del pixel (x, y) de la pantalla; x e y pueden tener decimales
    let trace_pixel = |x: f32, y: f32| -> [u8; 3] {
        let screen_x = (2.0 * x) / width as f32 - 1.0;
        let screen_y = -(2.0 * y) / height as f32 + 1.0;
        let screen_x = screen_x * aspect_ratio * fov_scale;
        let screen_y = screen_y * fov_scale;

        // Direccion en espacio de camara, pasada a espacio mundo
        let ray_direction = frame.ray_direction(&Vec3::new(screen_x, screen_y, -1.0));
        cast_ray(&frame.eye, &ray_direction, scene, 0).to_bytes()
    };

    // Cada banda es un pedazo contiguo del framebuffer con varias filas completas.
    // Cada hilo escribe solo en la banda que tomo, asi que no hacen falta locks
    // para los pixeles: el Mutex solo protege "cual es la siguiente banda"
    let row_bytes = width * CHANNELS;
    let band_rows = BAND_BLOCK_ROWS * scale;
    let bands = Mutex::new(pixels.chunks_mut(row_bytes * band_rows).enumerate());

    thread::scope(|scope| {
        for _ in 0..settings.threads.max(1) {
            scope.spawn(|| {
                loop {
                    let next = bands.lock().unwrap().next();
                    let Some((band_index, band)) = next else {
                        break;
                    };
                    let first_row = band_index * band_rows;
                    let rows = band.len() / row_bytes;

                    for block_y in (0..rows).step_by(scale) {
                        for block_x in (0..width).step_by(scale) {
                            // Un rayo por el centro del bloque de scale x scale pixeles
                            let center = scale as f32 / 2.0;
                            let y = (first_row + block_y) as f32 + center;
                            let rgb = trace_pixel(block_x as f32 + center, y);

                            // Se copia el mismo color a todo el bloque
                            for py in block_y..(block_y + scale).min(rows) {
                                for px in block_x..(block_x + scale).min(width) {
                                    let index = py * row_bytes + px * CHANNELS;
                                    band[index..index + CHANNELS]
                                        .copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
                                }
                            }
                        }
                    }
                }
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const UP: Vec3 = Vec3::new(0.0, 1.0, 0.0);

    #[test]
    fn fresnel_de_frente_refleja_poco_y_de_lado_mucho() {
        let de_frente = fresnel(&Vec3::new(0.0, -1.0, 0.0), &UP, 1.33);
        let rasante = fresnel(&Vec3::new(1.0, -0.05, 0.0).normalize(), &UP, 1.33);
        // Agua de frente: ((1 - 1.33) / (1 + 1.33))^2 = ~2%
        assert!((de_frente - 0.02).abs() < 0.005);
        assert!(rasante > 0.6);
    }

    #[test]
    fn fresnel_reflexion_interna_total_refleja_todo() {
        // Desde dentro del agua hacia arriba, casi rasante
        let incidente = Vec3::new(0.95, 0.312, 0.0).normalize();
        assert_eq!(fresnel(&incidente, &UP, 1.33), 1.0);
    }
}
