// Skybox: un cubemap de 6 caras generado por codigo. Hay dos cielos, atardecer y
// noche, hechos con el mismo generador y distintos colores.
// Cada cara es una imagen; un rayo que no choca con nada toma el color del cielo
// buscando en que cara y en que pixel cae su direccion

use crate::color::Color;
use crate::noise::{hash3, value_noise};
use crate::vec3::Vec3;

// Pixeles por lado de cada cara. Con 256 el degradado se ve suave y las nubes
// siguen cuadradas como en Minecraft
const FACE_SIZE: usize = 256;

const CLOUD_SEED: u32 = 77;
const STAR_SEED: u32 = 91;
const CRATER_SEED: u32 = 93;

// Todo lo que cambia entre un cielo y otro
struct SkyStyle {
    // Degradado de abajo hacia arriba
    below: Color,
    horizon: Color,
    low_sky: Color,
    high_sky: Color,
    zenith: Color,
    // El astro (sol o luna): centro, orilla, resplandor y medio lado del cuadrado
    body_core: Color,
    body_edge: Color,
    glow: Color,
    body_size: f32,
    // La luna tiene crateres; el sol no
    craters: bool,
    cloud_lit: Color,
    cloud_shadow: Color,
    cloud_opacity: f32,
    // Desde que altura salen las estrellas y que tan seguido (mas bajo = mas estrellas)
    star_height: f32,
    star_threshold: f32,
}

const SUNSET: SkyStyle = SkyStyle {
    below: Color::from_rgb(92, 46, 72),
    horizon: Color::from_rgb(255, 148, 72),
    low_sky: Color::from_rgb(232, 96, 82),
    high_sky: Color::from_rgb(104, 52, 118),
    zenith: Color::from_rgb(30, 24, 72),
    body_core: Color::new(1.3, 1.05, 0.7),
    body_edge: Color::new(1.2, 0.72, 0.38),
    glow: Color::new(1.0, 0.55, 0.28),
    body_size: 0.07,
    craters: false,
    cloud_lit: Color::from_rgb(255, 184, 150),
    cloud_shadow: Color::from_rgb(146, 86, 128),
    cloud_opacity: 0.9,
    star_height: 0.45,
    star_threshold: 0.996,
};

const NIGHT: SkyStyle = SkyStyle {
    below: Color::from_rgb(6, 6, 16),
    horizon: Color::from_rgb(28, 34, 72),
    low_sky: Color::from_rgb(18, 22, 54),
    high_sky: Color::from_rgb(10, 12, 34),
    zenith: Color::from_rgb(4, 5, 16),
    body_core: Color::new(1.1, 1.1, 1.15),
    body_edge: Color::new(0.8, 0.82, 0.9),
    glow: Color::new(0.25, 0.3, 0.5),
    body_size: 0.055,
    craters: true,
    cloud_lit: Color::from_rgb(58, 64, 98),
    cloud_shadow: Color::from_rgb(24, 26, 48),
    cloud_opacity: 0.7,
    star_height: 0.02,
    star_threshold: 0.99,
};

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
fn gradient(style: &SkyStyle, height: f32) -> Color {
    if height < 0.0 {
        style.horizon.lerp(&style.below, smoothstep(0.0, 0.3, -height))
    } else if height < 0.08 {
        style.horizon.lerp(&style.low_sky, smoothstep(0.0, 0.08, height))
    } else if height < 0.4 {
        style.low_sky.lerp(&style.high_sky, smoothstep(0.08, 0.4, height))
    } else {
        style.high_sky.lerp(&style.zenith, smoothstep(0.4, 1.0, height))
    }
}

// El sol o la luna
struct Body {
    direction: Vec3,
    // Base del plano tangente al astro, para dibujarlo cuadrado
    right: Vec3,
    up: Vec3,
}

impl Body {
    fn new(direction: Vec3) -> Self {
        let direction = direction.normalize();
        let right = direction.cross(&Vec3::new(0.0, 1.0, 0.0)).normalize();
        let up = right.cross(&direction);
        Body {
            direction,
            right,
            up,
        }
    }
}

// Color del cielo en una direccion. Se usa solo al generar las caras
fn sky(direction: &Vec3, body: &Body, style: &SkyStyle) -> Color {
    let height = direction.y;
    let mut color = gradient(style, height);

    // Estrellas: puntos al azar que se encienden mas arriba en el cielo
    if height > style.star_height {
        let cell = *direction * 180.0;
        let star = hash3(cell.x as i32, cell.y as i32, cell.z as i32, STAR_SEED);
        if star > style.star_threshold {
            let brightness = smoothstep(style.star_height, style.star_height + 0.35, height);
            color = color + Color::new(1.0, 0.95, 0.9) * (brightness * 0.8);
        }
    }

    // Resplandor alrededor del astro
    let toward_body = direction.dot(&body.direction).max(0.0);
    color = color + style.glow * (toward_body.powf(8.0) * 0.35 + toward_body.powf(64.0) * 0.6);

    // Nubes cuadradas en un "techo" plano: la direccion se proyecta sobre el plano
    // y = 1, y cada celda de la cuadricula es nube o no
    if height > 0.02 {
        let x = direction.x / height * 4.0;
        let z = direction.z / height * 4.0;
        let density = value_noise(x.floor() * 0.3, z.floor() * 0.3, CLOUD_SEED);
        if density > 0.6 {
            // Mas iluminada del lado del astro
            let lit = 0.5 + 0.5 * direction.dot(&body.direction);
            let cloud = style.cloud_shadow.lerp(&style.cloud_lit, lit);
            // Las nubes lejanas (cerca del horizonte) se desvanecen en el cielo
            let opacity = smoothstep(0.02, 0.25, height) * style.cloud_opacity;
            color = color.lerp(&cloud, opacity);
        }
    }

    // Astro cuadrado: la direccion se proyecta sobre el plano tangente al astro
    let facing = direction.dot(&body.direction);
    if facing > 0.0 {
        let local_x = direction.dot(&body.right) / facing;
        let local_y = direction.dot(&body.up) / facing;
        let edge = local_x.abs().max(local_y.abs());
        if edge < style.body_size {
            color = if edge < style.body_size * 0.6 { style.body_core } else { style.body_edge };

            // Crateres de la luna: manchas grises en una cuadricula de 8x8
            if style.craters {
                let cell_x = ((local_x / style.body_size + 1.0) * 4.0) as i32;
                let cell_y = ((local_y / style.body_size + 1.0) * 4.0) as i32;
                if hash3(cell_x, cell_y, 0, CRATER_SEED) > 0.72 {
                    color = color * 0.72;
                }
            }
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
    fn generate(body_direction: Vec3, style: &SkyStyle) -> Self {
        let body = Body::new(body_direction);
        let faces = FACES
            .iter()
            .map(|&face| {
                let mut pixels = Vec::with_capacity(FACE_SIZE * FACE_SIZE);
                for y in 0..FACE_SIZE {
                    for x in 0..FACE_SIZE {
                        let u = (x as f32 + 0.5) / FACE_SIZE as f32;
                        let v = (y as f32 + 0.5) / FACE_SIZE as f32;
                        pixels.push(sky(&direction_of(face, u, v), &body, style));
                    }
                }
                pixels
            })
            .collect();
        Skybox { faces }
    }

    // Atardecer rojizo con el sol cuadrado bajo en el horizonte
    pub fn sunset(sun_direction: Vec3) -> Self {
        Self::generate(sun_direction, &SUNSET)
    }

    // Noche: azul oscuro, muchas estrellas y la luna cuadrada con crateres
    pub fn night(moon_direction: Vec3) -> Self {
        Self::generate(moon_direction, &NIGHT)
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
        assert!(gradient(&SUNSET, 0.0).r > gradient(&SUNSET, 1.0).r);
        assert!(gradient(&SUNSET, 1.0).b > gradient(&SUNSET, 1.0).r);
    }

    #[test]
    fn la_noche_es_mas_oscura_que_el_atardecer() {
        for height in [-0.5, 0.0, 0.3, 1.0] {
            let night = gradient(&NIGHT, height);
            let sunset = gradient(&SUNSET, height);
            assert!(night.r + night.g + night.b < sunset.r + sunset.g + sunset.b);
        }
    }
}
