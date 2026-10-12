// Construcciones de cada isla: arbol de sakura, cabana y estanque con cascada; piramide
// y cactus; hongos gigantes; pinos, lago congelado e iglu; y el portal en ruinas.
// Todas reciben el centro de su isla y trabajan en coordenadas del mundo

use std::f32::consts::TAU;

use crate::color::Color;
use crate::light::Light;
use crate::noise::Rng;
use crate::terrain::{HeightMap, Palette, WORLD_BOTTOM};
use crate::vec3::Vec3;
use crate::world::{AIR, BlockId, World};

// Bloque de agua (o hielo) que se coloca despues de llenar el terreno: (x, y, z, tipo)
pub type WaterBlock = (i32, i32, i32, BlockId);

// Color e intensidad de la luz que emite cada tipo de bloque
const LAMP_LIGHT: Color = Color::new(1.0, 0.72, 0.38);
const LAMP_INTENSITY: f32 = 2.2;
const PORTAL_LIGHT: Color = Color::new(0.72, 0.32, 1.0);
const PORTAL_INTENSITY: f32 = 2.6;
const MAGMA_LIGHT: Color = Color::new(1.0, 0.45, 0.15);
const MAGMA_INTENSITY: f32 = 1.5;
const SHROOM_LIGHT: Color = Color::new(1.0, 0.58, 0.24);
const SHROOM_INTENSITY: f32 = 1.8;
// Media diagonal de un bloque: los shadow rays ignoran el bloque de la lampara
const BLOCK_RADIUS: f32 = 0.87;

// Hasta donde puede llegar el canal buscando la orilla de la isla
const MAX_CHANNEL_LENGTH: i32 = 12;
// Cuantos bloques cae la cascada por debajo de la orilla
const WATERFALL_DROP_BELOW_EDGE: i32 = 3;

// Punto a `distance` bloques del centro de la isla en la direccion `angle`
fn around(center: (i32, i32), angle: f32, distance: f32) -> (i32, i32) {
    (
        center.0 + (angle.cos() * distance).round() as i32,
        center.1 + (angle.sin() * distance).round() as i32,
    )
}

// Eje (x o z) mas cercano a una direccion, para orientar construcciones
fn axis_toward(angle: f32) -> (i32, i32) {
    if angle.cos().abs() >= angle.sin().abs() {
        (angle.cos().signum() as i32, 0)
    } else {
        (0, angle.sin().signum() as i32)
    }
}

// Ejes locales de una construccion: la celda (a, b) esta `a` bloques de lado y `b`
// bloques hacia `forward` desde `origin`
fn local_frame(origin: (i32, i32), forward: (i32, i32)) -> impl Fn(i32, i32) -> (i32, i32) {
    let side = (forward.1.abs(), forward.0.abs());
    move |a, b| {
        (
            origin.0 + side.0 * a + forward.0 * b,
            origin.1 + side.1 * a + forward.1 * b,
        )
    }
}

fn surface_at(map: &HeightMap, x: i32, z: i32, fallback: i32) -> i32 {
    map.get(x, z).map_or(fallback, |column| column.surface)
}

// Luz de un bloque emisivo con centro en (x, y, z) de la celda
fn block_light(x: i32, y: f32, z: i32, color: Color, intensity: f32) -> Light {
    Light::point(
        Vec3::new(x as f32 + 0.5, y, z as f32 + 0.5),
        color,
        intensity,
        BLOCK_RADIUS,
    )
}

// Poste de 2 de alto (del bloque que se pida) con una lampara arriba. Devuelve la luz
fn lamp_post(world: &mut World, blocks: &Palette, post: BlockId, x: i32, floor: i32, z: i32) -> Light {
    world.set(x, floor + 1, z, post);
    world.set(x, floor + 2, z, post);
    world.set(x, floor + 3, z, blocks.lamp);
    block_light(x, floor as f32 + 3.5, z, LAMP_LIGHT, LAMP_INTENSITY)
}

// Farol: poste de tronco con lampara
fn lantern(world: &mut World, blocks: &Palette, x: i32, floor: i32, z: i32) -> Light {
    lamp_post(world, blocks, blocks.log, x, floor, z)
}

// Nivel de agua para una zona: la altura promedio. En una ladera se hunde un poco
// la parte alta y se levanta el borde de la parte baja
fn average_surface(map: &HeightMap, cells: &[(i32, i32)]) -> i32 {
    let heights: Vec<i32> = cells.iter().filter_map(|&(x, z)| map.get(x, z).map(|c| c.surface)).collect();
    if heights.is_empty() {
        return 1;
    }
    (heights.iter().sum::<i32>() as f32 / heights.len() as f32).round() as i32
}

// ---------- Isla de sakura ----------

// Estanque de 6x4 y 2 de profundidad, lleno de agua hasta el nivel del borde. El
// fondo mezcla arena y piedra para que se note como la refraccion lo deforma.
// Un canal de 2 de ancho lleva el agua hasta la orilla y ahi cae como cascada
pub fn carve_pond(map: &mut HeightMap, blocks: &Palette, center: (i32, i32), angle: f32) -> Vec<WaterBlock> {
    let (px, pz) = around(center, angle, 7.0);
    let in_pond = |x: i32, z: i32| (px - 3..=px + 2).contains(&x) && (pz - 2..=pz + 1).contains(&z);
    let cells: Vec<(i32, i32)> = (px - 3..=px + 2).flat_map(|x| (pz - 2..=pz + 1).map(move |z| (x, z))).collect();
    let level = average_surface(map, &cells);

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

    // El canal sale del lado del estanque que mira hacia afuera de la isla
    let forward = axis_toward(angle);
    let start = match forward {
        (dx, 0) if dx < 0 => (px - 4, pz - 1),
        (_, 0) => (px + 3, pz - 1),
        (_, dz) if dz < 0 => (px - 1, pz - 3),
        _ => (px - 1, pz + 2),
    };
    let cell = local_frame(start, forward);

    let mut edge_bottom = level - 3;
    for step in 0..MAX_CHANNEL_LENGTH {
        let lanes = [cell(0, step), cell(1, step)];

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
        for (x, z) in [cell(-1, step), cell(2, step)] {
            if map.get(x, z).is_none_or(|column| column.surface < level) {
                map.set_surface(x, z, level, None);
            }
        }
    }
    water
}

// Cabana de 5x5: esquinas de tronco, paredes de tablas, puerta al frente, ventanas de
// vidrio, techo de dos aguas de lana a rayas con chimenea, una lampara colgando
// adentro (de noche se ve por las ventanas) y dos faroles al frente.
// La puerta mira hacia afuera de la isla. Devuelve las luces
pub fn build_cabin(
    map: &mut HeightMap,
    world: &mut World,
    blocks: &Palette,
    center: (i32, i32),
    angle: f32,
) -> Vec<Light> {
    let origin = around(center, angle, 6.0);
    let floor = surface_at(map, origin.0, origin.1, 2);
    let at = local_frame(origin, axis_toward(angle));

    // Terreno plano alrededor y piso de tablas adentro
    for a in -3..=3 {
        for b in -3..=4 {
            let (x, z) = at(a, b);
            let inside = a.abs() <= 2 && b.abs() <= 2;
            map.set_surface(x, z, floor, inside.then_some(blocks.planks));
        }
    }

    // Paredes de 3 de alto, con troncos en las esquinas
    for a in -2..=2_i32 {
        for b in -2..=2_i32 {
            if a.abs() != 2 && b.abs() != 2 {
                continue;
            }
            let corner = a.abs() == 2 && b.abs() == 2;
            let (x, z) = at(a, b);
            for y in floor + 1..=floor + 3 {
                world.set(x, y, z, if corner { blocks.log } else { blocks.planks });
            }
        }
    }

    // Puerta al frente y ventanas de vidrio a media altura
    let (x, z) = at(0, 2);
    world.set(x, floor + 1, z, AIR);
    world.set(x, floor + 2, z, AIR);
    for (a, b) in [(-1, 2), (1, 2), (-2, 0), (2, 0), (0, -2)] {
        let (x, z) = at(a, b);
        world.set(x, floor + 2, z, blocks.glass);
    }

    // Techo de dos aguas que baja hacia los lados (`a`), con alero de 1 bloque.
    // Capa k: las orillas estan en a = +-(3 - k); las rayas van segun b
    let stripe = |b: i32| if b.rem_euclid(2) == 0 { blocks.blue_wool } else { blocks.white_wool };
    for k in 0..=3 {
        let y = floor + 4 + k;
        let edge = 3 - k;
        for b in -3..=3 {
            for a in [-edge, edge] {
                let (x, z) = at(a, b);
                world.set(x, y, z, stripe(b));
            }
            // Triangulo de tablas que cierra el techo adelante y atras
            if b.abs() == 2 {
                for a in -edge + 1..edge {
                    let (x, z) = at(a, b);
                    world.set(x, y, z, blocks.planks);
                }
            }
        }
    }

    // Chimenea de ladrillos que sale del techo
    let (x, z) = at(1, -1);
    for y in floor + 6..=floor + 8 {
        world.set(x, y, z, blocks.stone_bricks);
    }

    // Lampara colgando del techo, adentro
    let (x, z) = at(0, 0);
    world.set(x, floor + 3, z, blocks.lamp);
    let mut lights = vec![block_light(x, floor as f32 + 3.5, z, LAMP_LIGHT, LAMP_INTENSITY)];

    // Faroles a los lados de la entrada
    for a in [-3, 3] {
        let (x, z) = at(a, 4);
        lights.push(lantern(world, blocks, x, floor, z));
    }
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
pub fn grow_cherry_tree(
    map: &mut HeightMap,
    world: &mut World,
    blocks: &Palette,
    center: (i32, i32),
    rng: &mut Rng,
) {
    let (cx, cz) = center;
    let trunk = [(cx - 1, cz - 1), (cx, cz - 1), (cx - 1, cz), (cx, cz)];
    let ground = trunk.iter().map(|&(x, z)| surface_at(map, x, z, 6)).max().unwrap_or(6);
    for &(x, z) in &trunk {
        map.set_surface(x, z, ground, None);
    }

    let trunk_top = ground + 6;
    let top = trunk_top as f32;
    // Centro del tronco: la esquina entre sus 4 bloques
    let (ox, oz) = (cx as f32, cz as f32);

    // Copa principal encima del tronco y una copa al final de cada rama
    let mut blobs = vec![Blob {
        center: (ox, top + 3.0, oz),
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
            branches.push((ox + dx * reach * t, top + 0.8 * step as f32, oz + dz * reach * t));
        }
        blobs.push(Blob {
            center: (ox + dx * reach, top + 2.2 + rng.next_f32(), oz + dz * reach),
            radius: 3.0 + 0.6 * rng.next_f32(),
            height: 2.1,
        });
    }

    // Hojas: todo lo que cae dentro de algun bulto, con la orilla irregular
    for y in trunk_top - 1..=trunk_top + 7 {
        for x in cx - 9..cx + 9 {
            for z in cz - 9..cz + 9 {
                let (fx, fy, fz) = (x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5);
                let inside = blobs.iter().any(|blob| {
                    let (bx, by, bz) = blob.center;
                    let horizontal = ((fx - bx).powi(2) + (fz - bz).powi(2)) / blob.radius.powi(2);
                    let vertical = (fy - by).powi(2) / blob.height.powi(2);
                    horizontal + vertical < 1.0
                });
                if inside && rng.next_f32() > 0.08 && world.get(x, y, z) == AIR {
                    world.set(x, y, z, blocks.cherry_leaves);
                }
            }
        }
    }

    // Petalos colgando: debajo de algunas columnas de la copa cuelgan 1 a 3 hojas
    for x in cx - 9..cx + 9 {
        for z in cz - 9..cz + 9 {
            let lowest = (trunk_top - 1..=trunk_top + 7).find(|&y| world.get(x, y, z) == blocks.cherry_leaves);
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

// ---------- Isla del desierto ----------

// Piramide escalonada de arenisca de 7x7 en el centro, con una entrada, un glowstone
// en la punta como faro y dos pilares de arenisca con glowstone a los lados de la
// entrada. Devuelve las luces
pub fn build_pyramid(map: &mut HeightMap, world: &mut World, blocks: &Palette, center: (i32, i32)) -> Vec<Light> {
    let (cx, cz) = center;
    let floor = surface_at(map, cx, cz, 2);
    for x in cx - 4..=cx + 4 {
        for z in cz - 4..=cz + 6 {
            map.set_surface(x, z, floor, None);
        }
    }

    for level in 0..4 {
        let reach = 3 - level;
        for x in cx - reach..=cx + reach {
            for z in cz - reach..=cz + reach {
                world.set(x, floor + 1 + level, z, blocks.sandstone);
            }
        }
    }

    // Entrada de 1x2 en el frente
    world.set(cx, floor + 1, cz + 3, AIR);
    world.set(cx, floor + 2, cz + 3, AIR);

    // Glowstone en la punta
    world.set(cx, floor + 5, cz, blocks.lamp);
    let mut lights = vec![block_light(cx, floor as f32 + 5.5, cz, LAMP_LIGHT, LAMP_INTENSITY)];

    // Pilares a los lados de la entrada
    for x in [cx - 2, cx + 2] {
        lights.push(lamp_post(world, blocks, blocks.sandstone, x, floor, cz + 5));
    }
    lights
}

// Cactus de 2 a 4 de alto repartidos por la isla, sin tocar la piramide
pub fn scatter_cacti(map: &HeightMap, world: &mut World, blocks: &Palette, center: (i32, i32), rng: &mut Rng) {
    let mut planted = Vec::new();
    for _ in 0..40 {
        if planted.len() >= 9 {
            break;
        }
        let angle = rng.next_f32() * TAU;
        let distance = 6.0 + rng.next_f32() * 4.0;
        let (x, z) = around(center, angle, distance);
        let Some(column) = map.get(x, z) else {
            continue;
        };
        // Los cactus no se tocan entre si
        let crowded = planted.iter().any(|&(px, pz): &(i32, i32)| (px - x).abs() <= 1 && (pz - z).abs() <= 1);
        if crowded || column.top.is_some() || world.get(x, column.surface + 1, z) != AIR {
            continue;
        }
        let height = 2 + (rng.next_f32() * 3.0) as i32;
        for y in column.surface + 1..=column.surface + height {
            world.set(x, y, z, blocks.cactus);
        }
        planted.push((x, z));
    }
}

// ---------- Isla de hongos ----------

// Hongo rojo: tallo y un sombrero de 5x5 arriba con los lados que cuelgan
fn red_mushroom(world: &mut World, blocks: &Palette, x: i32, floor: i32, z: i32, height: i32) {
    let top = floor + height;
    for y in floor + 1..top {
        world.set(x, y, z, blocks.mushroom_stem);
    }
    for dx in -3..=3_i32 {
        for dz in -3..=3_i32 {
            let ring = dx.abs().max(dz.abs());
            let corner = dx.abs() == dz.abs() && ring >= 2;
            if ring <= 2 && !(corner && ring == 2) {
                world.set(x + dx, top, z + dz, blocks.red_cap);
            }
            if ring == 3 && !corner {
                world.set(x + dx, top - 1, z + dz, blocks.red_cap);
                world.set(x + dx, top - 2, z + dz, blocks.red_cap);
            }
        }
    }
}

// Hongo cafe: tallo alto y un sombrero plano y ancho de 7x7 sin esquinas
fn brown_mushroom(world: &mut World, blocks: &Palette, x: i32, floor: i32, z: i32, height: i32) {
    let top = floor + height;
    for y in floor + 1..top {
        world.set(x, y, z, blocks.mushroom_stem);
    }
    for dx in -3..=3_i32 {
        for dz in -3..=3_i32 {
            if !(dx.abs() == 3 && dz.abs() == 3) {
                world.set(x + dx, top, z + dz, blocks.brown_cap);
            }
        }
    }
}

// Hongos gigantes con shroomlight colgando del sombrero: en los rojos cuelga por
// debajo de la orilla (los lados del sombrero lo taparian si fuera adentro) y en el
// cafe, que es plano, justo debajo. Dos por hongo, a lados opuestos. Devuelve sus luces
pub fn grow_mushrooms(
    map: &mut HeightMap,
    world: &mut World,
    blocks: &Palette,
    center: (i32, i32),
    rng: &mut Rng,
) -> Vec<Light> {
    let first = rng.next_f32() * TAU;
    let spots = [(first, 3.5, true, 6), (first + 2.3, 6.5, true, 5), (first + 4.2, 5.5, false, 7)];
    let mut lights = Vec::new();
    for (angle, distance, red, height) in spots {
        let (x, z) = around(center, angle, distance);
        let floor = surface_at(map, x, z, 3);
        map.set_surface(x, z, floor, None);
        // (dx, dz, cuantos bloques abajo de la punta del sombrero)
        let hanging = if red {
            red_mushroom(world, blocks, x, floor, z, height);
            [(3, 0, 3), (-3, 0, 3)]
        } else {
            brown_mushroom(world, blocks, x, floor, z, height);
            [(2, 0, 1), (-2, 0, 1)]
        };

        for (dx, dz, drop) in hanging {
            let y = floor + height - drop;
            world.set(x + dx, y, z + dz, blocks.shroomlight);
            lights.push(block_light(x + dx, y as f32 + 0.5, z + dz, SHROOM_LIGHT, SHROOM_INTENSITY));
        }
    }
    lights
}

// ---------- Isla de nieve ----------

// Lago congelado de 5x4: piedra al fondo, agua y una capa de hielo encima
pub fn freeze_lake(map: &mut HeightMap, blocks: &Palette, center: (i32, i32), angle: f32) -> Vec<WaterBlock> {
    let (lx, lz) = around(center, angle, 6.0);
    let in_lake = |x: i32, z: i32| (lx - 2..=lx + 2).contains(&x) && (lz - 2..=lz + 1).contains(&z);
    let cells: Vec<(i32, i32)> = (lx - 2..=lx + 2).flat_map(|x| (lz - 2..=lz + 1).map(move |z| (x, z))).collect();
    let level = average_surface(map, &cells);

    let mut water = Vec::new();
    for x in lx - 3..=lx + 3 {
        for z in lz - 3..=lz + 2 {
            if in_lake(x, z) {
                map.set_surface(x, z, level - 2, Some(blocks.stone));
                water.push((x, level - 1, z, blocks.water));
                water.push((x, level, z, blocks.ice));
            } else if map.get(x, z).is_none_or(|column| column.surface < level) {
                map.set_surface(x, z, level, None);
            }
        }
    }
    water
}

// Iglu: domo hueco de nieve con la entrada hacia afuera de la isla y una lampara
// adentro que de noche ilumina la entrada. Devuelve la luz
pub fn build_igloo(
    map: &mut HeightMap,
    world: &mut World,
    blocks: &Palette,
    center: (i32, i32),
    angle: f32,
) -> Vec<Light> {
    let origin = around(center, angle, 5.0);
    let floor = surface_at(map, origin.0, origin.1, 3);
    let at = local_frame(origin, axis_toward(angle));

    for a in -4..=4 {
        for b in -4..=4 {
            let (x, z) = at(a, b);
            map.set_surface(x, z, floor, None);
        }
    }

    // Cascaron: los bloques cuya distancia al centro del domo cae entre 2.2 y 3.3
    for a in -3..=3_i32 {
        for b in -3..=3_i32 {
            for level in 0..=3 {
                let d = ((a * a + b * b) as f32 + (level as f32 + 0.5).powi(2)).sqrt();
                if (2.2..3.3).contains(&d) {
                    let (x, z) = at(a, b);
                    world.set(x, floor + 1 + level, z, blocks.snow);
                }
            }
        }
    }

    // Entrada de 1x2 al frente
    for b in 2..=3 {
        let (x, z) = at(0, b);
        world.set(x, floor + 1, z, AIR);
        world.set(x, floor + 2, z, AIR);
    }

    // Lampara en el piso, al fondo
    let (x, z) = at(0, -1);
    world.set(x, floor + 1, z, blocks.lamp);
    vec![block_light(x, floor as f32 + 1.5, z, LAMP_LIGHT, LAMP_INTENSITY)]
}

// Pino: tronco y capas de hojas que se angostan hacia arriba, alternando el ancho
fn spruce(world: &mut World, blocks: &Palette, x: i32, floor: i32, z: i32, height: i32) {
    let top = floor + height;
    // Radio de cada capa desde la punta hacia abajo
    let radii = [0.0_f32, 1.0, 1.0, 2.0, 1.4, 2.4, 1.8];
    for (i, radius) in radii.iter().enumerate() {
        let y = top + 1 - i as i32;
        if y <= floor + 2 {
            break;
        }
        let reach = radius.ceil() as i32;
        for dx in -reach..=reach {
            for dz in -reach..=reach {
                let inside = ((dx * dx + dz * dz) as f32) <= radius * radius + 0.5;
                if inside && world.get(x + dx, y, z + dz) == AIR {
                    world.set(x + dx, y, z + dz, blocks.spruce_leaves);
                }
            }
        }
    }
    for y in floor + 1..=top {
        world.set(x, y, z, blocks.spruce_log);
    }
}

// Pinos repartidos por la isla, lejos del lago y del iglu
pub fn grow_spruces(
    map: &HeightMap,
    world: &mut World,
    blocks: &Palette,
    center: (i32, i32),
    lake_angle: f32,
    rng: &mut Rng,
) {
    let mut planted = 0;
    for attempt in 0..30 {
        if planted >= 5 {
            break;
        }
        // Los pinos van a los lados (perpendicular al eje lago-iglu) y en el centro
        let side = if attempt % 2 == 0 { 1.0 } else { -1.0 };
        let angle = lake_angle + side * TAU / 4.0 + (rng.next_f32() - 0.5) * 1.2;
        let distance = 1.0 + rng.next_f32() * 8.0;
        let (x, z) = around(center, angle, distance);
        let Some(column) = map.get(x, z) else {
            continue;
        };
        let clear = (-2..=2).all(|dx| (-2..=2).all(|dz| world.get(x + dx, column.surface + 2, z + dz) == AIR));
        if column.top.is_some() || !clear {
            continue;
        }
        let height = 6 + (rng.next_f32() * 3.0) as i32;
        spruce(world, blocks, x, column.surface, z, height);
        planted += 1;
    }
}

// ---------- Isla del centro ----------

// Portal en ruinas como los de Minecraft, en una isla hecha solo de bloques del
// Nether: el marco de obsidiana queda completo (y el portal encendido) en el centro,
// con una plaza rota de ladrillos del Nether alrededor, magma que brilla en el suelo,
// restos tirados y manchas de piedra negra en el netherrack.
// Devuelve las luces del portal (una por cara) y del magma
pub fn build_portal_ruins(
    map: &mut HeightMap,
    world: &mut World,
    blocks: &Palette,
    center: (i32, i32),
    rng: &mut Rng,
) -> Vec<Light> {
    let floor = surface_at(map, center.0, center.1, 5);
    // El portal mira hacia +z y -z; `a` va a lo largo del marco
    let at = local_frame(center, (0, 1));

    // Manchas de piedra negra en el netherrack
    for x in center.0 - 8..=center.0 + 8 {
        for z in center.1 - 8..=center.1 + 8 {
            let Some(column) = map.get(x, z) else {
                continue;
            };
            if rng.next_f32() < 0.2 {
                map.set_surface(x, z, column.surface, Some(blocks.blackstone));
            }
        }
    }

    // Plaza rota de 8x7 a los dos lados del marco: ladrillos del Nether con huecos
    // de netherrack
    for a in -4..=3 {
        for b in -3..=3 {
            let (x, z) = at(a, b);
            let top = if rng.next_f32() < 0.3 { blocks.netherrack } else { blocks.nether_bricks };
            map.set_surface(x, z, floor, Some(top));
        }
    }

    // Magma en las cuatro esquinas de la plaza
    let mut lights = Vec::new();
    for (a, b) in [(-3, 2), (2, 2), (-3, -2), (2, -2)] {
        let (x, z) = at(a, b);
        map.set_surface(x, z, floor, Some(blocks.magma));
        lights.push(block_light(x, floor as f32 + 1.4, z, MAGMA_LIGHT, MAGMA_INTENSITY));
    }

    // Marco completo de obsidiana de 4x5 con el portal de 2x3 adentro
    for a in -2..=1 {
        for y in floor + 1..=floor + 5 {
            let is_frame = a == -2 || a == 1 || y == floor + 1 || y == floor + 5;
            let (x, z) = at(a, 0);
            world.set(x, y, z, if is_frame { blocks.obsidian } else { blocks.portal });
        }
    }

    // Restos de la ruina: un pilar roto de ladrillos del Nether y pedazos tirados
    let (x, z) = at(2, 0);
    world.set(x, floor + 1, z, blocks.nether_bricks);
    world.set(x, floor + 2, z, blocks.nether_bricks);
    world.set(x, floor + 3, z, blocks.blackstone);
    for (a, b, block) in [
        (3, 0, blocks.nether_bricks),
        (-3, 0, blocks.blackstone),
        (-4, 3, blocks.nether_bricks),
        (1, 3, blocks.netherrack),
        (3, -2, blocks.blackstone),
        (-2, -3, blocks.nether_bricks),
    ] {
        let (x, z) = at(a, b);
        world.set(x, floor + 1, z, block);
    }

    // Una luz frente a cada cara del portal, entre sus dos columnas
    let (x0, z0) = at(-1, 0);
    let (x1, z1) = at(0, 0);
    let middle_x = (x0 + x1) as f32 / 2.0 + 0.5;
    let middle_z = (z0 + z1) as f32 / 2.0 + 0.5;
    for side in [-1.3, 1.3] {
        lights.push(Light::point(
            Vec3::new(middle_x, floor as f32 + 3.5, middle_z + side),
            PORTAL_LIGHT,
            PORTAL_INTENSITY,
            0.0,
        ));
    }
    lights
}

// ---------- Puentes ----------

// Ancho del puente medido desde su linea central: tablas al centro y troncos en
// las orillas, como vigas
const BRIDGE_DECK: f32 = 0.75;
const BRIDGE_EDGE: f32 = 1.35;

// Puente de madera entre dos islas, en linea recta de centro a centro. Solo ocupa el
// hueco entre las dos orillas, y su altura pasa en escalones de la orilla de una isla
// a la de la otra, porque flotan a distinta altura
pub fn build_bridge(map: &HeightMap, world: &mut World, blocks: &Palette, from: (i32, i32), to: (i32, i32)) {
    let start = (from.0 as f32 + 0.5, from.1 as f32 + 0.5);
    let end = (to.0 as f32 + 0.5, to.1 as f32 + 0.5);
    let point = |t: f32| (start.0 + (end.0 - start.0) * t, start.1 + (end.1 - start.1) * t);
    let column_at = |t: f32| {
        let (x, z) = point(t);
        map.get(x.floor() as i32, z.floor() as i32)
    };

    // Se camina por la linea para encontrar donde termina la primera isla y donde
    // empieza la segunda
    const SAMPLES: usize = 400;
    let mut gap_start = None;
    let mut gap_end = None;
    let mut last_surface = 0;
    for i in 0..=SAMPLES {
        let t = i as f32 / SAMPLES as f32;
        match (column_at(t), gap_start) {
            (Some(column), None) => last_surface = column.surface,
            (None, None) => gap_start = Some((t, last_surface)),
            (Some(column), Some(_)) => {
                gap_end = Some((t, column.surface));
                break;
            }
            (None, Some(_)) => {}
        }
    }
    let (Some((t0, y0)), Some((t1, y1))) = (gap_start, gap_end) else {
        return;
    };

    let (a, b) = (point(t0), point(t1));
    let (dx, dz) = (b.0 - a.0, b.1 - a.1);
    let length = (dx * dx + dz * dz).sqrt();
    let (ux, uz) = (dx / length, dz / length);

    let min_x = a.0.min(b.0).floor() as i32 - 2;
    let max_x = a.0.max(b.0).ceil() as i32 + 2;
    let min_z = a.1.min(b.1).floor() as i32 - 2;
    let max_z = a.1.max(b.1).ceil() as i32 + 2;

    for x in min_x..=max_x {
        for z in min_z..=max_z {
            // Solo en el hueco entre islas
            if map.get(x, z).is_some() {
                continue;
            }
            // Que tan lejos queda la celda de la linea central y en que parte del puente
            let (cx, cz) = (x as f32 + 0.5 - a.0, z as f32 + 0.5 - a.1);
            let along = (cx * ux + cz * uz) / length;
            let across = (cx * uz - cz * ux).abs();
            if !(-0.05..=1.05).contains(&along) || across > BRIDGE_EDGE {
                continue;
            }
            let t = along.clamp(0.0, 1.0);
            let y = (y0 as f32 + (y1 - y0) as f32 * t).round() as i32;
            let block = if across <= BRIDGE_DECK { blocks.planks } else { blocks.log };
            world.set(x, y, z, block);
        }
    }
}
