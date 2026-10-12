use crate::material::Material;
use crate::vec3::Vec3;

#[derive(Debug, Clone, Copy)]
pub struct Intersect {
    pub point: Vec3,
    pub normal: Vec3,
    pub distance: f32,
    // Coordenadas de textura en [0,1]
    pub uv: (f32, f32),
    pub is_intersecting: bool,
    pub material: Material,
}

impl Intersect {
    pub fn new(
        point: Vec3,
        normal: Vec3,
        distance: f32,
        uv: (f32, f32),
        material: Material,
    ) -> Self {
        Intersect {
            point,
            normal,
            distance,
            uv,
            is_intersecting: true,
            material,
        }
    }

    pub fn empty() -> Self {
        Intersect {
            point: Vec3::default(),
            normal: Vec3::default(),
            distance: f32::INFINITY,
            uv: (0.0, 0.0),
            is_intersecting: false,
            material: Material::default(),
        }
    }
}

// Sync: varios hilos leen la escena al mismo tiempo mientras trazan
pub trait RayIntersect: Sync {
    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Intersect;
}
