use std::collections::HashSet;

use crate::material::Material;
use crate::ray_intersect::{Intersect, RayIntersect};
use crate::vec3::Vec3;

const EPSILON: f32 = 1e-4;

// Si dos planos dan casi el mismo t, el rayo pego justo en una arista
const EDGE_TOLERANCE: f32 = 1e-4;

// Indice de cada cara en `hidden_faces`: eje * 2 + (0 si mira a negativo, 1 si a positivo)
// 0 = -X, 1 = +X, 2 = -Y, 3 = +Y, 4 = -Z, 5 = +Z
fn face_index(axis: usize, sign: f32) -> usize {
    axis * 2 + if sign > 0.0 { 1 } else { 0 }
}

// Caja alineada a los ejes (AABB): un bloque de Minecraft
pub struct Cube {
    pub min: Vec3,
    pub max: Vec3,
    pub material: Material,
    // Caras pegadas a otro bloque: nunca se ven, como en Minecraft
    pub hidden_faces: [bool; 6],
}

impl Cube {
    // Bloque de 1x1x1 cuya esquina inferior esta en `position`
    pub fn block(position: Vec3, material: Material) -> Self {
        Cube {
            min: position,
            max: position + Vec3::new(1.0, 1.0, 1.0),
            material,
            hidden_faces: [false; 6],
        }
    }

    // Posicion entera del bloque si es un bloque 1x1x1 alineado a la cuadricula
    fn grid_position(&self) -> Option<(i32, i32, i32)> {
        let size = self.max - self.min;
        let aligned = [self.min.x, self.min.y, self.min.z]
            .iter()
            .all(|value| value.fract() == 0.0);
        if aligned && size == Vec3::new(1.0, 1.0, 1.0) {
            Some((self.min.x as i32, self.min.y as i32, self.min.z as i32))
        } else {
            None
        }
    }
}

// Marca como ocultas las caras que tocan a otro bloque de la cuadricula
pub fn hide_touching_faces(cubes: &mut [Cube]) {
    let occupied: HashSet<(i32, i32, i32)> =
        cubes.iter().filter_map(|cube| cube.grid_position()).collect();

    for cube in cubes.iter_mut() {
        let Some((x, y, z)) = cube.grid_position() else {
            continue;
        };
        let neighbors = [
            (x - 1, y, z),
            (x + 1, y, z),
            (x, y - 1, z),
            (x, y + 1, z),
            (x, y, z - 1),
            (x, y, z + 1),
        ];
        for (face, neighbor) in neighbors.iter().enumerate() {
            cube.hidden_faces[face] = occupied.contains(neighbor);
        }
    }
}

// Entrada y salida del rayo en el par de planos de un eje (metodo slab)
fn slab(origin: f32, direction: f32, min: f32, max: f32) -> (f32, f32) {
    // Rayo paralelo a los planos: o siempre esta entre ellos o nunca.
    // Sin este caso, si el origen cae justo en un plano queda 0 * infinito = NaN
    if direction == 0.0 {
        return if origin >= min && origin <= max {
            (f32::NEG_INFINITY, f32::INFINITY)
        } else {
            (f32::INFINITY, f32::NEG_INFINITY)
        };
    }

    let inverse = 1.0 / direction;
    let t1 = (min - origin) * inverse;
    let t2 = (max - origin) * inverse;
    if t1 < t2 { (t1, t2) } else { (t2, t1) }
}

fn axis_normal(axis: usize, sign: f32) -> Vec3 {
    match axis {
        0 => Vec3::new(sign, 0.0, 0.0),
        1 => Vec3::new(0.0, sign, 0.0),
        _ => Vec3::new(0.0, 0.0, sign),
    }
}

// UV en [0,1] con las dos coordenadas que no son el eje de la normal.
// v = 0 es la parte de arriba de la textura, y u avanza hacia la derecha
// de quien mira la cara desde afuera, para que ninguna cara salga en espejo
fn face_uv(local: Vec3, normal: Vec3) -> (f32, f32) {
    if normal.x > 0.0 {
        (1.0 - local.z, 1.0 - local.y)
    } else if normal.x < 0.0 {
        (local.z, 1.0 - local.y)
    } else if normal.z > 0.0 {
        (local.x, 1.0 - local.y)
    } else if normal.z < 0.0 {
        (1.0 - local.x, 1.0 - local.y)
    } else if normal.y > 0.0 {
        (local.x, local.z)
    } else {
        (local.x, 1.0 - local.z)
    }
}

impl RayIntersect for Cube {
    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Intersect {
        let origin = [ray_origin.x, ray_origin.y, ray_origin.z];
        let direction = [ray_direction.x, ray_direction.y, ray_direction.z];
        let min = [self.min.x, self.min.y, self.min.z];
        let max = [self.max.x, self.max.y, self.max.z];

        // t_enter = el mayor de las entradas, t_exit = el menor de las salidas
        let mut t_near = [0.0; 3];
        let mut t_enter = f32::NEG_INFINITY;
        let mut t_exit = f32::INFINITY;
        let mut enter_axis = 0;
        let mut exit_axis = 0;

        for axis in 0..3 {
            let (near, far) = slab(origin[axis], direction[axis], min[axis], max[axis]);
            t_near[axis] = near;
            if near > t_enter {
                t_enter = near;
                enter_axis = axis;
            }
            if far < t_exit {
                t_exit = far;
                exit_axis = axis;
            }
        }

        if t_enter > t_exit || t_exit < EPSILON {
            return Intersect::empty();
        }

        // En una arista, la cara elegida puede ser una pegada al vecino. Si otra
        // cara empata en t y si se ve, el rayo en realidad entro por esa
        if self.hidden_faces[face_index(enter_axis, -direction[enter_axis])] {
            let visible_axis = (0..3).find(|&axis| {
                (t_near[axis] - t_enter).abs() < EDGE_TOLERANCE
                    && !self.hidden_faces[face_index(axis, -direction[axis])]
            });
            if let Some(axis) = visible_axis {
                enter_axis = axis;
            }
        }

        // Si el origen esta dentro del cubo (agua, vidrio) el impacto es la salida.
        // La normal siempre apunta hacia afuera de la cara
        let (t, normal) = if t_enter > EPSILON {
            (t_enter, axis_normal(enter_axis, -direction[enter_axis].signum()))
        } else {
            (t_exit, axis_normal(exit_axis, direction[exit_axis].signum()))
        };

        let point = *ray_origin + *ray_direction * t;
        let size = self.max - self.min;
        let offset = point - self.min;
        let local = Vec3::new(offset.x / size.x, offset.y / size.y, offset.z / size.z);
        let uv = face_uv(local, normal);

        Intersect::new(point, normal, t, uv, self.material)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;

    const TOLERANCE: f32 = 1e-5;

    fn bloque() -> Cube {
        // Ocupa de (-0.5, -0.5, -5.5) a (0.5, 0.5, -4.5)
        let material = Material::new(Color::new(1.0, 1.0, 1.0), 10.0, [1.0, 0.0, 0.0, 0.0], 1.0);
        Cube::block(Vec3::new(-0.5, -0.5, -5.5), material)
    }

    #[test]
    fn choca_con_la_cara_del_frente() {
        let hit = bloque().ray_intersect(&Vec3::default(), &Vec3::new(0.0, 0.0, -1.0));
        assert!(hit.is_intersecting);
        assert!((hit.distance - 4.5).abs() < TOLERANCE);
        assert_eq!(hit.normal, Vec3::new(0.0, 0.0, 1.0));
        assert!((hit.uv.0 - 0.5).abs() < TOLERANCE && (hit.uv.1 - 0.5).abs() < TOLERANCE);
    }

    #[test]
    fn choca_con_la_cara_de_arriba() {
        let hit = bloque().ray_intersect(&Vec3::new(0.0, 5.0, -5.0), &Vec3::new(0.0, -1.0, 0.0));
        assert!(hit.is_intersecting);
        assert!((hit.distance - 4.5).abs() < TOLERANCE);
        assert_eq!(hit.normal, Vec3::new(0.0, 1.0, 0.0));
    }

    #[test]
    fn rayo_que_pasa_de_lado_no_choca() {
        let hit = bloque().ray_intersect(&Vec3::new(2.0, 0.0, 0.0), &Vec3::new(0.0, 0.0, -1.0));
        assert!(!hit.is_intersecting);
    }

    #[test]
    fn cubo_detras_de_la_camara_no_se_ve() {
        let hit = bloque().ray_intersect(&Vec3::default(), &Vec3::new(0.0, 0.0, 1.0));
        assert!(!hit.is_intersecting);
    }

    #[test]
    fn en_la_arista_con_un_vecino_elige_la_cara_visible() {
        let material = Material::new(Color::new(1.0, 1.0, 1.0), 10.0, [1.0, 0.0, 0.0, 0.0], 1.0);
        let mut cubes = [
            Cube::block(Vec3::new(0.0, 0.0, -5.0), material),
            Cube::block(Vec3::new(1.0, 0.0, -5.0), material),
        ];
        hide_touching_faces(&mut cubes);
        assert!(cubes[0].hidden_faces[1] && cubes[1].hidden_faces[0]);

        // Rayo que baja en diagonal y pega justo en la arista x = 1 de la cara de arriba:
        // la cara -X del cubo derecho esta tapada, asi que debe salir la de arriba
        let origin = Vec3::new(0.0, 2.0, -4.5);
        let direction = Vec3::new(1.0, -1.0, 0.0).normalize();
        let hit = cubes[1].ray_intersect(&origin, &direction);
        assert!(hit.is_intersecting);
        assert_eq!(hit.normal, Vec3::new(0.0, 1.0, 0.0));
    }

    #[test]
    fn rayo_paralelo_sobre_el_borde_no_da_nan() {
        // El origen esta en x = -0.5, justo en el plano de la cara -X, y el rayo no se mueve en x
        let hit = bloque().ray_intersect(&Vec3::new(-0.5, 0.0, 0.0), &Vec3::new(0.0, 0.0, -1.0));
        assert!(hit.is_intersecting);
        assert!((hit.distance - 4.5).abs() < TOLERANCE);
    }

    #[test]
    fn desde_adentro_choca_con_la_salida() {
        let hit = bloque().ray_intersect(&Vec3::new(0.0, 0.0, -5.0), &Vec3::new(1.0, 0.0, 0.0));
        assert!(hit.is_intersecting);
        assert!((hit.distance - 0.5).abs() < TOLERANCE);
        assert_eq!(hit.normal, Vec3::new(1.0, 0.0, 0.0));
    }
}
