use std::f32::consts::FRAC_PI_2;

use crate::vec3::Vec3;

const WORLD_UP: Vec3 = Vec3::new(0.0, 1.0, 0.0);

// Limites para no pasar por debajo del piso ni llegar a mirar justo hacia
// abajo, donde forward y WORLD_UP son paralelos y el producto cruz da cero
const MIN_PITCH: f32 = 0.05;
const MAX_PITCH: f32 = FRAC_PI_2 - 0.05;

// Posicion y base ortonormal de la camara para un frame. Se calcula una sola
// vez por frame, no por cada pixel
pub struct CameraFrame {
    pub eye: Vec3,
    forward: Vec3,
    right: Vec3,
    up: Vec3,
}

impl CameraFrame {
    // Cambio de base: pasa una direccion en espacio de camara (mirando a -Z)
    // a espacio mundo
    pub fn ray_direction(&self, direction: &Vec3) -> Vec3 {
        (self.right * direction.x + self.up * direction.y - self.forward * direction.z).normalize()
    }
}

// Camara que orbita alrededor de `center` en coordenadas esfericas
pub struct Camera {
    pub center: Vec3,
    // Angulo alrededor del eje Y (girar el diorama)
    pub yaw: f32,
    // Angulo sobre el horizonte (inclinar)
    pub pitch: f32,
    // Distancia al centro (zoom)
    pub radius: f32,
    pub min_radius: f32,
    pub max_radius: f32,
    // Campo de vision vertical en radianes
    pub fov: f32,
}

impl Camera {
    pub fn eye(&self) -> Vec3 {
        let offset = Vec3::new(
            self.pitch.cos() * self.yaw.sin(),
            self.pitch.sin(),
            self.pitch.cos() * self.yaw.cos(),
        );
        self.center + offset * self.radius
    }

    // Base ortonormal de la camara: hacia donde mira, su derecha y su arriba
    pub fn frame(&self) -> CameraFrame {
        let eye = self.eye();
        let forward = (self.center - eye).normalize();
        let right = forward.cross(&WORLD_UP).normalize();
        let up = right.cross(&forward);
        CameraFrame {
            eye,
            forward,
            right,
            up,
        }
    }

    pub fn orbit(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.yaw += delta_yaw;
        self.pitch = (self.pitch + delta_pitch).clamp(MIN_PITCH, MAX_PITCH);
    }

    pub fn zoom(&mut self, delta: f32) {
        self.radius = (self.radius + delta).clamp(self.min_radius, self.max_radius);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f32 = 1e-5;

    fn cerca(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < TOLERANCE
    }

    fn camara() -> Camera {
        Camera {
            center: Vec3::new(0.0, 1.0, 0.0),
            yaw: 0.0,
            pitch: MIN_PITCH,
            radius: 10.0,
            min_radius: 5.0,
            max_radius: 20.0,
            fov: 1.0,
        }
    }

    #[test]
    fn el_centro_de_la_pantalla_mira_al_centro_del_diorama() {
        let mut camera = camara();
        for (yaw, pitch) in [(0.0, 0.0), (1.3, 0.4), (-2.0, 1.0)] {
            camera.yaw = yaw;
            camera.pitch = 0.0;
            camera.orbit(0.0, pitch);
            let toward_center = (camera.center - camera.eye()).normalize();
            let ray = camera.frame().ray_direction(&Vec3::new(0.0, 0.0, -1.0));
            assert!(cerca(ray, toward_center));
        }
    }

    #[test]
    fn la_base_es_ortonormal() {
        let mut camera = camara();
        camera.orbit(0.7, 0.5);
        let CameraFrame {
            forward, right, up, ..
        } = camera.frame();
        assert!(forward.dot(&right).abs() < TOLERANCE);
        assert!(forward.dot(&up).abs() < TOLERANCE);
        assert!(right.dot(&up).abs() < TOLERANCE);
        assert!((up.length() - 1.0).abs() < TOLERANCE);
        // Arriba de la camara apunta hacia arriba del mundo, no de cabeza
        assert!(up.y > 0.0);
    }

    #[test]
    fn el_zoom_y_la_inclinacion_respetan_los_limites() {
        let mut camera = camara();
        camera.zoom(-100.0);
        assert_eq!(camera.radius, 5.0);
        camera.zoom(100.0);
        assert_eq!(camera.radius, 20.0);
        camera.orbit(0.0, 10.0);
        assert_eq!(camera.pitch, MAX_PITCH);
        camera.orbit(0.0, -10.0);
        assert_eq!(camera.pitch, MIN_PITCH);
    }
}
