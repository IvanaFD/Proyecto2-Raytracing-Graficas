use crate::color::Color;

// Cada bloque tiene sus propios parametros, como pide la rubrica
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Material {
    pub diffuse: Color,
    // Exponente de Phong: mas alto = brillo mas pequeno y concentrado
    pub specular: f32,
    // Pesos de cada canal de luz: [difuso, especular, reflectividad, transparencia].
    // Reflectividad se usa en la fase 14 y transparencia en la 15
    pub albedo: [f32; 4],
    // Indice de refraccion (aire 1.0, agua 1.33, vidrio 1.5); se usa en la fase 15
    #[allow(dead_code)]
    pub refractive_index: f32,
}

impl Material {
    pub const fn new(diffuse: Color, specular: f32, albedo: [f32; 4], refractive_index: f32) -> Self {
        Material {
            diffuse,
            specular,
            albedo,
            refractive_index,
        }
    }
}
