use crate::material::Material;
use crate::ray_intersect::{Intersect, RayIntersect};
use crate::vec3::Vec3;

const EPSILON: f32 = 1e-4;

// Caja alineada a los ejes (AABB): un bloque de Minecraft
pub struct Cube {
    pub min: Vec3,
    pub max: Vec3,
    pub material: Material,
}

impl Cube {
    // Bloque de 1x1x1 cuya esquina inferior esta en `position`
    pub fn block(position: Vec3, material: Material) -> Self {
        Cube {
            min: position,
            max: position + Vec3::new(1.0, 1.0, 1.0),
            material,
        }
    }
}

// Entrada y salida del rayo en el par de planos de un eje (metodo slab)
fn slab(origin: f32, direction: f32, min: f32, max: f32) -> (f32, f32) {
    // Si direction es 0, 1/0 = infinito y el eje nunca limita el intervalo
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
        let mut t_enter = f32::NEG_INFINITY;
        let mut t_exit = f32::INFINITY;
        let mut enter_axis = 0;
        let mut exit_axis = 0;

        for axis in 0..3 {
            let (t_near, t_far) = slab(origin[axis], direction[axis], min[axis], max[axis]);
            if t_near > t_enter {
                t_enter = t_near;
                enter_axis = axis;
            }
            if t_far < t_exit {
                t_exit = t_far;
                exit_axis = axis;
            }
        }

        if t_enter > t_exit || t_exit < EPSILON {
            return Intersect::empty();
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
    fn desde_adentro_choca_con_la_salida() {
        let hit = bloque().ray_intersect(&Vec3::new(0.0, 0.0, -5.0), &Vec3::new(1.0, 0.0, 0.0));
        assert!(hit.is_intersecting);
        assert!((hit.distance - 0.5).abs() < TOLERANCE);
        assert_eq!(hit.normal, Vec3::new(1.0, 0.0, 0.0));
    }
}
