use crate::color::Color;
use crate::vec3::Vec3;

// Luz puntual: ilumina en todas direcciones desde una posicion
pub struct Light {
    pub position: Vec3,
    pub color: Color,
    pub intensity: f32,
    // Que tan rapido se apaga con la distancia: intensidad / (1 + k * d^2).
    // El sol esta tan lejos que se toma como 0 (no se apaga)
    pub falloff: f32,
    // Tamano del objeto que emite la luz. Los shadow rays ignoran lo que choquen a
    // menos de esta distancia de la luz: si no, la lampara se haria sombra a si misma
    pub radius: f32,
}

impl Light {
    pub const fn sun(position: Vec3, color: Color, intensity: f32) -> Self {
        Light {
            position,
            color,
            intensity,
            falloff: 0.0,
            radius: 0.0,
        }
    }

    // Luz de un bloque emisivo: se apaga con la distancia
    pub const fn point(position: Vec3, color: Color, intensity: f32, radius: f32) -> Self {
        Light {
            position,
            color,
            intensity,
            falloff: 0.25,
            radius,
        }
    }

    // Intensidad que llega a una distancia dada
    pub fn intensity_at(&self, distance: f32) -> f32 {
        self.intensity / (1.0 + self.falloff * distance * distance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHITE: Color = Color::new(1.0, 1.0, 1.0);

    #[test]
    fn el_sol_no_se_apaga_con_la_distancia() {
        let sun = Light::sun(Vec3::default(), WHITE, 1.2);
        assert_eq!(sun.intensity_at(500.0), 1.2);
    }

    #[test]
    fn una_luz_puntual_se_apaga_con_la_distancia() {
        let lamp = Light::point(Vec3::default(), WHITE, 2.0, 0.9);
        assert_eq!(lamp.intensity_at(0.0), 2.0);
        // A 2 bloques: 2 / (1 + 0.25 * 4) = 1
        assert!((lamp.intensity_at(2.0) - 1.0).abs() < 1e-6);
        assert!(lamp.intensity_at(8.0) < lamp.intensity_at(4.0));
    }
}
