use crate::cube::{axis_normal, face_uv, ray_box};
use crate::material::Material;
use crate::ray_intersect::{Intersect, RayIntersect};
use crate::vec3::Vec3;

// Cada celda guarda un numero: 0 es aire y los demas son el indice (+1) de su
// material. Un u8 por celda en lugar de un Material completo ocupa mucho menos
// memoria y cabe mejor en la cache del procesador
pub type BlockId = u8;
pub const AIR: BlockId = 0;

// Grid 3D de bloques de 1x1x1. La celda (x, y, z) ocupa el cubo que va de
// `origin + (x, y, z)` a `origin + (x+1, y+1, z+1)`
pub struct World {
    origin: [i32; 3],
    size: [usize; 3],
    cells: Vec<BlockId>,
    materials: Vec<Material>,
}

impl World {
    // `origin` es la esquina inferior del grid en coordenadas del mundo
    pub fn new(origin: [i32; 3], size: [usize; 3]) -> Self {
        World {
            origin,
            size,
            cells: vec![AIR; size[0] * size[1] * size[2]],
            materials: Vec::new(),
        }
    }

    // Registra un material y devuelve el id con el que se colocan sus bloques
    pub fn add_material(&mut self, material: Material) -> BlockId {
        self.materials.push(material);
        assert!(self.materials.len() <= BlockId::MAX as usize, "demasiados materiales");
        self.materials.len() as BlockId
    }

    // Posicion en el grid (0..size) de una coordenada del mundo, si esta adentro
    fn local(&self, x: i32, y: i32, z: i32) -> Option<[usize; 3]> {
        let local = [x - self.origin[0], y - self.origin[1], z - self.origin[2]];
        let inside = (0..3).all(|axis| local[axis] >= 0 && (local[axis] as usize) < self.size[axis]);
        inside.then(|| local.map(|value| value as usize))
    }

    fn index(&self, cell: [usize; 3]) -> usize {
        (cell[1] * self.size[2] + cell[2]) * self.size[0] + cell[0]
    }

    // Coloca un bloque en coordenadas del mundo; fuera del grid no hace nada
    pub fn set(&mut self, x: i32, y: i32, z: i32, block: BlockId) {
        if let Some(cell) = self.local(x, y, z) {
            let index = self.index(cell);
            self.cells[index] = block;
        }
    }

    // Bloque en coordenadas del mundo; fuera del grid es aire
    #[allow(dead_code)]
    pub fn get(&self, x: i32, y: i32, z: i32) -> BlockId {
        self.local(x, y, z).map_or(AIR, |cell| self.cells[self.index(cell)])
    }

    fn min_corner(&self) -> Vec3 {
        Vec3::new(self.origin[0] as f32, self.origin[1] as f32, self.origin[2] as f32)
    }

    fn max_corner(&self) -> Vec3 {
        self.min_corner() + Vec3::new(self.size[0] as f32, self.size[1] as f32, self.size[2] as f32)
    }
}

// Recorrido 3D-DDA (Amanatides & Woo): el rayo avanza de celda en celda, siempre a
// la siguiente pared que cruza, y se detiene en el primer bloque solido.
// Solo visita las celdas que atraviesa: el costo depende de la distancia recorrida,
// no de cuantos bloques tiene el mundo
impl RayIntersect for World {
    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Intersect {
        let min = self.min_corner();
        let Some(bounds) = ray_box(ray_origin, ray_direction, &min, &self.max_corner()) else {
            return Intersect::empty();
        };

        // Si el rayo nace dentro del grid (shadow rays) se empieza desde el origen
        let starts_inside = bounds.t_enter <= 0.0;
        let mut t = bounds.t_enter.max(0.0);

        let origin = [ray_origin.x - min.x, ray_origin.y - min.y, ray_origin.z - min.z];
        let direction = [ray_direction.x, ray_direction.y, ray_direction.z];

        let mut cell = [0i32; 3];
        let mut step = [0i32; 3];
        // t en el que el rayo cruza la siguiente pared de cada eje
        let mut t_max = [f32::INFINITY; 3];
        // Cuanto t cuesta cruzar una celda completa en cada eje
        let mut t_delta = [f32::INFINITY; 3];

        for axis in 0..3 {
            let position = origin[axis] + direction[axis] * t;
            // En la pared de salida del grid floor daria size: se queda en la ultima celda
            cell[axis] = (position.floor() as i32).clamp(0, self.size[axis] as i32 - 1);

            if direction[axis] > 0.0 {
                step[axis] = 1;
                t_max[axis] = (cell[axis] as f32 + 1.0 - origin[axis]) / direction[axis];
                t_delta[axis] = 1.0 / direction[axis];
            } else if direction[axis] < 0.0 {
                step[axis] = -1;
                t_max[axis] = (cell[axis] as f32 - origin[axis]) / direction[axis];
                t_delta[axis] = -1.0 / direction[axis];
            }
        }

        // Eje de la ultima pared cruzada: da la normal de la cara del impacto
        let mut axis = bounds.enter_axis;

        // Si el rayo nace dentro de un bloque (un shadow ray por redondeo, o un rayo
        // refractado dentro del agua), ese bloque es su "medio": se atraviesan las
        // celdas del mismo tipo, porque la pared entre dos bloques de agua no es una
        // superficie real, y se choca con el primer bloque distinto
        let first = self.cells[self.index(cell.map(|value| value as usize))];
        let mut medium = if starts_inside { first } else { AIR };

        loop {
            let block = self.cells[self.index(cell.map(|value| value as usize))];

            if block != medium {
                if block != AIR {
                    let material = self.materials[block as usize - 1];
                    let normal = axis_normal(axis, -step[axis] as f32);
                    let point = *ray_origin + *ray_direction * t;
                    let inside_block =
                        point - min - Vec3::new(cell[0] as f32, cell[1] as f32, cell[2] as f32);
                    let local_point = Vec3::new(
                        inside_block.x.clamp(0.0, 1.0),
                        inside_block.y.clamp(0.0, 1.0),
                        inside_block.z.clamp(0.0, 1.0),
                    );
                    let uv = face_uv(local_point, normal);
                    return Intersect::new(point, normal, t, uv, material);
                }
                // Salio del medio al aire: desde aqui cualquier bloque cuenta
                medium = AIR;
            }

            // Avanza por el eje cuya pared esta mas cerca
            axis = if t_max[0] < t_max[1] {
                if t_max[0] < t_max[2] { 0 } else { 2 }
            } else if t_max[1] < t_max[2] {
                1
            } else {
                2
            };

            t = t_max[axis];
            if t > bounds.t_exit {
                return Intersect::empty();
            }

            cell[axis] += step[axis];
            if cell[axis] < 0 || cell[axis] >= self.size[axis] as i32 {
                return Intersect::empty();
            }
            t_max[axis] += t_delta[axis];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;

    const TOLERANCE: f32 = 1e-4;

    // Piso de 4x4 en y = 0 y un bloque encima en (1, 1, 1)
    fn mundo() -> World {
        let mut world = World::new([-2, 0, -2], [4, 3, 4]);
        let material = Material::new(Color::new(1.0, 1.0, 1.0), 10.0, [1.0, 0.0, 0.0, 0.0], 1.0);
        let stone = world.add_material(material);
        for x in -2..2 {
            for z in -2..2 {
                world.set(x, 0, z, stone);
            }
        }
        world.set(1, 1, 1, stone);
        world
    }

    #[test]
    fn set_y_get_usan_coordenadas_del_mundo() {
        let world = mundo();
        assert_ne!(world.get(-2, 0, -2), AIR);
        assert_eq!(world.get(0, 1, 0), AIR);
        // Fuera del grid siempre es aire
        assert_eq!(world.get(10, 0, 0), AIR);
    }

    #[test]
    fn rayo_hacia_abajo_choca_con_el_piso() {
        let hit = mundo().ray_intersect(&Vec3::new(-0.5, 5.0, -0.5), &Vec3::new(0.0, -1.0, 0.0));
        assert!(hit.is_intersecting);
        assert!((hit.distance - 4.0).abs() < TOLERANCE);
        assert_eq!(hit.normal, Vec3::new(0.0, 1.0, 0.0));
        assert!((hit.uv.0 - 0.5).abs() < TOLERANCE && (hit.uv.1 - 0.5).abs() < TOLERANCE);
    }

    #[test]
    fn el_bloque_de_encima_tapa_al_piso() {
        let hit = mundo().ray_intersect(&Vec3::new(1.5, 5.0, 1.5), &Vec3::new(0.0, -1.0, 0.0));
        assert!((hit.distance - 3.0).abs() < TOLERANCE);
    }

    #[test]
    fn choca_de_lado_con_la_cara_correcta() {
        // Rayo horizontal a la altura del bloque de encima, viniendo desde -X
        let hit = mundo().ray_intersect(&Vec3::new(-5.0, 1.5, 1.5), &Vec3::new(1.0, 0.0, 0.0));
        assert!(hit.is_intersecting);
        assert!((hit.point.x - 1.0).abs() < TOLERANCE);
        assert_eq!(hit.normal, Vec3::new(-1.0, 0.0, 0.0));
    }

    #[test]
    fn en_la_arista_entre_dos_bloques_sale_la_cara_de_arriba() {
        // Baja en diagonal y pega justo en la union x = 0 entre dos bloques del piso
        let direction = Vec3::new(1.0, -1.0, 0.0).normalize();
        let hit = mundo().ray_intersect(&Vec3::new(-1.0, 2.0, -0.5), &direction);
        assert!(hit.is_intersecting);
        assert_eq!(hit.normal, Vec3::new(0.0, 1.0, 0.0));
    }

    #[test]
    fn rayo_que_no_pasa_por_el_grid_no_choca() {
        let hit = mundo().ray_intersect(&Vec3::new(0.0, 5.0, 0.0), &Vec3::new(0.0, 1.0, 0.0));
        assert!(!hit.is_intersecting);
    }

    #[test]
    fn rayo_dentro_del_agua_atraviesa_el_agua_y_choca_con_el_fondo() {
        let mut world = mundo();
        let material = Material::new(Color::new(0.0, 0.0, 1.0), 10.0, [1.0, 0.0, 0.0, 0.8], 1.33);
        let water = world.add_material(material);
        for x in -2..2 {
            world.set(x, 1, -2, water);
        }
        // Nace dentro del agua y va en diagonal hacia abajo, cruzando a otro bloque de agua
        let direction = Vec3::new(1.0, -0.3, 0.0).normalize();
        let hit = world.ray_intersect(&Vec3::new(-1.9, 1.8, -1.5), &direction);
        assert!(hit.is_intersecting);
        assert_ne!(hit.material.albedo[3], 0.8, "no debe chocar con el agua");
        assert_eq!(hit.normal, Vec3::new(0.0, 1.0, 0.0));
    }

    #[test]
    fn rayo_que_nace_adentro_sale_de_su_celda() {
        // Como un shadow ray que por redondeo quedo dentro del bloque del piso
        let hit = mundo().ray_intersect(&Vec3::new(-0.5, 0.999, -0.5), &Vec3::new(0.0, 1.0, 0.0));
        assert!(!hit.is_intersecting);
    }
}
