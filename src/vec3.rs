use std::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Vec3 { x, y, z }
    }

    pub fn dot(&self, other: &Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    // Vector perpendicular a ambos (regla de la mano derecha)
    pub fn cross(&self, other: &Vec3) -> Vec3 {
        Vec3::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    pub fn length(&self) -> f32 {
        self.dot(self).sqrt()
    }

    pub fn normalize(&self) -> Vec3 {
        let length = self.length();
        // Evita dividir entre cero y propagar NaN
        if length == 0.0 {
            return *self;
        }
        *self / length
    }

    // Refleja el vector respecto a la normal: R = I - 2(I.N)N
    pub fn reflect(&self, normal: &Vec3) -> Vec3 {
        *self - *normal * (2.0 * self.dot(normal))
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, other: Vec3) -> Vec3 {
        Vec3::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, other: Vec3) -> Vec3 {
        Vec3::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
}

impl Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, scalar: f32) -> Vec3 {
        Vec3::new(self.x * scalar, self.y * scalar, self.z * scalar)
    }
}

impl Div<f32> for Vec3 {
    type Output = Vec3;
    fn div(self, scalar: f32) -> Vec3 {
        Vec3::new(self.x / scalar, self.y / scalar, self.z / scalar)
    }
}

impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-6;

    #[test]
    fn suma_y_resta() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        assert_eq!(a + b, Vec3::new(5.0, 7.0, 9.0));
        assert_eq!(b - a, Vec3::new(3.0, 3.0, 3.0));
        assert_eq!(-a, Vec3::new(-1.0, -2.0, -3.0));
    }

    #[test]
    fn escalar() {
        let a = Vec3::new(1.0, -2.0, 3.0);
        assert_eq!(a * 2.0, Vec3::new(2.0, -4.0, 6.0));
        assert_eq!(a / 2.0, Vec3::new(0.5, -1.0, 1.5));
    }

    #[test]
    fn producto_punto() {
        let x = Vec3::new(1.0, 0.0, 0.0);
        let y = Vec3::new(0.0, 1.0, 0.0);
        assert_eq!(x.dot(&x), 1.0);
        assert_eq!(x.dot(&y), 0.0);
        assert_eq!(x.dot(&-x), -1.0);
        assert_eq!(Vec3::new(1.0, 2.0, 3.0).dot(&Vec3::new(4.0, 5.0, 6.0)), 32.0);
    }

    #[test]
    fn producto_cruz() {
        let x = Vec3::new(1.0, 0.0, 0.0);
        let y = Vec3::new(0.0, 1.0, 0.0);
        let z = Vec3::new(0.0, 0.0, 1.0);
        assert_eq!(x.cross(&y), z);
        assert_eq!(y.cross(&z), x);
        assert_eq!(y.cross(&x), -z);
    }

    #[test]
    fn largo() {
        assert_eq!(Vec3::new(3.0, 4.0, 0.0).length(), 5.0);
    }

    #[test]
    fn normalize_da_largo_uno() {
        let n = Vec3::new(3.0, -7.0, 12.0).normalize();
        assert!((n.length() - 1.0).abs() < EPSILON);
    }

    #[test]
    fn reflect_invierte_la_componente_normal() {
        let incidente = Vec3::new(1.0, -1.0, 0.0);
        let normal = Vec3::new(0.0, 1.0, 0.0);
        assert_eq!(incidente.reflect(&normal), Vec3::new(1.0, 1.0, 0.0));
    }

    #[test]
    fn normalize_de_cero_no_da_nan() {
        let n = Vec3::default().normalize();
        assert_eq!(n, Vec3::default());
    }
}
