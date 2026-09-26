use crate::color::Color;
use crate::texture::BlockTextures;

// Cada bloque tiene sus propios parametros, como pide la rubrica
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Material {
    // Color base; se usa cuando el material no tiene textura
    pub diffuse: Color,
    // Textura de cada cara (arriba, lados, abajo)
    pub textures: Option<&'static BlockTextures>,
    // Exponente de Phong: mas alto = brillo mas pequeno y concentrado
    pub specular: f32,
    // Pesos de cada canal de luz: [difuso, especular, reflectividad, transparencia]
    pub albedo: [f32; 4],
    // Indice de refraccion (aire 1.0, agua 1.33, vidrio 1.5): que tanto se dobla la luz
    pub refractive_index: f32,
}

impl Material {
    pub const fn new(diffuse: Color, specular: f32, albedo: [f32; 4], refractive_index: f32) -> Self {
        Material {
            diffuse,
            textures: None,
            specular,
            albedo,
            refractive_index,
        }
    }

    pub const fn textured(self, textures: &'static BlockTextures) -> Self {
        Material {
            textures: Some(textures),
            ..self
        }
    }
}
