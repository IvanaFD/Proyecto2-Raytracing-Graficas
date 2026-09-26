// Aleatoriedad propia (sin crates): un generador con semilla y ruido 2D suave.
// Con la misma semilla siempre sale el mismo mundo

// Generador pseudoaleatorio SplitMix64: rapido y con buena distribucion
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    // Numero en [0, 1)
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
}

// Valor fijo en [0, 1) para cada punto entero (x, z) de la cuadricula
fn lattice(x: i32, z: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374_761_393)
        ^ (z as u32).wrapping_mul(668_265_263)
        ^ seed.wrapping_mul(2_246_822_519);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    (h & 0x00FF_FFFF) as f32 / 16_777_216.0
}

// Valor fijo en [0, 1) para cada punto entero (x, y, z) del espacio
pub fn hash3(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    lattice(x ^ (z.wrapping_mul(1_619)), y ^ (z.wrapping_mul(31_337)), seed)
}

// Curva suave 3t^2 - 2t^3: evita que se noten los bordes de la cuadricula
fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

// Value noise: valores al azar en las esquinas de la cuadricula, interpolados
// suavemente entre ellas. Devuelve [0, 1)
pub fn value_noise(x: f32, z: f32, seed: u32) -> f32 {
    let x0 = x.floor() as i32;
    let z0 = z.floor() as i32;
    let tx = smooth(x - x0 as f32);
    let tz = smooth(z - z0 as f32);

    let top = lerp(lattice(x0, z0, seed), lattice(x0 + 1, z0, seed), tx);
    let bottom = lerp(lattice(x0, z0 + 1, seed), lattice(x0 + 1, z0 + 1, seed), tx);
    lerp(top, bottom, tz)
}

// fBm (fractional Brownian motion): suma varias capas ("octavas") de ruido, cada
// una con el doble de frecuencia y la mitad de fuerza. Las primeras dan la forma
// general (colinas) y las siguientes el detalle. Devuelve [0, 1)
pub fn fbm(x: f32, z: f32, seed: u32, octaves: u32) -> f32 {
    let mut total = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut max_total = 0.0;

    for octave in 0..octaves {
        total += value_noise(x * frequency, z * frequency, seed.wrapping_add(octave)) * amplitude;
        max_total += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }

    total / max_total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn misma_semilla_misma_secuencia() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..10 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
        assert_ne!(Rng::new(1).next_u64(), Rng::new(2).next_u64());
    }

    #[test]
    fn next_f32_esta_en_rango() {
        let mut rng = Rng::new(7);
        for _ in 0..1000 {
            assert!((0.0..1.0).contains(&rng.next_f32()));
        }
    }

    #[test]
    fn value_noise_coincide_con_la_cuadricula_en_los_enteros() {
        assert_eq!(value_noise(3.0, -2.0, 5), lattice(3, -2, 5));
    }

    #[test]
    fn value_noise_es_continuo() {
        // Dos puntos muy cercanos deben dar valores muy cercanos
        let a = value_noise(1.4999, 2.3, 9);
        let b = value_noise(1.5001, 2.3, 9);
        assert!((a - b).abs() < 1e-3);
    }

    #[test]
    fn fbm_esta_en_rango() {
        for i in 0..200 {
            let value = fbm(i as f32 * 0.37, i as f32 * 0.11, 3, 4);
            assert!((0.0..1.0).contains(&value));
        }
    }
}
