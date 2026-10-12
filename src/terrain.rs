// Generacion procedural del archipielago: cuatro islas flotantes de distinto bioma
// (sakura, desierto, hongos y nieve) alrededor de una isla pequena con un portal del
// Nether en ruinas. Cada isla es una colina en terrazas con la parte de abajo en cono.
// Todo depende de la semilla: otra semilla da otras islas

use std::f32::consts::TAU;

use crate::color::Color;
use crate::light::Light;
use crate::material::Material;
use crate::noise::{Rng, fbm};
use crate::structures;
use crate::texture::textures;
use crate::vec3::Vec3;
use crate::world::{BlockId, World};

// El grid va de -34 a 34 en x y z, y de -18 a 30 en y
pub const HALF_SIZE: i32 = 34;
pub const WORLD_BOTTOM: i32 = -18;
const WORLD_HEIGHT: usize = 48;

// Distancia (en x y en z) del centro del archipielago a cada isla de bioma
const ISLAND_OFFSET: i32 = 21;

// Id de cada tipo de bloque en el mundo
pub struct Palette {
    pub grass: BlockId,
    pub dirt: BlockId,
    pub stone: BlockId,
    pub sand: BlockId,
    pub log: BlockId,
    pub planks: BlockId,
    pub cherry_leaves: BlockId,
    pub water: BlockId,
    pub obsidian: BlockId,
    pub portal: BlockId,
    pub lamp: BlockId,
    pub stone_bricks: BlockId,
    pub waterfall: BlockId,
    pub blue_wool: BlockId,
    pub white_wool: BlockId,
    pub netherrack: BlockId,
    pub magma: BlockId,
    pub glass: BlockId,
    pub sandstone: BlockId,
    pub cactus: BlockId,
    pub mycelium: BlockId,
    pub mushroom_stem: BlockId,
    pub red_cap: BlockId,
    pub brown_cap: BlockId,
    pub snowy_grass: BlockId,
    pub snow: BlockId,
    pub ice: BlockId,
    pub spruce_log: BlockId,
    pub spruce_leaves: BlockId,
    pub nether_bricks: BlockId,
    pub blackstone: BlockId,
    pub shroomlight: BlockId,
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
            // Lana del techo de la cabana: mate
            blue_wool: add(Color::from_rgb(52, 84, 190), 4.0, [0.95, 0.05, 0.0, 0.0], 1.0, 0.0, &t.blue_wool),
            white_wool: add(Color::from_rgb(234, 236, 236), 4.0, [0.95, 0.05, 0.0, 0.0], 1.0, 0.0, &t.white_wool),
            // Bloques del portal en ruinas: netherrack con relieve y magma que brilla
            netherrack: add(Color::from_rgb(112, 44, 44), 15.0, [0.85, 0.15, 0.0, 0.0], 1.0, 0.0, &t.netherrack),
            magma: add(Color::from_rgb(255, 128, 32), 20.0, [0.8, 0.2, 0.0, 0.0], 1.0, 1.5, &t.magma),
            // Vidrio de las ventanas (IOR 1.5): casi transparente, dobla lo que se ve detras
            glass: add(Color::from_rgb(200, 225, 235), 120.0, [0.1, 0.6, 0.08, 0.85], 1.5, 0.0, &t.glass),
            // Desierto
            sandstone: add(Color::from_rgb(216, 200, 150), 15.0, [0.85, 0.15, 0.0, 0.0], 1.0, 0.0, &t.sandstone),
            cactus: add(Color::from_rgb(70, 130, 46), 10.0, [0.9, 0.1, 0.0, 0.0], 1.0, 0.0, &t.cactus),
            // Hongos
            mycelium: add(Color::from_rgb(120, 98, 116), 6.0, [0.9, 0.1, 0.0, 0.0], 1.0, 0.0, &t.mycelium),
            mushroom_stem: add(Color::from_rgb(214, 206, 188), 10.0, [0.9, 0.1, 0.0, 0.0], 1.0, 0.0, &t.mushroom_stem),
            red_cap: add(Color::from_rgb(196, 40, 36), 25.0, [0.85, 0.2, 0.0, 0.0], 1.0, 0.0, &t.red_cap),
            brown_cap: add(Color::from_rgb(150, 110, 80), 10.0, [0.9, 0.1, 0.0, 0.0], 1.0, 0.0, &t.brown_cap),
            // Nieve
            snowy_grass: add(Color::from_rgb(240, 246, 252), 20.0, [0.85, 0.2, 0.0, 0.0], 1.0, 0.0, &t.snowy_grass),
            snow: add(Color::from_rgb(240, 246, 252), 20.0, [0.85, 0.2, 0.0, 0.0], 1.0, 0.0, &t.snow),
            // Hielo (IOR 1.31): transparente y brillante, deja ver el agua de abajo
            ice: add(Color::from_rgb(150, 190, 245), 90.0, [0.35, 0.6, 0.12, 0.55], 1.31, 0.0, &t.ice),
            spruce_log: add(Color::from_rgb(70, 50, 30), 12.0, [0.85, 0.15, 0.0, 0.0], 1.0, 0.0, &t.spruce_log),
            spruce_leaves: add(Color::from_rgb(46, 84, 52), 8.0, [0.9, 0.1, 0.0, 0.0], 1.0, 0.0, &t.spruce_leaves),
            // Nether (isla del portal)
            nether_bricks: add(Color::from_rgb(54, 25, 28), 30.0, [0.8, 0.25, 0.0, 0.0], 1.0, 0.0, &t.nether_bricks),
            blackstone: add(Color::from_rgb(38, 35, 43), 35.0, [0.8, 0.25, 0.0, 0.0], 1.0, 0.0, &t.blackstone),
            // Shroomlight: el bloque de luz de los hongos, emisivo
            shroomlight: add(Color::from_rgb(250, 168, 66), 15.0, [0.85, 0.15, 0.0, 0.0], 1.0, 1.5, &t.shroomlight),
        }
    }
}

// Bloques de cada capa de una columna segun su bioma
#[derive(Clone, Copy)]
pub struct Layers {
    // Lo de arriba (pasto, arena, micelio, nieve)
    pub top: BlockId,
    // Las capas de abajo (tierra, arenisca)
    pub filler: BlockId,
    pub filler_depth: i32,
    // Lo mas profundo (piedra)
    pub deep: BlockId,
}

// Una columna de una isla: de `bottom` (parte de abajo) a `surface` (lo de arriba)
#[derive(Clone, Copy)]
pub struct Column {
    pub surface: i32,
    pub bottom: i32,
    // Bloque de arriba distinto al del bioma (arena del estanque, ladrillos de la plaza)
    pub top: Option<BlockId>,
    pub layers: Layers,
}

// Mapa de alturas del archipielago: una columna opcional por cada (x, z)
pub struct HeightMap {
    columns: Vec<Option<Column>>,
}

impl HeightMap {
    fn new() -> Self {
        let width = (2 * HALF_SIZE) as usize;
        HeightMap {
            columns: vec![None; width * width],
        }
    }

    fn index(x: i32, z: i32) -> Option<usize> {
        let inside = (-HALF_SIZE..HALF_SIZE).contains(&x) && (-HALF_SIZE..HALF_SIZE).contains(&z);
        let width = (2 * HALF_SIZE) as usize;
        inside.then(|| (z + HALF_SIZE) as usize * width + (x + HALF_SIZE) as usize)
    }

    pub fn get(&self, x: i32, z: i32) -> Option<Column> {
        Self::index(x, z).and_then(|index| self.columns[index])
    }

    // Cambia la altura de una columna; si no existia (fuera de la isla) la crea igual
    // que sus vecinas, y tan profunda como ellas para que no quede un hueco en la orilla
    pub fn set_surface(&mut self, x: i32, z: i32, surface: i32, top: Option<BlockId>) {
        let Some(index) = Self::index(x, z) else {
            return;
        };
        let column = match self.columns[index] {
            Some(column) => Column {
                surface,
                bottom: column.bottom.min(surface - 1),
                top,
                layers: column.layers,
            },
            None => {
                let neighbors: Vec<Column> = [(x - 1, z), (x + 1, z), (x, z - 1), (x, z + 1)]
                    .iter()
                    .filter_map(|&(nx, nz)| self.get(nx, nz))
                    .collect();
                // Sin vecinas no se sabe de que bioma es: no se crea
                let Some(first) = neighbors.first() else {
                    return;
                };
                let bottom = neighbors.iter().map(|column| column.bottom).min().unwrap_or(surface - 3);
                Column {
                    surface,
                    bottom: bottom.min(surface - 1),
                    top,
                    layers: first.layers,
                }
            }
        };
        self.columns[index] = Some(column);
    }
}

// Forma y bioma de una isla
struct IslandShape {
    center: (i32, i32),
    // Radio promedio en bloques (el borde se deforma con ruido)
    radius: f32,
    // Altura de la cima sobre la orilla
    hill: f32,
    // Profundidad maxima de la parte de abajo
    underside: f32,
    // Que tan arriba o abajo flota respecto a las demas
    base: i32,
    layers: Layers,
}

// Una colina en el centro que baja hacia el borde. Al redondear la altura a bloques
// enteros aparecen las terrazas. Abajo la isla se angosta como un cono irregular
fn shape_island(map: &mut HeightMap, shape: &IslandShape, noise_seed: u32) {
    let (cx, cz) = shape.center;
    let reach = (shape.radius * 1.2).ceil() as i32;

    for z in cz - reach..=cz + reach {
        for x in cx - reach..=cx + reach {
            // Centro de la columna, medido desde el centro de la isla
            let (dx, dz) = (x as f32 + 0.5 - cx as f32, z as f32 + 0.5 - cz as f32);
            let distance = (dx * dx + dz * dz).sqrt();

            // El borde se mueve con ruido para que no sea un circulo perfecto
            let edge = shape.radius * (0.8 + 0.35 * fbm(dx * 0.12, dz * 0.12, noise_seed, 2));
            let r = distance / edge;
            if r >= 1.0 {
                continue;
            }

            let hill = (1.0 - r).powf(1.3) * shape.hill;
            let bumps = fbm(dx * 0.1, dz * 0.1, noise_seed + 10, 3) * 3.0;
            let surface = shape.base + (1.0 + hill + bumps).round() as i32;

            // Delgada en la orilla y profunda en el centro
            let roughness = 0.6 + 0.8 * fbm(dx * 0.15, dz * 0.15, noise_seed + 20, 3);
            let depth = 1.0 + (1.0 - r).powf(1.2) * shape.underside * roughness;

            if let Some(index) = HeightMap::index(x, z) {
                map.columns[index] = Some(Column {
                    surface,
                    bottom: shape.base - depth.round() as i32,
                    top: None,
                    layers: shape.layers,
                });
            }
        }
    }
}

// Un lugar al que la camara puede enfocar (teclas 1 a 5) y su zoom inicial
pub struct Focus {
    pub name: &'static str,
    pub center: Vec3,
    pub radius: f32,
}

const OFFSET: f32 = ISLAND_OFFSET as f32;

// Islas en orden: sakura, desierto, hongos, nieve y la del portal en el centro
pub const FOCUS_POINTS: [Focus; 5] = [
    Focus { name: "Isla de sakura", center: Vec3::new(OFFSET, 6.0, OFFSET), radius: 34.0 },
    Focus { name: "Isla del desierto", center: Vec3::new(-OFFSET, 3.0, OFFSET), radius: 32.0 },
    Focus { name: "Isla de hongos", center: Vec3::new(-OFFSET, 6.0, -OFFSET), radius: 32.0 },
    Focus { name: "Isla de nieve", center: Vec3::new(OFFSET, 7.0, -OFFSET), radius: 32.0 },
    Focus { name: "Portal en ruinas", center: Vec3::new(0.0, 8.0, 0.0), radius: 22.0 },
];

// El archipielago generado: sus bloques y las luces de sus bloques emisivos
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
    let mut map = HeightMap::new();

    let layers = |top, filler, filler_depth| Layers {
        top,
        filler,
        filler_depth,
        deep: blocks.stone,
    };
    let offset = ISLAND_OFFSET;
    let cherry = (offset, offset);
    let desert = (-offset, offset);
    let mushroom = (-offset, -offset);
    let snow = (offset, -offset);
    let middle = (0, 0);

    let shapes = [
        IslandShape {
            center: cherry,
            radius: 11.5,
            hill: 7.0,
            underside: 12.0,
            base: 0,
            layers: layers(blocks.grass, blocks.dirt, 3),
        },
        IslandShape {
            center: desert,
            radius: 11.0,
            hill: 3.5,
            underside: 10.0,
            base: -2,
            layers: layers(blocks.sand, blocks.sandstone, 5),
        },
        IslandShape {
            center: mushroom,
            radius: 11.0,
            hill: 5.0,
            underside: 11.0,
            base: 1,
            layers: layers(blocks.mycelium, blocks.dirt, 3),
        },
        IslandShape {
            center: snow,
            radius: 11.0,
            hill: 6.0,
            underside: 12.0,
            base: 2,
            layers: layers(blocks.snowy_grass, blocks.dirt, 3),
        },
        // La isla del portal es pequena, casi plana, flota mas alto que las demas y
        // es toda del Nether: netherrack y piedra negra abajo
        IslandShape {
            center: middle,
            radius: 6.5,
            hill: 1.5,
            underside: 7.0,
            base: 4,
            layers: Layers {
                top: blocks.netherrack,
                filler: blocks.netherrack,
                filler_depth: 3,
                deep: blocks.blackstone,
            },
        },
    ];
    for shape in &shapes {
        shape_island(&mut map, shape, rng.next_u64() as u32);
    }

    let mut water = Vec::new();
    let mut lights = Vec::new();

    // Sakura: el estanque mira hacia afuera del archipielago para que la cascada
    // caiga por fuera; la cabana queda del otro lado del arbol
    let outward = (cherry.1 as f32).atan2(cherry.0 as f32);
    let pond_angle = outward + (rng.next_f32() - 0.5) * 0.8;
    water.extend(structures::carve_pond(&mut map, &blocks, cherry, pond_angle));
    lights.extend(structures::build_cabin(&mut map, &mut world, &blocks, cherry, pond_angle + 0.6 * TAU));
    structures::grow_cherry_tree(&mut map, &mut world, &blocks, cherry, &mut rng);

    // Desierto: piramide escalonada con glowstone en la punta y en la entrada, y cactus
    lights.extend(structures::build_pyramid(&mut map, &mut world, &blocks, desert));
    structures::scatter_cacti(&map, &mut world, &blocks, desert, &mut rng);

    // Hongos gigantes (dos rojos y uno cafe) con shroomlight colgando bajo el sombrero
    lights.extend(structures::grow_mushrooms(&mut map, &mut world, &blocks, mushroom, &mut rng));

    // Nieve: lago congelado, iglu y pinos
    let lake_angle = rng.next_f32() * TAU;
    water.extend(structures::freeze_lake(&mut map, &blocks, snow, lake_angle));
    lights.extend(structures::build_igloo(&mut map, &mut world, &blocks, snow, lake_angle + TAU / 2.0));
    structures::grow_spruces(&map, &mut world, &blocks, snow, lake_angle, &mut rng);

    // Centro: el portal en ruinas
    lights.extend(structures::build_portal_ruins(&mut map, &mut world, &blocks, middle, &mut rng));

    // Capas de cada columna segun su bioma: arriba, relleno y piedra hasta el fondo
    for z in -HALF_SIZE..HALF_SIZE {
        for x in -HALF_SIZE..HALF_SIZE {
            let Some(column) = map.get(x, z) else {
                continue;
            };
            for y in column.bottom..=column.surface {
                // Lo que ya puso una estructura (troncos, paredes) no se tapa
                if world.get(x, y, z) != crate::world::AIR {
                    continue;
                }
                let layers = column.layers;
                let block = if y == column.surface {
                    column.top.unwrap_or(layers.top)
                } else if y >= column.surface - layers.filler_depth {
                    layers.filler
                } else {
                    layers.deep
                };
                world.set(x, y, z, block);
            }
        }
    }

    for (x, y, z, block) in water {
        world.set(x, y, z, block);
    }

    // Puentes de madera de la isla del portal a cada isla de bioma
    for island in [cherry, desert, mushroom, snow] {
        structures::build_bridge(&map, &mut world, &blocks, middle, island);
    }

    Island { world, lights }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::AIR;

    fn column_has_blocks(world: &World, x: i32, z: i32) -> bool {
        (WORLD_BOTTOM..WORLD_BOTTOM + WORLD_HEIGHT as i32).any(|y| world.get(x, y, z) != AIR)
    }

    #[test]
    fn hay_cinco_islas_separadas_por_aire() {
        let world = generate(5).world;
        for focus in &FOCUS_POINTS {
            let (x, z) = (focus.center.x as i32, focus.center.z as i32);
            assert!(column_has_blocks(&world, x, z), "falta {}", focus.name);
        }
        // Fuera de los puentes, entre una isla y otra solo hay aire
        assert!(!column_has_blocks(&world, 14, 0));
        assert!(!column_has_blocks(&world, 0, -14));
    }

    #[test]
    fn un_puente_une_el_centro_con_cada_isla() {
        let world = generate(5).world;
        // A la mitad del camino entre el centro y cada isla hay puente
        let half = ISLAND_OFFSET / 2;
        for (x, z) in [(half, half), (-half, half), (-half, -half), (half, -half)] {
            let near_bridge = (-1..=1).any(|dx| column_has_blocks(&world, x + dx, z));
            assert!(near_bridge, "no hay puente cerca de ({x}, {z})");
        }
    }

    #[test]
    fn cada_isla_cubre_al_menos_16x16_columnas() {
        let mut map = HeightMap::new();
        let shape = IslandShape {
            center: (0, 0),
            radius: 11.0,
            hill: 5.0,
            underside: 10.0,
            base: 0,
            layers: Layers {
                top: 1,
                filler: 2,
                filler_depth: 3,
                deep: 3,
            },
        };
        shape_island(&mut map, &shape, 1);
        let columns = map.columns.iter().filter(|column| column.is_some()).count();
        assert!(columns >= 16 * 16, "solo hay {columns} columnas");
    }

    #[test]
    fn las_luces_estan_junto_a_sus_bloques() {
        let island = generate(11);
        // Portal (2 caras), 4 magmas, cabana (lampara y 2 faroles), iglu,
        // piramide (punta y 2 pilares) y 6 shroomlights en los hongos
        assert_eq!(island.lights.len(), 19);
        for light in &island.lights {
            let p = light.position;
            let (x, y, z) = (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
            let near_block = (-1..=1).any(|dx| {
                (-1..=1).any(|dy| (-1..=1).any(|dz| island.world.get(x + dx, y + dy, z + dz) != AIR))
            });
            assert!(near_block);
        }
    }

    #[test]
    fn misma_semilla_mismo_mundo_y_otra_semilla_otro() {
        let line = |world: &World| -> Vec<BlockId> {
            (-HALF_SIZE..HALF_SIZE).map(|x| world.get(x, 3, ISLAND_OFFSET)).collect()
        };
        let world = |seed| generate(seed).world;
        assert!(line(&world(7)) == line(&world(7)));
        let differs = (1..5).any(|seed| line(&world(seed)) != line(&world(seed + 100)));
        assert!(differs);
    }
}
