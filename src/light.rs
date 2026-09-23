use crate::color::Color;
use crate::vec3::Vec3;

// Luz puntual: ilumina en todas direcciones desde una posicion
pub struct Light {
    pub position: Vec3,
    pub color: Color,
    pub intensity: f32,
}

impl Light {
    pub const fn new(position: Vec3, color: Color, intensity: f32) -> Self {
        Light {
            position,
            color,
            intensity,
        }
    }
}
