// Skybox de atardecer: un cubemap de 6 caras generado por codigo.
// Cada cara es una imagen; un rayo que no choca con nada toma el color del cielo
// buscando en que cara y en que pixel cae su direccion

use crate::color::Color;
use crate::noise::{hash3, value_noise};
use crate::vec3::Vec3;

// Pixeles por lado de cada cara. Con 256 el degradado se ve suave y las nubes
// siguen cuadradas como en Minecraft
const FACE_SIZE: usize = 256;

// Degradado del atardecer, de abajo hacia arriba
const BELOW: Color = Color::from_rgb(92, 46, 72);
const HORIZON: Color = Color::from_rgb(255, 148, 72);
const LOW_SKY: Color = Color::from_rgb(232, 96, 82);
const HIGH_SKY: Color = Color::from_rgb(104, 52, 118);
const ZENITH: Color = Color::from_rgb(30, 24, 72);

const SUN_CORE: Color = Color::new(1.3, 1.05, 0.7);
const SUN_EDGE: Color = Color::new(1.2, 0.72, 0.38);
const SUN_GLOW: Color = Color::new(1.0, 0.55, 0.28);
// Medio lado del sol cuadrado, medido en el plano tangente al sol
const SUN_SIZE: f32 = 0.07;

const CLOUD_LIT: Color = Color::from_rgb(255, 184, 150);
const CLOUD_SHADOW: Color = Color::from_rgb(146, 86, 128);
const CLOUD_SEED: u32 = 77;
const STAR_SEED: u32 = 91;

// Las 6 caras en el orden +X, -X, +Y, -Y, +Z, -Z
#[derive(Debug, Clone, Copy, PartialEq)]
enum Face {
    PositiveX,
    NegativeX,
    PositiveY,
    NegativeY,
    PositiveZ,
    NegativeZ,
}

const FACES: [Face; 6] = [
    Face::PositiveX,
    Face::NegativeX,
    Face::PositiveY,
    Face::NegativeY,
    Face::PositiveZ,
    Face::NegativeZ,
];

// La cara la elige el componente con mayor valor absoluto de la direccion.
// Las otras dos componentes divididas entre ese valor dan una posicion en
// [-1, 1] dentro de la cara, que se lleva a UV en [0, 1]
fn face_and_uv(direction: &Vec3) -> (Face, f32, f32) {
    let (ax, ay, az) = (direction.x.abs(), direction.y.abs(), direction.z.abs());

    let (face, sc, tc, major) = if ax >= ay && ax >= az {
        if direction.x > 0.0 {
            (Face::PositiveX, -direction.z, -direction.y, ax)
        } else {
            (Face::NegativeX, direction.z, -direction.y, ax)
        }
    } else if ay >= az {
        if direction.y > 0.0 {
            (Face::PositiveY, direction.x, direction.z, ay)
        } else {
            (Face::NegativeY, direction.x, -direction.z, ay)
        }
    } else if direction.z > 0.0 {
        (Face::PositiveZ, direction.x, -direction.y, az)
    } else {
        (Face::NegativeZ, -direction.x, -direction.y, az)
    };

    (face, 0.5 * (sc / major + 1.0), 0.5 * (tc / major + 1.0))
}

// Lo inverso: la direccion que corresponde a (u, v) en una cara
fn direction_of(face: Face, u: f32, v: f32) -> Vec3 {
    let sc = 2.0 * u - 1.0;
    let tc = 2.0 * v - 1.0;
    let direction = match face {
        Face::PositiveX => Vec3::new(1.0, -tc, -sc),
        Face::NegativeX => Vec3::new(-1.0, -tc, sc),
        Face::PositiveY => Vec3::new(sc, 1.0, tc),
        Face::NegativeY => Vec3::new(sc, -1.0, -tc),
        Face::PositiveZ => Vec3::new(sc, -tc, 1.0),
        Face::NegativeZ => Vec3::new(-sc, -tc, -1.0),
    };
    direction.normalize()
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

// Degradado vertical del cielo segun la altura de la direccion (y de -1 a 1)
fn gradient(height: f32) -> Color {
    if height < 0.0 {
        HORIZON.lerp(&BELOW, smoothstep(0.0, 0.3, -height))
    } else if height < 0.08 {
        HORIZON.lerp(&LOW_SKY, smoothstep(0.0, 0.08, height))
    } else if height < 0.4 {
        LOW_SKY.lerp(&HIGH_SKY, smoothstep(0.08, 0.4, height))
    } else {
        HIGH_SKY.lerp(&ZENITH, smoothstep(0.4, 1.0, height))
    }
}

struct Sun {
    direction: Vec3,
    // Base del plano tangente al sol, para dibujarlo cuadrado
    right: Vec3,
    up: Vec3,
}

impl Sun {
    fn new(direction: Vec3) -> Self {
        let direction = direction.normalize();
        let right = direction.cross(&Vec3::new(0.0, 1.0, 0.0)).normalize();
        let up = right.cross(&direction);
        Sun {
            direction,
            right,
            up,
        }
    }
}

// Color del cielo en una direccion. Se usa solo al generar las caras
fn sky(direction: &Vec3, sun: &Sun) -> Color {
    let height = direction.y;
    let mut color = gradient(height);

    // Estrellas que empiezan a salir arriba
    if height > 0.45 {
        let cell = *direction * 180.0;
        let star = hash3(cell.x as i32, cell.y as i32, cell.z as i32, STAR_SEED);
        if star > 0.996 {
            color = color + Color::new(1.0, 0.95, 0.9) * (smoothstep(0.45, 0.8, height) * 0.8);
        }
    }

    // Resplandor alrededor del sol
    let toward_sun = direction.dot(&sun.direction).max(0.0);
    color = color + SUN_GLOW * (toward_sun.powf(8.0) * 0.35 + toward_sun.powf(64.0) * 0.6);

    // Nubes cuadradas en un "techo" plano: la direccion se proyecta sobre el plano
    // y = 1, y cada celda de la cuadricula es nube o no
    if height > 0.02 {
        let x = direction.x / height * 4.0;
        let z = direction.z / height * 4.0;
        let density = value_noise(x.floor() * 0.3, z.floor() * 0.3, CLOUD_SEED);
        if density > 0.6 {
            // Mas naranja del lado del sol, mas morada del lado contrario
            let lit = 0.5 + 0.5 * direction.dot(&sun.direction);
            let cloud = CLOUD_SHADOW.lerp(&CLOUD_LIT, lit);
            // Las nubes lejanas (cerca del horizonte) se desvanecen en el cielo
            let opacity = smoothstep(0.02, 0.25, height) * 0.9;
            color = color.lerp(&cloud, opacity);
        }
    }

    // Sol cuadrado: la direccion se proyecta sobre el plano tangente al sol
    let facing = direction.dot(&sun.direction);
    if facing > 0.0 {
        let local_x = direction.dot(&sun.right) / facing;
        let local_y = direction.dot(&sun.up) / facing;
        let edge = local_x.abs().max(local_y.abs());
        if edge < SUN_SIZE {
            color = if edge < SUN_SIZE * 0.6 { SUN_CORE } else { SUN_EDGE };
        }
    }

    color
}

pub struct Skybox {
    faces: Vec<Vec<Color>>,
}

impl Skybox {
    // Genera las 6 caras una sola vez: para cada pixel de cada cara se calcula
    // su direccion y el color del cielo en esa direccion
    pub fn sunset(sun_direction: Vec3) -> Self {
        let sun = Sun::new(sun_direction);
        let faces = FACES
            .iter()
            .map(|&face| {
                let mut pixels = Vec::with_capacity(FACE_SIZE * FACE_SIZE);
                for y in 0..FACE_SIZE {
                    for x in 0..FACE_SIZE {
                        let u = (x as f32 + 0.5) / FACE_SIZE as f32;
                        let v = (y as f32 + 0.5) / FACE_SIZE as f32;
                        pixels.push(sky(&direction_of(face, u, v), &sun));
                    }
                }
                pixels
            })
            .collect();
        Skybox { faces }
    }

    pub fn sample(&self, direction: &Vec3) -> Color {
        let (face, u, v) = face_and_uv(direction);
        let x = ((u * FACE_SIZE as f32) as usize).min(FACE_SIZE - 1);
        let y = ((v * FACE_SIZE as f32) as usize).min(FACE_SIZE - 1);
        self.faces[face as usize][y * FACE_SIZE + x]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cerca(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-4
    }

    #[test]
    fn cada_eje_cae_en_su_cara() {
        let casos = [
            (Vec3::new(1.0, 0.0, 0.0), Face::PositiveX),
            (Vec3::new(-1.0, 0.0, 0.0), Face::NegativeX),
            (Vec3::new(0.0, 1.0, 0.0), Face::PositiveY),
            (Vec3::new(0.0, -1.0, 0.0), Face::NegativeY),
            (Vec3::new(0.0, 0.0, 1.0), Face::PositiveZ),
            (Vec3::new(0.0, 0.0, -1.0), Face::NegativeZ),
        ];
        for (direction, face) in casos {
            let (found, u, v) = face_and_uv(&direction);
            assert_eq!(found, face);
            assert!((u - 0.5).abs() < 1e-6 && (v - 0.5).abs() < 1e-6);
        }
    }

    #[test]
    fn ida_y_vuelta_entre_direccion_y_uv() {
        // Si la cara y la UV no fueran consistentes se verian costuras entre caras
        let direcciones = [
            Vec3::new(0.3, 0.8, -0.5),
            Vec3::new(-0.9, 0.2, 0.4),
            Vec3::new(0.1, -0.7, 0.7),
            Vec3::new(0.6, -0.1, -0.79),
        ];
        for direction in direcciones {
            let direction = direction.normalize();
            let (face, u, v) = face_and_uv(&direction);
            assert!(cerca(direction_of(face, u, v), direction));
        }
    }

    #[test]
    fn el_horizonte_es_mas_calido_que_el_cenit() {
        assert!(gradient(0.0).r > gradient(1.0).r);
        assert!(gradient(1.0).b > gradient(1.0).r);
    }
}
