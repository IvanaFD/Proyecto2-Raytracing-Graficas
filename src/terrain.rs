// Generacion procedural de la isla flotante: colina en terrazas, parte de abajo de
// piedra que se angosta, estanque, arbol de sakura y portal con dos lamparas.
// Todo depende de la semilla: otra semilla da otra isla

use std::f32::consts::TAU;

use crate::color::Color;
use crate::material::Material;
use crate::noise::{Rng, fbm};
use crate::texture::textures;
use crate::world::{BlockId, World};

// El grid va de -16 a 16 en x y z, y de -14 a 26 en y
const HALF_SIZE: i32 = 16;
const WORLD_BOTTOM: i32 = -14;
const WORLD_HEIGHT: usize = 40;

// Radio promedio de la isla en bloques (el borde se deforma con ruido)
const ISLAND_RADIUS: f32 = 14.0;
// Altura de la cima de la colina sobre el borde de la isla
const HILL_HEIGHT: f32 = 8.0;
// Profundidad maxima de la parte de abajo de la isla
const UNDERSIDE_DEPTH: f32 = 13.0;
// Capas de tierra entre el pasto y la piedra
const DIRT_LAYERS: i32 = 3;

// Id de cada tipo de bloque en el mundo
struct Palette {
    grass: BlockId,
    dirt: BlockId,
    stone: BlockId,
    sand: BlockId,
    log: BlockId,
    planks: BlockId,
    cherry_leaves: BlockId,
    water: BlockId,
    obsidian: BlockId,
    portal: BlockId,
    lamp: BlockId,
    stone_bricks: BlockId,
}

impl Palette {
    // Material::new(color, exponente especular, [difuso, especular, reflectividad, transparencia], IOR)
    // El color base es el tono promedio del bloque; lo que se ve es la textura
    fn register(world: &mut World) -> Self {
        let t = textures();
        let mut add = |color: Color, specular: f32, albedo: [f32; 4], textures| {
            world.add_material(Material::new(color, specular, albedo, 1.0).textured(textures))
        };

        Palette {
            grass: add(Color::from_rgb(106, 170, 64), 10.0, [0.9, 0.1, 0.0, 0.0], &t.grass),
            dirt: add(Color::from_rgb(134, 96, 67), 5.0, [0.95, 0.05, 0.0, 0.0], &t.dirt),
            stone: add(Color::from_rgb(128, 128, 128), 30.0, [0.8, 0.25, 0.0, 0.0], &t.stone),
            sand: add(Color::from_rgb(219, 207, 163), 10.0, [0.9, 0.1, 0.0, 0.0], &t.sand),
            log: add(Color::from_rgb(102, 81, 49), 12.0, [0.85, 0.15, 0.0, 0.0], &t.log),
            planks: add(Color::from_rgb(162, 130, 78), 20.0, [0.8, 0.2, 0.0, 0.0], &t.planks),
            cherry_leaves: add(Color::from_rgb(242, 164, 204), 8.0, [0.9, 0.1, 0.0, 0.0], &t.cherry_leaves),
            // Agua: por ahora opaca; en las fases 14 y 15 se vuelve reflejante y transparente
            water: add(Color::from_rgb(52, 98, 205), 60.0, [0.6, 0.5, 0.0, 0.0], &t.water),
            obsidian: add(Color::from_rgb(22, 15, 34), 80.0, [0.6, 0.6, 0.0, 0.0], &t.obsidian),
            // Portal y lampara: en la fase 17 se vuelven emisivos
            portal: add(Color::from_rgb(128, 42, 214), 40.0, [0.8, 0.3, 0.0, 0.0], &t.portal),
            lamp: add(Color::from_rgb(255, 214, 120), 20.0, [0.9, 0.2, 0.0, 0.0], &t.lamp),
            stone_bricks: add(Color::from_rgb(122, 122, 122), 25.0, [0.8, 0.2, 0.0, 0.0], &t.stone_bricks),
        }
    }
}

// Una columna de la isla: de `bottom` (parte de abajo) a `surface` (el pasto)
#[derive(Clone, Copy)]
struct Column {
    surface: i32,
    bottom: i32,
    // Bloque de arriba distinto al pasto (arena del estanque, ladrillos de la plaza)
    top: Option<BlockId>,
}

// Mapa de alturas de la isla: una columna opcional por cada (x, z)
struct HeightMap {
    columns: Vec<Option<Column>>,
}

impl HeightMap {
    fn index(x: i32, z: i32) -> Option<usize> {
        let inside = (-HALF_SIZE..HALF_SIZE).contains(&x) && (-HALF_SIZE..HALF_SIZE).contains(&z);
        let width = (2 * HALF_SIZE) as usize;
        inside.then(|| (z + HALF_SIZE) as usize * width + (x + HALF_SIZE) as usize)
    }

    fn get(&self, x: i32, z: i32) -> Option<Column> {
        Self::index(x, z).and_then(|index| self.columns[index])
    }

    // Cambia la altura de una columna; si no existia (fuera de la isla) la crea
    fn set_surface(&mut self, x: i32, z: i32, surface: i32, top: Option<BlockId>) {
        let Some(index) = Self::index(x, z) else {
            return;
        };
        let bottom = self.columns[index].map_or(surface - 3, |column| column.bottom.min(surface - 1));
        self.columns[index] = Some(Column {
            surface,
            bottom,
            top,
        });
    }
}

// Forma de la isla: una colina en el centro que baja hacia el borde. Al redondear
// la altura a bloques enteros aparecen las terrazas
fn shape_island(noise_seed: u32) -> HeightMap {
    let width = (2 * HALF_SIZE) as usize;
    let mut map = HeightMap {
        columns: vec![None; width * width],
    };

    for z in -HALF_SIZE..HALF_SIZE {
        for x in -HALF_SIZE..HALF_SIZE {
            // Centro de la columna, medido desde el centro de la isla
            let (cx, cz) = (x as f32 + 0.5, z as f32 + 0.5);
            let distance = (cx * cx + cz * cz).sqrt();

            // El borde de la isla se mueve con ruido para que no sea un circulo perfecto
            let edge = ISLAND_RADIUS * (0.8 + 0.35 * fbm(cx * 0.12, cz * 0.12, noise_seed, 2));
            let r = distance / edge;
            if r >= 1.0 {
                continue;
            }

            let hill = (1.0 - r).powf(1.3) * HILL_HEIGHT;
            let bumps = fbm(cx * 0.1, cz * 0.1, noise_seed + 10, 3) * 3.0;
            let surface = (1.0 + hill + bumps).round() as i32;

            // Abajo la isla se angosta como un cono irregular: delgada en la orilla
            // y profunda en el centro
            let roughness = 0.6 + 0.8 * fbm(cx * 0.15, cz * 0.15, noise_seed + 20, 3);
            let depth = 1.0 + (1.0 - r).powf(1.2) * UNDERSIDE_DEPTH * roughness;

            if let Some(index) = HeightMap::index(x, z) {
                map.columns[index] = Some(Column {
                    surface,
                    bottom: -depth.round() as i32,
                    top: None,
                });
            }
        }
    }
    map
}

// Punto a `distance` bloques del centro en la direccion `angle`
fn around_center(angle: f32, distance: f32) -> (i32, i32) {
    (
        (angle.cos() * distance).round() as i32,
        (angle.sin() * distance).round() as i32,
    )
}

// Estanque de 5x3: se hunde un bloque y se llena de agua hasta el nivel del borde
fn carve_pond(map: &mut HeightMap, blocks: &Palette, angle: f32) -> Vec<(i32, i32, i32)> {
    let (px, pz) = around_center(angle, 9.0);
    let in_pond = |x: i32, z: i32| (px - 2..=px + 2).contains(&x) && (pz - 1..=pz + 1).contains(&z);

    let level = (px - 2..=px + 2)
        .flat_map(|x| (pz - 1..=pz + 1).map(move |z| (x, z)))
        .filter_map(|(x, z)| map.get(x, z).map(|column| column.surface))
        .min()
        .unwrap_or(1);

    let mut water = Vec::new();
    for x in px - 3..=px + 3 {
        for z in pz - 2..=pz + 2 {
            if in_pond(x, z) {
                // Fondo de arena y agua encima
                map.set_surface(x, z, level - 1, Some(blocks.sand));
                water.push((x, level, z));
            } else if map.get(x, z).is_none_or(|column| column.surface < level) {
                // El borde sube hasta el nivel del agua para que no se derrame
                map.set_surface(x, z, level, None);
            }
        }
    }
    water
}

// Plaza de ladrillos de 8x4 con el portal atras y un poste con lampara a cada lado
fn build_portal(map: &mut HeightMap, world: &mut World, blocks: &Palette, angle: f32) {
    let (qx, qz) = around_center(angle, 6.5);
    let floor = map.get(qx, qz).map_or(2, |column| column.surface);

    for x in qx - 4..=qx + 3 {
        for z in qz - 1..=qz + 2 {
            map.set_surface(x, z, floor, Some(blocks.stone_bricks));
        }
    }

    // Marco de obsidiana de 4x5 en la fila de atras, con el portal de 2x3 adentro
    let back = qz - 1;
    for x in qx - 2..=qx + 1 {
        for y in floor + 1..=floor + 5 {
            let is_frame = x == qx - 2 || x == qx + 1 || y == floor + 1 || y == floor + 5;
            let block = if is_frame { blocks.obsidian } else { blocks.portal };
            world.set(x, y, back, block);
        }
    }

    // Postes de tablas de 2 de alto con la lampara arriba
    for x in [qx - 4, qx + 3] {
        world.set(x, floor + 1, qz, blocks.planks);
        world.set(x, floor + 2, qz, blocks.planks);
        world.set(x, floor + 3, qz, blocks.lamp);
    }
}

// Arbol de sakura en la cima: tronco de 2x2 y una copa rosada ancha e irregular
fn grow_cherry_tree(map: &mut HeightMap, world: &mut World, blocks: &Palette, rng: &mut Rng) {
    let trunk = [(-1, -1), (0, -1), (-1, 0), (0, 0)];
    let ground = trunk
        .iter()
        .filter_map(|&(x, z)| map.get(x, z).map(|column| column.surface))
        .max()
        .unwrap_or(HILL_HEIGHT as i32);
    for &(x, z) in &trunk {
        map.set_surface(x, z, ground, None);
    }

    let trunk_top = ground + 7;
    let canopy_y = trunk_top + 1;
    let (radius, height) = (5.5_f32, 2.8_f32);

    for y in canopy_y - 2..=canopy_y + 3 {
        for x in -6..6 {
            for z in -6..6 {
                // Centro del bloque medido desde el centro del tronco (esquina 0, 0)
                let (fx, fz) = (x as f32 + 0.5, z as f32 + 0.5);
                let fy = (y - canopy_y) as f32;
                let d = (fx * fx + fz * fz) / (radius * radius) + (fy * fy) / (height * height);
                // Borde irregular: cada bloque de la orilla entra o no segun el azar
                if d < 1.0 - 0.3 * rng.next_f32() {
                    world.set(x, y, z, blocks.cherry_leaves);
                }
            }
        }
    }

    // El tronco va despues para reemplazar las hojas que caen en sus columnas
    for &(x, z) in &trunk {
        for y in ground + 1..=trunk_top {
            world.set(x, y, z, blocks.log);
        }
    }
}

pub fn generate(seed: u64) -> World {
    let mut world = World::new(
        [-HALF_SIZE, WORLD_BOTTOM, -HALF_SIZE],
        [(2 * HALF_SIZE) as usize, WORLD_HEIGHT, (2 * HALF_SIZE) as usize],
    );
    let blocks = Palette::register(&mut world);

    let mut rng = Rng::new(seed);
    let noise_seed = rng.next_u64() as u32;
    let mut map = shape_island(noise_seed);

    // El estanque y el portal quedan separados un tercio de vuelta alrededor de la colina
    let pond_angle = rng.next_f32() * TAU;
    let water = carve_pond(&mut map, &blocks, pond_angle);
    build_portal(&mut map, &mut world, &blocks, pond_angle + TAU / 3.0);
    grow_cherry_tree(&mut map, &mut world, &blocks, &mut rng);

    // Capas de cada columna: pasto arriba, tierra debajo y piedra hasta el fondo
    for z in -HALF_SIZE..HALF_SIZE {
        for x in -HALF_SIZE..HALF_SIZE {
            let Some(column) = map.get(x, z) else {
                continue;
            };
            for y in column.bottom..=column.surface {
                let block = if y == column.surface {
                    column.top.unwrap_or(blocks.grass)
                } else if y >= column.surface - DIRT_LAYERS {
                    blocks.dirt
                } else {
                    blocks.stone
                };
                world.set(x, y, z, block);
            }
        }
    }

    for (x, y, z) in water {
        world.set(x, y, z, blocks.water);
    }

    world
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::AIR;

    #[test]
    fn la_isla_cubre_al_menos_16x16_columnas() {
        let map = shape_island(1);
        let columns = map.columns.iter().filter(|column| column.is_some()).count();
        assert!(columns >= 16 * 16, "solo hay {columns} columnas");
    }

    #[test]
    fn misma_semilla_mismo_mundo_y_otra_semilla_otro() {
        let surface = |world: &World| -> Vec<BlockId> {
            (-HALF_SIZE..HALF_SIZE).map(|x| world.get(x, 3, 5)).collect()
        };
        assert!(surface(&generate(7)) == surface(&generate(7)));
        let differs = (1..5).any(|seed| surface(&generate(seed)) != surface(&generate(seed + 100)));
        assert!(differs);
    }

    #[test]
    fn el_centro_tiene_tronco_y_el_borde_es_aire() {
        let world = generate(3);
        let has_trunk = (0..20).any(|y| world.get(0, y, 0) != AIR && world.get(0, y + 1, 0) != AIR);
        assert!(has_trunk);
        assert_eq!(world.get(-16, 0, -16), AIR);
    }
}
