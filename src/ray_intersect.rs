use crate::material::Material;
use crate::vec3::Vec3;

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct Intersect {
    // Punto y normal se usan desde la fase 6 (iluminacion)
    pub point: Vec3,
    pub normal: Vec3,
    pub distance: f32,
    pub is_intersecting: bool,
    pub material: Material,
}

impl Intersect {
    pub fn new(point: Vec3, normal: Vec3, distance: f32, material: Material) -> Self {
        Intersect {
            point,
            normal,
            distance,
            is_intersecting: true,
            material,
        }
    }

    pub fn empty() -> Self {
        Intersect {
            point: Vec3::default(),
            normal: Vec3::default(),
            distance: f32::INFINITY,
            is_intersecting: false,
            material: Material::default(),
        }
    }
}

pub trait RayIntersect {
    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Intersect;
}
