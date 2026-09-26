// Matematica de cajas alineadas a los ejes (AABB), la forma de un bloque de Minecraft:
// interseccion con el metodo slab, normal de la cara y UV por cara.
// El mundo de voxels (world.rs) la usa para entrar al grid y para las texturas

use crate::vec3::Vec3;

const EPSILON: f32 = 1e-4;

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

// Donde entra y sale el rayo de la caja
pub struct BoxHit {
    pub t_enter: f32,
    pub t_exit: f32,
    // Eje (0 = x, 1 = y, 2 = z) de la cara por donde entra
    pub enter_axis: usize,
}

// t_enter = la mayor de las entradas de los 3 ejes, t_exit = la menor de las salidas.
// Hay impacto si t_enter <= t_exit y la caja no queda completa detras del rayo.
// Si el origen esta dentro de la caja, t_enter es negativo
pub fn ray_box(ray_origin: &Vec3, ray_direction: &Vec3, min: &Vec3, max: &Vec3) -> Option<BoxHit> {
    let origin = [ray_origin.x, ray_origin.y, ray_origin.z];
    let direction = [ray_direction.x, ray_direction.y, ray_direction.z];
    let min = [min.x, min.y, min.z];
    let max = [max.x, max.y, max.z];

    let mut t_enter = f32::NEG_INFINITY;
    let mut t_exit = f32::INFINITY;
    let mut enter_axis = 0;

    for axis in 0..3 {
        let (near, far) = slab(origin[axis], direction[axis], min[axis], max[axis]);
        if near > t_enter {
            t_enter = near;
            enter_axis = axis;
        }
        t_exit = t_exit.min(far);
    }

    if t_enter > t_exit || t_exit < EPSILON {
        return None;
    }

    Some(BoxHit {
        t_enter,
        t_exit,
        enter_axis,
    })
}

pub fn axis_normal(axis: usize, sign: f32) -> Vec3 {
    match axis {
        0 => Vec3::new(sign, 0.0, 0.0),
        1 => Vec3::new(0.0, sign, 0.0),
        _ => Vec3::new(0.0, 0.0, sign),
    }
}

// UV en [0,1] con las dos coordenadas que no son el eje de la normal.
// `local` es el punto dentro del bloque, de (0,0,0) a (1,1,1).
// v = 0 es la parte de arriba de la textura, y u avanza hacia la derecha
// de quien mira la cara desde afuera, para que ninguna cara salga en espejo
pub fn face_uv(local: Vec3, normal: Vec3) -> (f32, f32) {
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

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f32 = 1e-5;

    // Caja de (-0.5, -0.5, -5.5) a (0.5, 0.5, -4.5)
    const MIN: Vec3 = Vec3::new(-0.5, -0.5, -5.5);
    const MAX: Vec3 = Vec3::new(0.5, 0.5, -4.5);

    #[test]
    fn entra_por_la_cara_del_frente() {
        let hit = ray_box(&Vec3::default(), &Vec3::new(0.0, 0.0, -1.0), &MIN, &MAX).unwrap();
        assert!((hit.t_enter - 4.5).abs() < TOLERANCE);
        assert!((hit.t_exit - 5.5).abs() < TOLERANCE);
        assert_eq!(hit.enter_axis, 2);
    }

    #[test]
    fn rayo_que_pasa_de_lado_no_choca() {
        let hit = ray_box(&Vec3::new(2.0, 0.0, 0.0), &Vec3::new(0.0, 0.0, -1.0), &MIN, &MAX);
        assert!(hit.is_none());
    }

    #[test]
    fn caja_detras_del_rayo_no_se_ve() {
        let hit = ray_box(&Vec3::default(), &Vec3::new(0.0, 0.0, 1.0), &MIN, &MAX);
        assert!(hit.is_none());
    }

    #[test]
    fn rayo_paralelo_sobre_el_borde_no_da_nan() {
        // El origen esta en x = -0.5, justo en el plano de la cara -X, y el rayo no se mueve en x
        let hit = ray_box(&Vec3::new(-0.5, 0.0, 0.0), &Vec3::new(0.0, 0.0, -1.0), &MIN, &MAX);
        assert!((hit.unwrap().t_enter - 4.5).abs() < TOLERANCE);
    }

    #[test]
    fn desde_adentro_la_entrada_es_negativa() {
        let hit = ray_box(&Vec3::new(0.0, 0.0, -5.0), &Vec3::new(1.0, 0.0, 0.0), &MIN, &MAX).unwrap();
        assert!(hit.t_enter < 0.0);
        assert!((hit.t_exit - 0.5).abs() < TOLERANCE);
    }

    #[test]
    fn uv_del_centro_de_una_cara() {
        let uv = face_uv(Vec3::new(0.5, 0.5, 1.0), Vec3::new(0.0, 0.0, 1.0));
        assert_eq!(uv, (0.5, 0.5));
    }
}
