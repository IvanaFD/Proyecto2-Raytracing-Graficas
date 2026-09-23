use std::ops::{Add, Mul};

// Canales en rango 0.0 - 1.0; pueden pasarse de 1.0 al acumular luz
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Color {
    pub const fn new(r: f32, g: f32, b: f32) -> Self {
        Color { r, g, b }
    }

    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Color::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
    }

    // t = 0 da self, t = 1 da other
    pub fn lerp(&self, other: &Color, t: f32) -> Color {
        *self * (1.0 - t) + *other * t
    }

    pub fn to_bytes(&self) -> [u8; 3] {
        [
            (self.r.clamp(0.0, 1.0) * 255.0) as u8,
            (self.g.clamp(0.0, 1.0) * 255.0) as u8,
            (self.b.clamp(0.0, 1.0) * 255.0) as u8,
        ]
    }
}

impl Add for Color {
    type Output = Color;
    fn add(self, other: Color) -> Color {
        Color::new(self.r + other.r, self.g + other.g, self.b + other.b)
    }
}

impl Mul<f32> for Color {
    type Output = Color;
    fn mul(self, scalar: f32) -> Color {
        Color::new(self.r * scalar, self.g * scalar, self.b * scalar)
    }
}

// Canal por canal: se usa para teñir la luz con el color de una superficie
impl Mul<Color> for Color {
    type Output = Color;
    fn mul(self, other: Color) -> Color {
        Color::new(self.r * other.r, self.g * other.g, self.b * other.b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_rgb_convierte_a_rango_unitario() {
        assert_eq!(Color::from_rgb(255, 0, 51), Color::new(1.0, 0.0, 0.2));
    }

    #[test]
    fn to_bytes_recorta_fuera_de_rango() {
        assert_eq!(Color::new(1.5, -0.3, 0.5).to_bytes(), [255, 0, 127]);
    }

    #[test]
    fn lerp_extremos_y_mitad() {
        let negro = Color::new(0.0, 0.0, 0.0);
        let blanco = Color::new(1.0, 1.0, 1.0);
        assert_eq!(negro.lerp(&blanco, 0.0), negro);
        assert_eq!(negro.lerp(&blanco, 1.0), blanco);
        assert_eq!(negro.lerp(&blanco, 0.5), Color::new(0.5, 0.5, 0.5));
    }

    #[test]
    fn multiplicar_colores_es_por_canal() {
        let luz = Color::new(1.0, 0.5, 0.0);
        let superficie = Color::new(0.5, 0.5, 1.0);
        assert_eq!(luz * superficie, Color::new(0.5, 0.25, 0.0));
    }
}
