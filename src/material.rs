use crate::color::Color;

// Por ahora solo el canal difuso; en la fase 6 se agregan specular,
// reflectividad, transparencia, indice de refraccion y emision
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Material {
    pub diffuse: Color,
}

impl Material {
    pub const fn new(diffuse: Color) -> Self {
        Material { diffuse }
    }
}
