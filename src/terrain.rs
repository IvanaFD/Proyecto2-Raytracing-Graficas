// Generacion procedural de la isla flotante: colina en terrazas, parte de abajo de
// piedra que se angosta, estanque, arbol de sakura y portal con dos lamparas.
// Todo depende de la semilla: otra semilla da otra isla

use std::f32::consts::TAU;

use crate::color::Color;
use crate::light::Light;
use crate::material::Material;
use crate::noise::{Rng, fbm};
use crate::vec3::Vec3;
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
    waterfall: BlockId,
    blue_wool: BlockId,
    white_wool: BlockId,
}

impl Palette {
    // add(color, exponente especular, [difuso, especular, reflectividad, transparencia], IOR,
    //     emision, textura)
    // El color base es el tono promedio del bloque; lo que se ve es la textura
    fn register(world: &mut World) -> Self {
        let t = textures();
        let mut add = |color: Color, specular: f32, albedo: [f32; 4], ior: f32, emission: f32, textures| {
            let material = Material::new(color, specular, albedo, ior).emissive(emission);
            world.add_material(material.textured(textures))
        };

        Palette {
            grass: add(Color::from_rgb(106, 170, 64), 10.0, [0.9, 0.1, 0.0, 0.0], 1.0, 0.0, &t.grass),
            dirt: add(Color::from_rgb(134, 96, 67), 5.0, [0.95, 0.05, 0.0, 0.0], 1.0, 0.0, &t.dirt),
            stone: add(Color::from_rgb(128, 128, 128), 30.0, [0.8, 0.25, 0.0, 0.0], 1.0, 0.0, &t.stone),
            sand: add(Color::from_rgb(219, 207, 163), 10.0, [0.9, 0.1, 0.0, 0.0], 1.0, 0.0, &t.sand),
            log: add(Color::from_rgb(102, 81, 49), 12.0, [0.85, 0.15, 0.0, 0.0], 1.0, 0.0, &t.log),
            planks: add(Color::from_rgb(162, 130, 78), 20.0, [0.8, 0.2, 0.0, 0.0], 1.0, 0.0, &t.planks),
            cherry_leaves: add(Color::from_rgb(242, 164, 204), 8.0, [0.9, 0.1, 0.0, 0.0], 1.0, 0.0, &t.cherry_leaves),
            // Agua transparente (IOR 1.33): deja ver el fondo doblado y refleja mas de lado
            water: add(Color::from_rgb(52, 98, 205), 80.0, [0.45, 0.6, 0.1, 0.75], 1.33, 0.0, &t.water),
            // Obsidiana pulida: un reflejo leve que la hace ver como vidrio volcanico
            obsidian: add(Color::from_rgb(22, 15, 34), 80.0, [0.6, 0.6, 0.15, 0.0], 1.0, 0.0, &t.obsidian),
            // Portal y lampara: emisivos, brillan con luz propia
            portal: add(Color::from_rgb(128, 42, 214), 40.0, [0.8, 0.3, 0.0, 0.0], 1.0, 1.4, &t.portal),
            lamp: add(Color::from_rgb(255, 214, 120), 20.0, [0.9, 0.2, 0.0, 0.0], 1.0, 1.6, &t.lamp),
            stone_bricks: add(Color::from_rgb(122, 122, 122), 25.0, [0.8, 0.2, 0.0, 0.0], 1.0, 0.0, &t.stone_bricks),
            // Agua cayendo: con espuma, menos transparente que la del estanque
            waterfall: add(Color::from_rgb(120, 160, 230), 60.0, [0.6, 0.5, 0.1, 0.45], 1.33, 0.0, &t.waterfall),
            // Lana del toldo del puesto: mate
            blue_wool: add(Color::from_rgb(52, 84, 190), 4.0, [0.95, 0.05, 0.0, 0.0], 1.0, 0.0, &t.blue_wool),
            white_wool: add(Color::from_rgb(234, 236, 236), 4.0, [0.95, 0.05, 0.0, 0.0], 1.0, 0.0, &t.white_wool),
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
    // tan profunda como sus vecinas, para que no quede un hueco en la orilla
    fn set_surface(&mut self, x: i32, z: i32, surface: i32, top: Option<BlockId>) {
        let Some(index) = Self::index(x, z) else {
            return;
        };
        let bottom = match self.columns[index] {
            Some(column) => column.bottom.min(surface - 1),
            None => [(x - 1, z), (x + 1, z), (x, z - 1), (x, z + 1)]
                .iter()
                .filter_map(|&(nx, nz)| self.get(nx, nz).map(|column| column.bottom))
                .min()
                .unwrap_or(surface - 3)
                .min(surface - 1),
        };
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

// Bloque de agua que se coloca despues de llenar el terreno: (x, y, z, tipo)
type WaterBlock = (i32, i32, i32, BlockId);

// Hasta donde puede llegar el canal buscando la orilla de la isla
const MAX_CHANNEL_LENGTH: i32 = 12;
// Cuantos bloques cae la cascada por debajo de la orilla
const WATERFALL_DROP_BELOW_EDGE: i32 = 3;

// Estanque de 6x4 y 2 de profundidad, lleno de agua hasta el nivel del borde. El
// fondo mezcla arena y piedra para que se note como la refraccion lo deforma.
// Un canal de 2 de ancho lleva el agua hasta la orilla y ahi cae como cascada
fn carve_pond(map: &mut HeightMap, blocks: &Palette, angle: f32) -> Vec<WaterBlock> {
    let (px, pz) = around_center(angle, 8.0);
    let in_pond = |x: i32, z: i32| (px - 3..=px + 2).contains(&x) && (pz - 2..=pz + 1).contains(&z);

    // El nivel del agua es la altura promedio de la zona: en una ladera se hunde un
    // poco la parte alta y se levanta el borde de la parte baja
    let heights: Vec<i32> = (px - 3..=px + 2)
        .flat_map(|x| (pz - 2..=pz + 1).map(move |z| (x, z)))
        .filter_map(|(x, z)| map.get(x, z).map(|column| column.surface))
        .collect();
    let level = if heights.is_empty() {
        1
    } else {
        (heights.iter().sum::<i32>() as f32 / heights.len() as f32).round() as i32
    };

    let mut water = Vec::new();
    for x in px - 4..=px + 3 {
        for z in pz - 3..=pz + 2 {
            if in_pond(x, z) {
                let floor = if (x + z).rem_euclid(3) == 0 { blocks.stone } else { blocks.sand };
                map.set_surface(x, z, level - 2, Some(floor));
                water.push((x, level - 1, z, blocks.water));
                water.push((x, level, z, blocks.water));
            } else if map.get(x, z).is_none_or(|column| column.surface < level) {
                // El borde sube hasta el nivel del agua para que no se derrame
                map.set_surface(x, z, level, None);
            }
        }
    }

    // El canal sale del lado del estanque que mira hacia afuera de la isla, recto
    // por el eje mas cercano a esa direccion
    let (forward, side, start) = if angle.cos().abs() >= angle.sin().abs() {
        let dx = angle.cos().signum() as i32;
        let start_x = if dx < 0 { px - 4 } else { px + 3 };
        ((dx, 0), (0, 1), (start_x, pz - 1))
    } else {
        let dz = angle.sin().signum() as i32;
        let start_z = if dz < 0 { pz - 3 } else { pz + 2 };
        ((0, dz), (1, 0), (px - 1, start_z))
    };
    let cell = |step: i32, lane: i32| {
        (
            start.0 + forward.0 * step + side.0 * lane,
            start.1 + forward.1 * step + side.1 * lane,
        )
    };

    let mut edge_bottom = level - 3;
    for step in 0..MAX_CHANNEL_LENGTH {
        let lanes = [cell(step, 0), cell(step, 1)];

        // Se acabo la isla: aqui el agua cae por el costado
        if lanes.iter().any(|&(x, z)| map.get(x, z).is_none()) {
            let fall_bottom = (edge_bottom - WATERFALL_DROP_BELOW_EDGE).max(WORLD_BOTTOM);
            for (x, z) in lanes {
                for y in fall_bottom..=level {
                    water.push((x, y, z, blocks.waterfall));
                }
            }
            break;
        }

        for (x, z) in lanes {
            edge_bottom = map.get(x, z).map_or(edge_bottom, |column| column.bottom);
            map.set_surface(x, z, level - 1, Some(blocks.sand));
            water.push((x, level, z, blocks.water));
        }
        // Las orillas del canal suben hasta el nivel del agua
        for (x, z) in [cell(step, -1), cell(step, 2)] {
            if map.get(x, z).is_none_or(|column| column.surface < level) {
                map.set_surface(x, z, level, None);
            }
        }
    }
    water
}

// Puesto de mercado como el de la foto: postes de tronco, mostrador de tablas y un
// toldo de lana a rayas azules y blancas con techo de dos aguas
fn build_stall(map: &mut HeightMap, world: &mut World, blocks: &Palette, angle: f32) {
    let (sx, sz) = around_center(angle, 6.0);
    let floor = map.get(sx, sz).map_or(2, |column| column.surface);

    // Piso plano de tablas de 4x3
    for x in sx - 2..=sx + 1 {
        for z in sz - 1..=sz + 1 {
            map.set_surface(x, z, floor, Some(blocks.planks));
        }
    }

    // Postes en las cuatro esquinas
    for x in [sx - 2, sx + 1] {
        for z in [sz - 1, sz + 1] {
            for y in floor + 1..=floor + 3 {
                world.set(x, y, z, blocks.log);
            }
        }
    }

    // Mostrador al frente, entre los postes
    for x in sx - 1..=sx {
        world.set(x, floor + 1, sz + 1, blocks.planks);
    }

    // Toldo: cada capa mas angosta que la de abajo; rayas segun la columna x
    let stripe = |x: i32| if x.rem_euclid(2) == 0 { blocks.blue_wool } else { blocks.white_wool };
    for (layer, reach) in [(4, 2), (5, 1), (6, 0)] {
        for x in sx - 3..=sx + 2 {
            for z in sz - reach..=sz + reach {
                world.set(x, floor + layer, z, stripe(x));
            }
        }
    }
}

// Color e intensidad de la luz que emite cada tipo de bloque
const LAMP_LIGHT: Color = Color::new(1.0, 0.72, 0.38);
const LAMP_INTENSITY: f32 = 2.2;
const PORTAL_LIGHT: Color = Color::new(0.72, 0.32, 1.0);
const PORTAL_INTENSITY: f32 = 2.6;
// Media diagonal de un bloque: los shadow rays ignoran el bloque de la lampara
const BLOCK_RADIUS: f32 = 0.87;

// Plaza de ladrillos de 8x4 con el portal atras y un poste con lampara a cada lado.
// Devuelve las luces que emiten el portal y las lamparas
fn build_portal(map: &mut HeightMap, world: &mut World, blocks: &Palette, angle: f32) -> Vec<Light> {
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

    // La luz del portal va un poco enfrente de su cara, del lado de la plaza, para
    // que el propio portal no la tape
    let mut lights = vec![Light::point(
        Vec3::new(qx as f32, floor as f32 + 3.5, back as f32 + 1.3),
        PORTAL_LIGHT,
        PORTAL_INTENSITY,
        0.0,
    )];

    // Postes de tablas de 2 de alto con la lampara arriba; la luz sale del centro
    // de la lampara
    for x in [qx - 4, qx + 3] {
        world.set(x, floor + 1, qz, blocks.planks);
        world.set(x, floor + 2, qz, blocks.planks);
        world.set(x, floor + 3, qz, blocks.lamp);
        lights.push(Light::point(
            Vec3::new(x as f32 + 0.5, floor as f32 + 3.5, qz as f32 + 0.5),
            LAMP_LIGHT,
            LAMP_INTENSITY,
            BLOCK_RADIUS,
        ));
    }
    lights
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

// La isla generada: sus bloques y las luces de sus bloques emisivos
pub struct Island {
    pub world: World,
    pub lights: Vec<Light>,
}

pub fn generate(seed: u64) -> Island {
    let mut world = World::new(
        [-HALF_SIZE, WORLD_BOTTOM, -HALF_SIZE],
        [(2 * HALF_SIZE) as usize, WORLD_HEIGHT, (2 * HALF_SIZE) as usize],
    );
    let blocks = Palette::register(&mut world);

    let mut rng = Rng::new(seed);
    let noise_seed = rng.next_u64() as u32;
    let mut map = shape_island(noise_seed);

    // Alrededor de la colina: el estanque, el portal un tercio de vuelta despues y el
    // puesto del lado opuesto al portal, para que nada quede amontonado
    let pond_angle = rng.next_f32() * TAU;
    let portal_angle = pond_angle + TAU / 3.0;
    let water = carve_pond(&mut map, &blocks, pond_angle);
    let lights = build_portal(&mut map, &mut world, &blocks, portal_angle);
    build_stall(&mut map, &mut world, &blocks, portal_angle + TAU / 2.0);
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

    for (x, y, z, block) in water {
        world.set(x, y, z, block);
    }

    Island { world, lights }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::AIR;

    #[test]
    fn el_portal_y_las_dos_lamparas_emiten_luz() {
        let island = generate(11);
        assert_eq!(island.lights.len(), 3);
        // Cada luz esta pegada a un bloque emisivo
        for light in &island.lights {
            let p = light.position;
            let near_emissive = (-1..=1).any(|dz| {
                let block = island.world.get(p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32 + dz);
                block != AIR
            });
            assert!(near_emissive);
        }
    }

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
        let world = |seed| generate(seed).world;
        assert!(surface(&world(7)) == surface(&world(7)));
        let differs = (1..5).any(|seed| surface(&world(seed)) != surface(&world(seed + 100)));
        assert!(differs);
    }

    #[test]
    fn el_centro_tiene_tronco_y_el_borde_es_aire() {
        let world = generate(3).world;
        let has_trunk = (0..20).any(|y| world.get(0, y, 0) != AIR && world.get(0, y + 1, 0) != AIR);
        assert!(has_trunk);
        assert_eq!(world.get(-16, 0, -16), AIR);
    }
}
