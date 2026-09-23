use std::f32::consts::PI;

use crate::material::Material;
use crate::ray_intersect::{Intersect, RayIntersect};
use crate::vec3::Vec3;

// Distancia minima para aceptar un impacto; evita que el rayo choque
// con la misma superficie de la que sale
const EPSILON: f32 = 1e-4;

pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
    pub material: Material,
}

impl RayIntersect for Sphere {
    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Intersect {
        let oc = *ray_origin - self.center;

        let a = ray_direction.dot(ray_direction);
        let b = 2.0 * oc.dot(ray_direction);
        let c = oc.dot(&oc) - self.radius * self.radius;

        let discriminant = b * b - 4.0 * a * c;
        if discriminant < 0.0 {
            return Intersect::empty();
        }

        // Las dos soluciones de la cuadratica: t1 es la entrada, t2 la salida
        let sqrt_discriminant = discriminant.sqrt();
        let t1 = (-b - sqrt_discriminant) / (2.0 * a);
        let t2 = (-b + sqrt_discriminant) / (2.0 * a);

        // Si t1 queda detras de la camara, puede que estemos dentro de la esfera
        let t = if t1 > EPSILON {
            t1
        } else if t2 > EPSILON {
            t2
        } else {
            return Intersect::empty();
        };

        let point = *ray_origin + *ray_direction * t;
        let normal = (point - self.center).normalize();

        // Mapeo esferico: u da la vuelta en el ecuador, v va de polo a polo
        let u = 0.5 + normal.z.atan2(normal.x) / (2.0 * PI);
        let v = 0.5 - normal.y.asin() / PI;

        Intersect::new(point, normal, t, (u, v), self.material)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;

    fn esfera_en(center: Vec3) -> Sphere {
        Sphere {
            center,
            radius: 1.0,
            material: Material::new(Color::new(1.0, 1.0, 1.0)),
        }
    }

    #[test]
    fn distancia_al_frente_de_la_esfera() {
        let sphere = esfera_en(Vec3::new(0.0, 0.0, -5.0));
        let hit = sphere.ray_intersect(&Vec3::default(), &Vec3::new(0.0, 0.0, -1.0));
        assert!(hit.is_intersecting);
        assert!((hit.distance - 4.0).abs() < 1e-5);
        assert_eq!(hit.normal, Vec3::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn esfera_detras_de_la_camara_no_se_ve() {
        let sphere = esfera_en(Vec3::new(0.0, 0.0, 5.0));
        let hit = sphere.ray_intersect(&Vec3::default(), &Vec3::new(0.0, 0.0, -1.0));
        assert!(!hit.is_intersecting);
    }

    #[test]
    fn rayo_que_pasa_de_lado_no_choca() {
        let sphere = esfera_en(Vec3::new(3.0, 0.0, -5.0));
        let hit = sphere.ray_intersect(&Vec3::default(), &Vec3::new(0.0, 0.0, -1.0));
        assert!(!hit.is_intersecting);
    }
}
