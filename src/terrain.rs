// Generacion procedural de la isla flotante: colina en terrazas, parte de abajo de
// piedra que se angosta, estanque con cascada, arbol de sakura, portal en ruinas y
// un puesto de mercado con faroles.
// Todo depende de la semilla: otra semilla da otra isla

use std::f32::consts::TAU;

use crate::color::Color;
use crate::light::Light;
use crate::material::Material;
use crate::noise::{Rng, fbm};
use crate::vec3::Vec3;
use crate::texture::textures;
use crate::world::{AIR, BlockId, World};

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
    netherrack: BlockId,
    magma: BlockId,
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
            // Bloques del portal en ruinas: netherrack con relieve y magma que brilla
            netherrack: add(Color::from_rgb(112, 44, 44), 15.0, [0.85, 0.15, 0.0, 0.0], 1.0, 0.0, &t.netherrack),
            magma: add(Color::from_rgb(255, 128, 32), 20.0, [0.8, 0.2, 0.0, 0.0], 1.0, 1.5, &t.magma),
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

// Puesto de mercado como el de la foto: postes de tronco, mostrador de tablas, un
// toldo de lana a rayas azules y blancas con techo de dos aguas y dos faroles al
// frente. Devuelve las luces de los faroles
fn build_stall(map: &mut HeightMap, world: &mut World, blocks: &Palette, angle: f32) -> Vec<Light> {
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

    // Faroles a los lados del frente, fuera del toldo
    [sx - 3, sx + 2]
        .into_iter()
        .map(|x| {
            let z = sz + 3;
            map.set_surface(x, z, floor, None);
            lantern(world, blocks, x, floor, z)
        })
        .collect()
}

// Color e intensidad de la luz que emite cada tipo de bloque
const LAMP_LIGHT: Color = Color::new(1.0, 0.72, 0.38);
const LAMP_INTENSITY: f32 = 2.2;
const PORTAL_LIGHT: Color = Color::new(0.72, 0.32, 1.0);
const PORTAL_INTENSITY: f32 = 2.6;
const MAGMA_LIGHT: Color = Color::new(1.0, 0.45, 0.15);
const MAGMA_INTENSITY: f32 = 1.5;
// Media diagonal de un bloque: los shadow rays ignoran el bloque de la lampara
const BLOCK_RADIUS: f32 = 0.87;
// Hasta donde se extiende el netherrack alrededor del portal
const RUIN_RADIUS: f32 = 5.5;

// Poste de tronco de 2 de alto con una lampara arriba. Devuelve la luz de la lampara
fn lantern(world: &mut World, blocks: &Palette, x: i32, floor: i32, z: i32) -> Light {
    world.set(x, floor + 1, z, blocks.log);
    world.set(x, floor + 2, z, blocks.log);
    world.set(x, floor + 3, z, blocks.lamp);
    Light::point(
        Vec3::new(x as f32 + 0.5, floor as f32 + 3.5, z as f32 + 0.5),
        LAMP_LIGHT,
        LAMP_INTENSITY,
        BLOCK_RADIUS,
    )
}

// Portal en ruinas como los de Minecraft. El marco de obsidiana queda completo (y el
// portal encendido) mirando hacia afuera de la isla; la ruina queda de ese lado, para
// que se vea desde afuera del diorama: plaza rota, magma que brilla en el suelo,
// restos de ladrillos y netherrack, y netherrack que se extiende por el pasto.
// Devuelve las luces del portal y del magma
fn build_portal(
    map: &mut HeightMap,
    world: &mut World,
    blocks: &Palette,
    angle: f32,
    rng: &mut Rng,
) -> Vec<Light> {
    let (qx, qz) = around_center(angle, 6.0);
    let floor = map.get(qx, qz).map_or(2, |column| column.surface);

    // Ejes locales: `forward` apunta hacia afuera de la isla (por el eje mas cercano
    // a esa direccion) y `side` va a lo largo del marco. La celda local (a, b) esta
    // `a` bloques de lado y `b` bloques hacia afuera del centro del portal
    let forward = if angle.cos().abs() >= angle.sin().abs() {
        (angle.cos().signum() as i32, 0)
    } else {
        (0, angle.sin().signum() as i32)
    };
    let side = (forward.1.abs(), forward.0.abs());
    let at = |a: i32, b: i32| (qx + side.0 * a + forward.0 * b, qz + side.1 * a + forward.1 * b);

    // El netherrack se extiende por el pasto del lado de afuera, mas denso cerca de la plaza
    let (rx, rz) = at(0, 2);
    for x in rx - 6..=rx + 6 {
        for z in rz - 6..=rz + 6 {
            let (dx, dz) = (x - rx, z - rz);
            let outward = dx * forward.0 + dz * forward.1;
            let distance = ((dx * dx + dz * dz) as f32).sqrt();
            let chance = 0.85 * (1.0 - distance / RUIN_RADIUS);
            let Some(column) = map.get(x, z) else {
                continue;
            };
            if outward >= -1 && column.top.is_none() && rng.next_f32() < chance {
                map.set_surface(x, z, column.surface, Some(blocks.netherrack));
            }
        }
    }

    // Plaza rota de 8x4 que sale del marco hacia afuera: ladrillos de piedra con
    // pedazos de netherrack
    for a in -4..=3 {
        for b in 0..=3 {
            let (x, z) = at(a, b);
            let top = if rng.next_f32() < 0.3 { blocks.netherrack } else { blocks.stone_bricks };
            map.set_surface(x, z, floor, Some(top));
        }
    }

    // Magma en el suelo de la plaza; cada bloque ilumina desde un poco por encima de el
    let mut lights = Vec::new();
    for (a, b) in [(-3, 2), (2, 2), (-1, 3), (3, 3)] {
        let (x, z) = at(a, b);
        map.set_surface(x, z, floor, Some(blocks.magma));
        lights.push(Light::point(
            Vec3::new(x as f32 + 0.5, floor as f32 + 1.4, z as f32 + 0.5),
            MAGMA_LIGHT,
            MAGMA_INTENSITY,
            BLOCK_RADIUS,
        ));
    }

    // Marco completo de obsidiana de 4x5 con el portal de 2x3 adentro
    for a in -2..=1 {
        for y in floor + 1..=floor + 5 {
            let is_frame = a == -2 || a == 1 || y == floor + 1 || y == floor + 5;
            let (x, z) = at(a, 0);
            world.set(x, y, z, if is_frame { blocks.obsidian } else { blocks.portal });
        }
    }

    // Restos de la ruina (la obsidiana es solo del marco): un pilar roto de ladrillos
    // junto al marco, con netherrack arriba, y pedazos tirados en la plaza
    let (x, z) = at(2, 0);
    world.set(x, floor + 1, z, blocks.stone_bricks);
    world.set(x, floor + 2, z, blocks.stone_bricks);
    world.set(x, floor + 3, z, blocks.netherrack);
    for (a, b, block) in [
        (3, 0, blocks.stone_bricks),
        (-3, 0, blocks.netherrack),
        (-4, 3, blocks.stone_bricks),
        (1, 3, blocks.netherrack),
        (3, 1, blocks.netherrack),
    ] {
        let (x, z) = at(a, b);
        world.set(x, floor + 1, z, block);
    }

    // La luz del portal va enfrente de su cara de afuera, entre sus dos columnas
    let (x0, z0) = at(-1, 0);
    let (x1, z1) = at(0, 0);
    let center_x = (x0 + x1) as f32 / 2.0 + 0.5 + forward.0 as f32 * 1.3;
    let center_z = (z0 + z1) as f32 / 2.0 + 0.5 + forward.1 as f32 * 1.3;
    lights.push(Light::point(
        Vec3::new(center_x, floor as f32 + 3.5, center_z),
        PORTAL_LIGHT,
        PORTAL_INTENSITY,
        0.0,
    ));
    lights
}

// Un bulto redondeado de hojas de la copa
struct Blob {
    center: (f32, f32, f32),
    radius: f32,
    height: f32,
}

// Arbol de sakura en la cima: tronco de 2x2, cuatro ramas que salen hacia los lados
// y varias copas redondas encimadas, con petalos colgando por debajo
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

    let trunk_top = ground + 6;
    let top = trunk_top as f32;

    // Copa principal encima del tronco y una copa al final de cada rama
    let mut blobs = vec![Blob {
        center: (0.0, top + 3.0, 0.0),
        radius: 4.5,
        height: 2.8,
    }];
    let mut branches = Vec::new();
    let first_angle = rng.next_f32() * TAU;
    for branch in 0..4 {
        let angle = first_angle + branch as f32 * TAU / 4.0 + (rng.next_f32() - 0.5) * 0.6;
        let (dx, dz) = (angle.cos(), angle.sin());
        let reach = 3.8 + rng.next_f32();

        // La rama sube en diagonal desde la punta del tronco
        for step in 1..=3 {
            let t = step as f32 / 3.0;
            branches.push((dx * reach * t, top + 0.8 * step as f32, dz * reach * t));
        }
        blobs.push(Blob {
            center: (dx * reach, top + 2.2 + rng.next_f32(), dz * reach),
            radius: 3.0 + 0.6 * rng.next_f32(),
            height: 2.1,
        });
    }

    // Hojas: todo lo que cae dentro de algun bulto, con la orilla irregular
    for y in trunk_top - 1..=trunk_top + 7 {
        for x in -9..9 {
            for z in -9..9 {
                // Centro del bloque medido desde el centro del tronco (esquina 0, 0)
                let (fx, fy, fz) = (x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5);
                let inside = blobs.iter().any(|blob| {
                    let (cx, cy, cz) = blob.center;
                    let horizontal = ((fx - cx).powi(2) + (fz - cz).powi(2)) / blob.radius.powi(2);
                    let vertical = (fy - cy).powi(2) / blob.height.powi(2);
                    horizontal + vertical < 1.0
                });
                if inside && rng.next_f32() > 0.08 && world.get(x, y, z) == AIR {
                    world.set(x, y, z, blocks.cherry_leaves);
                }
            }
        }
    }

    // Petalos colgando: debajo de algunas columnas de la copa cuelgan 1 a 3 hojas
    for x in -9..9 {
        for z in -9..9 {
            let lowest = (trunk_top - 1..=trunk_top + 7)
                .find(|&y| world.get(x, y, z) == blocks.cherry_leaves);
            let Some(lowest) = lowest else {
                continue;
            };
            if rng.next_f32() < 0.3 {
                let length = 1 + (rng.next_f32() * 3.0) as i32;
                for y in lowest - length..lowest {
                    if world.get(x, y, z) == AIR {
                        world.set(x, y, z, blocks.cherry_leaves);
                    }
                }
            }
        }
    }

    // Tronco y ramas al final, para reemplazar las hojas que caen donde van
    for &(x, z) in &trunk {
        for y in ground + 1..=trunk_top {
            world.set(x, y, z, blocks.log);
        }
    }
    for (x, y, z) in branches {
        world.set(x.floor() as i32, y.floor() as i32, z.floor() as i32, blocks.log);
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
    let mut lights = build_portal(&mut map, &mut world, &blocks, portal_angle, &mut rng);
    lights.extend(build_stall(&mut map, &mut world, &blocks, portal_angle + TAU / 2.0));
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

    #[test]
    fn el_portal_el_magma_y_los_faroles_emiten_luz() {
        let island = generate(11);
        // Portal, 4 bloques de magma y 2 faroles
        assert_eq!(island.lights.len(), 7);
        // Cada luz esta pegada a un bloque (el que la emite)
        for light in &island.lights {
            let p = light.position;
            let (x, y, z) = (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
            let near_emissive = (-1..=1).any(|dx| {
                (-1..=1).any(|dy| (-1..=1).any(|dz| island.world.get(x + dx, y + dy, z + dz) != AIR))
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
