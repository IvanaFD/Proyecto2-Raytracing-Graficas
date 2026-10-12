use std::sync::OnceLock;

use crate::color::Color;
use crate::vec3::Vec3;

// Las texturas de Minecraft son de 16x16 pixeles
pub const TEXTURE_SIZE: usize = 16;

// Fuerza del relieve de los mapas normales (mas alto = pendientes mas marcadas)
const STONE_RELIEF: f32 = 6.0;
const BRICK_RELIEF: f32 = 5.0;
// Relieve suave para tierra, madera, hojas, nieve y hongos
const SOFT_RELIEF: f32 = 3.0;

// Oscurece apenas el anillo de pixeles del borde: casi no se nota, pero ayuda a
// distinguir donde termina un bloque y empieza el siguiente
const BORDER_SHADE: f32 = 0.92;

// Numero pseudoaleatorio en [0, 1) que siempre es el mismo para (x, y, seed).
// Asi las texturas se ven "al azar" pero son iguales en cada ejecucion
fn hash(x: usize, y: usize, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374_761_393)
        ^ (y as u32).wrapping_mul(668_265_263)
        ^ seed.wrapping_mul(2_246_822_519);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    (h & 0x00FF_FFFF) as f32 / 16_777_216.0
}

#[derive(Debug, PartialEq)]
pub struct Texture {
    // En el heap: hay decenas de texturas y juntas no caben en la pila
    pixels: Vec<Color>,
}

impl Texture {
    // Construye la textura pixel por pixel y le oscurece el borde
    fn from_fn(pixel: impl Fn(usize, usize) -> Color) -> Self {
        let mut pixels = vec![Color::default(); TEXTURE_SIZE * TEXTURE_SIZE];
        for y in 0..TEXTURE_SIZE {
            for x in 0..TEXTURE_SIZE {
                let on_border = x == 0 || y == 0 || x == TEXTURE_SIZE - 1 || y == TEXTURE_SIZE - 1;
                let shade = if on_border { BORDER_SHADE } else { 1.0 };
                pixels[y * TEXTURE_SIZE + x] = pixel(x, y) * shade;
            }
        }
        Texture { pixels }
    }

    // Nearest neighbor: toma el pixel mas cercano sin mezclar, para que se
    // conserve el estilo pixelado. u y v van de 0 a 1, con v = 0 arriba
    pub fn sample(&self, u: f32, v: f32) -> Color {
        let x = ((u * TEXTURE_SIZE as f32) as usize).min(TEXTURE_SIZE - 1);
        let y = ((v * TEXTURE_SIZE as f32) as usize).min(TEXTURE_SIZE - 1);
        self.pixels[y * TEXTURE_SIZE + x]
    }
}

// Mapa normal de 16x16: en cada pixel, hacia donde "mira" la superficie en espacio
// tangente. x va hacia donde crece u, y hacia donde crece v, y z sale de la cara.
// Un pixel plano es (0, 0, 1)
#[derive(Debug, PartialEq)]
pub struct NormalMap {
    normals: Vec<Vec3>,
}

impl NormalMap {
    // Se toma el brillo de la textura como altura: lo oscuro (grietas, juntas,
    // el borde del bloque) queda hundido. La normal sale de la pendiente de esa
    // altura, medida con diferencias finitas entre los pixeles vecinos
    fn from_texture(texture: &Texture, strength: f32) -> Self {
        let height = |x: usize, y: usize| {
            let color = texture.pixels[y * TEXTURE_SIZE + x];
            0.3 * color.r + 0.59 * color.g + 0.11 * color.b
        };
        let last = TEXTURE_SIZE - 1;

        let mut normals = vec![Vec3::new(0.0, 0.0, 1.0); TEXTURE_SIZE * TEXTURE_SIZE];
        for y in 0..TEXTURE_SIZE {
            for x in 0..TEXTURE_SIZE {
                // En la orilla de la textura se usa el mismo pixel (no se da la vuelta)
                let slope_u = height((x + 1).min(last), y) - height(x.saturating_sub(1), y);
                let slope_v = height(x, (y + 1).min(last)) - height(x, y.saturating_sub(1));
                normals[y * TEXTURE_SIZE + x] =
                    Vec3::new(-slope_u * strength, -slope_v * strength, 1.0).normalize();
            }
        }
        NormalMap { normals }
    }

    pub fn sample(&self, u: f32, v: f32) -> Vec3 {
        let x = ((u * TEXTURE_SIZE as f32) as usize).min(TEXTURE_SIZE - 1);
        let y = ((v * TEXTURE_SIZE as f32) as usize).min(TEXTURE_SIZE - 1);
        self.normals[y * TEXTURE_SIZE + x]
    }
}

// Un bloque puede tener textura distinta arriba, a los lados y abajo (como el pasto)
#[derive(Debug, PartialEq)]
pub struct BlockTextures {
    pub top: Texture,
    pub side: Texture,
    pub bottom: Texture,
    // Relieve de cada cara (arriba, lados, abajo), calculado de su propia textura
    pub normal_maps: Option<[NormalMap; 3]>,
}

// Que cara del bloque es segun su normal: 0 arriba, 1 lados, 2 abajo
fn face_of(normal: &Vec3) -> usize {
    if normal.y > 0.5 {
        0
    } else if normal.y < -0.5 {
        2
    } else {
        1
    }
}

impl BlockTextures {
    pub fn sample(&self, normal: &Vec3, uv: (f32, f32)) -> Color {
        let texture = [&self.top, &self.side, &self.bottom][face_of(normal)];
        texture.sample(uv.0, uv.1)
    }

    // Normal del mapa normal (en espacio tangente) de esa cara, si el bloque tiene relieve
    pub fn normal_at(&self, normal: &Vec3, uv: (f32, f32)) -> Option<Vec3> {
        self.normal_maps
            .as_ref()
            .map(|maps| maps[face_of(normal)].sample(uv.0, uv.1))
    }

    // Le da relieve a cada cara a partir de su propia textura
    fn with_normal_map(self, strength: f32) -> Self {
        let normal_maps = Some([
            NormalMap::from_texture(&self.top, strength),
            NormalMap::from_texture(&self.side, strength),
            NormalMap::from_texture(&self.bottom, strength),
        ]);
        BlockTextures { normal_maps, ..self }
    }
}

fn same_on_all_faces(pixel: impl Fn(usize, usize) -> Color) -> BlockTextures {
    BlockTextures {
        top: Texture::from_fn(&pixel),
        side: Texture::from_fn(&pixel),
        bottom: Texture::from_fn(&pixel),
        normal_maps: None,
    }
}

// ---------- Pixel art de cada bloque ----------
//
// Estilo Minecraft: cada bloque usa una paleta fija de pocos tonos (4 o 5), y el
// patron (manchas, vetas, tablas, ladrillos) decide que tono va en cada pixel.
// Son dibujos propios en ese estilo, generados por codigo; no copias de las texturas

// Un tono de la paleta empieza a usarse desde cierto valor de 0 a 1
type Palette = [(f32, Color); 4];

// El tono que le toca a un valor de 0 a 1
fn pick(value: f32, palette: &Palette) -> Color {
    let mut color = palette[0].1;
    for &(from, shade) in palette {
        if value >= from {
            color = shade;
        }
    }
    color
}

// Ruido agrupado: mezcla el valor de cada pixel con el de su bloque de 2x2, asi los
// tonos forman manchitas como en Minecraft y no puntos sueltos
fn cluster(x: usize, y: usize, seed: u32) -> f32 {
    0.55 * hash(x, y, seed) + 0.45 * hash(x / 2, y / 2, seed + 97)
}

const GRASS: Palette = [
    (0.0, Color::from_rgb(83, 130, 46)),
    (0.25, Color::from_rgb(98, 150, 54)),
    (0.55, Color::from_rgb(112, 166, 62)),
    (0.82, Color::from_rgb(128, 182, 74)),
];
const DIRT: Palette = [
    (0.0, Color::from_rgb(93, 63, 40)),
    (0.22, Color::from_rgb(111, 76, 48)),
    (0.5, Color::from_rgb(124, 86, 55)),
    (0.8, Color::from_rgb(140, 100, 66)),
];
const STONE: Palette = [
    (0.0, Color::from_rgb(98, 98, 98)),
    (0.22, Color::from_rgb(112, 112, 112)),
    (0.45, Color::from_rgb(125, 125, 125)),
    (0.82, Color::from_rgb(140, 140, 140)),
];
const SAND: Palette = [
    (0.0, Color::from_rgb(200, 186, 138)),
    (0.15, Color::from_rgb(214, 200, 151)),
    (0.5, Color::from_rgb(221, 208, 161)),
    (0.85, Color::from_rgb(230, 219, 176)),
];
const OAK_BARK: Palette = [
    (0.0, Color::from_rgb(78, 59, 35)),
    (0.3, Color::from_rgb(95, 74, 44)),
    (0.6, Color::from_rgb(107, 84, 51)),
    (0.88, Color::from_rgb(122, 97, 64)),
];
const OAK_PLANKS: Palette = [
    (0.0, Color::from_rgb(140, 110, 66)),
    (0.3, Color::from_rgb(158, 126, 77)),
    (0.6, Color::from_rgb(168, 136, 84)),
    (0.88, Color::from_rgb(180, 146, 92)),
];
const CHERRY: Palette = [
    (0.0, Color::from_rgb(196, 117, 153)),
    (0.2, Color::from_rgb(224, 154, 184)),
    (0.55, Color::from_rgb(240, 179, 204)),
    (0.85, Color::from_rgb(249, 205, 224)),
];
const WATER: Palette = [
    (0.0, Color::from_rgb(44, 76, 178)),
    (0.3, Color::from_rgb(54, 90, 196)),
    (0.65, Color::from_rgb(64, 104, 210)),
    (0.9, Color::from_rgb(82, 124, 222)),
];
const OBSIDIAN: Palette = [
    (0.0, Color::from_rgb(15, 10, 24)),
    (0.3, Color::from_rgb(22, 16, 31)),
    (0.7, Color::from_rgb(32, 23, 46)),
    (0.93, Color::from_rgb(59, 42, 85)),
];
const PORTAL: Palette = [
    (0.0, Color::from_rgb(90, 26, 160)),
    (0.35, Color::from_rgb(122, 42, 208)),
    (0.7, Color::from_rgb(154, 69, 232)),
    (0.9, Color::from_rgb(192, 112, 255)),
];
const GLOWSTONE: Palette = [
    (0.0, Color::from_rgb(138, 100, 48)),
    (0.22, Color::from_rgb(196, 140, 66)),
    (0.5, Color::from_rgb(230, 182, 84)),
    (0.8, Color::from_rgb(252, 222, 140)),
];
const STONE_BRICK: Palette = [
    (0.0, Color::from_rgb(106, 106, 106)),
    (0.3, Color::from_rgb(116, 116, 116)),
    (0.65, Color::from_rgb(124, 124, 124)),
    (0.9, Color::from_rgb(134, 134, 134)),
];
const NETHERRACK: Palette = [
    (0.0, Color::from_rgb(84, 28, 28)),
    (0.25, Color::from_rgb(104, 38, 38)),
    (0.55, Color::from_rgb(118, 46, 46)),
    (0.85, Color::from_rgb(138, 58, 56)),
];
const NETHER_BRICK: Palette = [
    (0.0, Color::from_rgb(44, 20, 22)),
    (0.35, Color::from_rgb(54, 25, 28)),
    (0.7, Color::from_rgb(64, 31, 35)),
    (0.92, Color::from_rgb(76, 38, 42)),
];
const BLACKSTONE: Palette = [
    (0.0, Color::from_rgb(30, 28, 34)),
    (0.3, Color::from_rgb(38, 35, 43)),
    (0.65, Color::from_rgb(47, 43, 53)),
    (0.9, Color::from_rgb(58, 53, 65)),
];
const SANDSTONE_TONES: Palette = [
    (0.0, Color::from_rgb(201, 186, 134)),
    (0.3, Color::from_rgb(214, 200, 150)),
    (0.65, Color::from_rgb(221, 209, 161)),
    (0.9, Color::from_rgb(229, 218, 174)),
];
const CACTUS: Palette = [
    (0.0, Color::from_rgb(46, 84, 28)),
    (0.3, Color::from_rgb(58, 104, 34)),
    (0.65, Color::from_rgb(70, 122, 40)),
    (0.9, Color::from_rgb(88, 146, 52)),
];
const MYCELIUM: Palette = [
    (0.0, Color::from_rgb(95, 84, 100)),
    (0.3, Color::from_rgb(111, 98, 114)),
    (0.6, Color::from_rgb(125, 111, 128)),
    (0.88, Color::from_rgb(154, 138, 156)),
];
const STEM: Palette = [
    (0.0, Color::from_rgb(196, 188, 166)),
    (0.35, Color::from_rgb(212, 204, 184)),
    (0.7, Color::from_rgb(224, 216, 198)),
    (0.92, Color::from_rgb(234, 228, 212)),
];
const RED_CAP: Palette = [
    (0.0, Color::from_rgb(150, 26, 22)),
    (0.3, Color::from_rgb(178, 34, 30)),
    (0.65, Color::from_rgb(196, 42, 36)),
    (0.92, Color::from_rgb(212, 56, 46)),
];
const BROWN_CAP: Palette = [
    (0.0, Color::from_rgb(112, 82, 58)),
    (0.3, Color::from_rgb(130, 96, 68)),
    (0.65, Color::from_rgb(144, 108, 78)),
    (0.9, Color::from_rgb(158, 120, 88)),
];
const SNOW: Palette = [
    (0.0, Color::from_rgb(222, 230, 238)),
    (0.3, Color::from_rgb(234, 240, 246)),
    (0.65, Color::from_rgb(243, 247, 251)),
    (0.9, Color::from_rgb(250, 252, 254)),
];
const ICE: Palette = [
    (0.0, Color::from_rgb(130, 168, 230)),
    (0.35, Color::from_rgb(146, 182, 238)),
    (0.7, Color::from_rgb(160, 194, 244)),
    (0.92, Color::from_rgb(182, 210, 248)),
];
const SPRUCE_BARK: Palette = [
    (0.0, Color::from_rgb(52, 36, 20)),
    (0.3, Color::from_rgb(66, 46, 27)),
    (0.65, Color::from_rgb(78, 55, 33)),
    (0.9, Color::from_rgb(90, 65, 40)),
];
const SPRUCE_NEEDLES: Palette = [
    (0.0, Color::from_rgb(34, 60, 38)),
    (0.3, Color::from_rgb(44, 74, 47)),
    (0.65, Color::from_rgb(54, 88, 56)),
    (0.9, Color::from_rgb(66, 102, 66)),
];

const SHROOMLIGHT: Palette = [
    (0.0, Color::from_rgb(196, 92, 30)),
    (0.3, Color::from_rgb(232, 128, 44)),
    (0.65, Color::from_rgb(250, 168, 66)),
    (0.88, Color::from_rgb(255, 214, 120)),
];

const WOOL_BLUE: Color = Color::from_rgb(52, 84, 190);
const WOOL_WHITE: Color = Color::from_rgb(234, 236, 236);
const GLASS_TINT: Color = Color::from_rgb(200, 225, 235);

// Pasto: verde parejo con manchitas mas claras y mas oscuras
fn grass_top(x: usize, y: usize) -> Color {
    pick(cluster(x, y, 1), &GRASS)
}

// Tierra: cafe con piedritas claras y huecos oscuros sueltos
fn dirt(x: usize, y: usize) -> Color {
    let speck = hash(x, y, 3);
    if speck > 0.95 {
        Color::from_rgb(160, 122, 86)
    } else if speck < 0.05 {
        Color::from_rgb(74, 50, 32)
    } else {
        pick(cluster(x, y, 2), &DIRT)
    }
}

// Lado del pasto: tierra con 3 filas de pasto arriba y gotas que bajan 1 o 2 mas
fn grass_side(x: usize, y: usize) -> Color {
    let drip = hash(x, 0, 4);
    let depth = 3 + usize::from(drip > 0.45) + usize::from(drip > 0.8);
    if y < depth {
        // Un poco mas oscuro que arriba, como la orilla del pasto en Minecraft
        pick(cluster(x, y, 1), &GRASS) * 0.92
    } else {
        dirt(x, y)
    }
}

// Piedra: gris con vetas oscuras alargadas en horizontal
fn stone(x: usize, y: usize) -> Color {
    let streak = hash(x / 4 + y * 3, y / 2, 5);
    pick(0.5 * hash(x, y, 6) + 0.5 * streak, &STONE)
}

fn sand(x: usize, y: usize) -> Color {
    pick(0.75 * hash(x, y, 8) + 0.25 * hash(x / 2, y / 2, 9), &SAND)
}

// Corteza: vetas verticales, cada columna con su tono y cortes a lo largo
fn log_side(x: usize, y: usize) -> Color {
    let column = hash(x, 0, 9);
    let cut = hash(x, y / 3, 10);
    pick(0.65 * column + 0.35 * cut, &OAK_BARK)
}

// Corte del tronco: anillos claros y oscuros con la corteza alrededor
fn log_top(x: usize, y: usize) -> Color {
    let (dx, dy) = (x as f32 - 7.5, y as f32 - 7.5);
    let ring = dx.abs().max(dy.abs());
    if ring > 6.5 {
        log_side(x, y)
    } else if (ring as usize) % 2 == 0 {
        Color::from_rgb(184, 148, 95)
    } else {
        Color::from_rgb(160, 126, 78)
    }
}

// Tablas: 4 tablas de 4 pixeles, cada una con una linea oscura abajo, un borde claro
// arriba, vetas cortas y la union vertical en distinto lugar
fn planks(x: usize, y: usize) -> Color {
    let board = y / 4;
    let row = y % 4;
    let seam = if board % 2 == 0 { 4 } else { 12 };
    if row == 3 || x == seam {
        Color::from_rgb(104, 80, 46)
    } else if row == 0 {
        Color::from_rgb(184, 150, 96)
    } else {
        pick(0.6 * hash(x / 3, y, 13 + board as u32) + 0.4 * hash(x, y, 14), &OAK_PLANKS)
    }
}

// Hojas de cerezo: rosado con huecos oscuros entre las hojas
fn cherry_leaves(x: usize, y: usize) -> Color {
    if hash(x, y, 15) < 0.12 {
        Color::from_rgb(138, 79, 106)
    } else {
        pick(cluster(x, y, 16), &CHERRY)
    }
}

// Agua: azul con franjas claras inclinadas, como olas
fn water(x: usize, y: usize) -> Color {
    let wave = (x + 2 * y) % 8 == 0 && hash(x / 3, y, 18) > 0.3;
    if wave {
        WATER[3].1
    } else {
        pick(0.7 * hash(x / 2, y, 19) + 0.3 * hash(x, y, 20), &WATER)
    }
}

// Obsidiana: casi negra con manchas moradas
fn obsidian(x: usize, y: usize) -> Color {
    pick(cluster(x, y, 21), &OBSIDIAN)
}

// Portal: remolino morado de bandas diagonales que se tuercen
fn portal(x: usize, y: usize) -> Color {
    let swirl = ((x as f32 * 0.9 + y as f32 * 0.6 + 3.0 * hash(x / 3, y / 3, 23)).sin() + 1.0) / 2.0;
    pick(swirl, &PORTAL)
}

// Lampara (glowstone): grumos amarillos brillantes con huecos cafes
fn lamp(x: usize, y: usize) -> Color {
    pick(0.4 * hash(x, y, 24) + 0.6 * hash(x / 3, y / 3, 25), &GLOWSTONE)
}

// Ladrillos de piedra: 2 filas de ladrillos de 8x4 alternados (en Minecraft son
// 4 filas), con junta oscura y cada ladrillo con borde claro arriba y oscuro abajo
fn stone_bricks(x: usize, y: usize) -> Color {
    let row = y / 4;
    let shifted = (x + if row % 2 == 0 { 0 } else { 4 }) % 8;
    if y % 4 == 3 || shifted == 7 {
        Color::from_rgb(78, 78, 78)
    } else if y % 4 == 0 || shifted == 0 {
        Color::from_rgb(146, 146, 146)
    } else if y % 4 == 2 && shifted == 6 {
        Color::from_rgb(98, 98, 98)
    } else {
        pick(cluster(x, y, 26), &STONE_BRICK)
    }
}

// Agua cayendo: azul con vetas verticales de espuma
fn waterfall(x: usize, y: usize) -> Color {
    let streak = hash(x, 0, 27) > 0.55 && hash(x, y / 3, 28) > 0.35;
    if streak {
        Color::from_rgb(196, 214, 246)
    } else {
        pick(0.6 * hash(x, y / 4, 29) + 0.4 * hash(x, y, 30), &WATER)
    }
}

// Netherrack: rojo oscuro grumoso con hoyitos
fn netherrack(x: usize, y: usize) -> Color {
    if hash(x, y, 33) < 0.07 {
        Color::from_rgb(66, 20, 20)
    } else {
        pick(cluster(x, y, 31), &NETHERRACK)
    }
}

// Ladrillos del Nether: ladrillos pequenos rojo oscuro con junta casi negra
fn nether_bricks(x: usize, y: usize) -> Color {
    let row = y / 4;
    let shifted = (x + if row % 2 == 0 { 0 } else { 3 }) % 6;
    if y % 4 == 3 || shifted == 5 {
        Color::from_rgb(26, 11, 13)
    } else if y % 4 == 0 {
        Color::from_rgb(84, 42, 46)
    } else {
        pick(cluster(x, y, 67), &NETHER_BRICK)
    }
}

// Piedra negra del Nether: gris muy oscuro con un tono azulado
fn blackstone(x: usize, y: usize) -> Color {
    pick(0.5 * hash(x, y, 68) + 0.5 * hash(x / 3, y / 2, 69), &BLACKSTONE)
}

// Magma: roca oscura con grietas de lava naranja que brillan
fn magma(x: usize, y: usize) -> Color {
    let crack = ((x as f32 * 0.8 + 3.0 * hash(x / 4, y / 4, 34)).sin()
        * (y as f32 * 0.7 + 3.0 * hash(y / 4, x / 4, 35)).cos())
    .abs();
    if crack < 0.12 {
        Color::from_rgb(255, 196, 80)
    } else if crack < 0.25 {
        Color::from_rgb(240, 120, 32)
    } else if hash(x, y, 37) > 0.5 {
        Color::from_rgb(74, 30, 20)
    } else {
        Color::from_rgb(56, 22, 16)
    }
}

// Lana: tejido de 3 tonos en diagonal
fn wool(color: Color) -> impl Fn(usize, usize) -> Color {
    move |x, y| {
        let weave = match (x + y) % 3 {
            0 => 1.0,
            1 => 0.93,
            _ => 0.88,
        };
        color * (weave * (0.96 + 0.06 * hash(x, y, 30)))
    }
}

// ---------- Desierto ----------

// Arenisca: franja clara arriba, oscura abajo y lineas finas entre capas
fn sandstone_side(x: usize, y: usize) -> Color {
    match y {
        0..=2 => SANDSTONE_TONES[3].1,
        3 | 12 => SANDSTONE_TONES[0].1,
        13..=15 => SANDSTONE_TONES[1].1,
        _ => pick(0.7 * hash(x, y, 47) + 0.3 * hash(x / 2, y, 48), &SANDSTONE_TONES),
    }
}

fn sandstone_top(x: usize, y: usize) -> Color {
    pick(0.4 + 0.6 * hash(x, y, 48), &SANDSTONE_TONES)
}

// Cactus: franjas verticales claras, orillas oscuras y espinas
fn cactus_side(x: usize, y: usize) -> Color {
    if x == 0 || x == 15 {
        CACTUS[0].1
    } else if x % 4 == 2 && y % 4 == 1 {
        Color::from_rgb(28, 46, 18)
    } else if x % 4 == 2 {
        CACTUS[3].1
    } else {
        pick(0.3 + 0.5 * hash(x, y / 2, 49), &CACTUS)
    }
}

fn cactus_top(x: usize, y: usize) -> Color {
    let (dx, dy) = (x as f32 - 7.5, y as f32 - 7.5);
    let ring = dx.abs().max(dy.abs());
    if ring > 6.5 {
        CACTUS[0].1
    } else if (ring as usize) % 3 == 0 {
        CACTUS[3].1
    } else {
        pick(0.3 + 0.4 * hash(x, y, 50), &CACTUS)
    }
}

// ---------- Hongos ----------

// Micelio: gris morado con puntitos claros
fn mycelium_top(x: usize, y: usize) -> Color {
    pick(cluster(x, y, 51), &MYCELIUM)
}

// Tierra con una franja de micelio arriba, como el pasto
fn mycelium_side(x: usize, y: usize) -> Color {
    let drip = hash(x, 0, 53);
    let depth = 2 + usize::from(drip > 0.4) + usize::from(drip > 0.8);
    if y < depth { mycelium_top(x, y) } else { dirt(x, y) }
}

fn mushroom_stem(x: usize, y: usize) -> Color {
    pick(0.6 * hash(x, 0, 54) + 0.4 * hash(x, y / 2, 55), &STEM)
}

// Sombrero rojo con manchas blancas de 2x2 a 3x3
fn red_cap(x: usize, y: usize) -> Color {
    let spots = [(2, 2), (9, 1), (5, 7), (12, 8), (1, 12), (9, 13)];
    let on_spot = spots.iter().any(|&(sx, sy)| {
        let (dx, dy) = (x as i32 - sx, y as i32 - sy);
        (0..=2).contains(&dx) && (0..=2).contains(&dy) && !(dx == 2 && dy == 2)
    });
    if on_spot {
        Color::from_rgb(232, 228, 220)
    } else {
        pick(cluster(x, y, 57), &RED_CAP)
    }
}

// Shroomlight: naranja brillante en grumos, el bloque de luz que crece en los hongos
fn shroomlight(x: usize, y: usize) -> Color {
    pick(0.45 * hash(x, y, 70) + 0.55 * hash(x / 3, y / 3, 71), &SHROOMLIGHT)
}

fn brown_cap(x: usize, y: usize) -> Color {
    pick(cluster(x, y, 58), &BROWN_CAP)
}

// ---------- Nieve ----------

fn snow(x: usize, y: usize) -> Color {
    pick(cluster(x, y, 60), &SNOW)
}

// Tierra con una capa de nieve arriba que baja un poco en algunas columnas
fn snowy_grass_side(x: usize, y: usize) -> Color {
    let drip = hash(x, 0, 61);
    let depth = 3 + usize::from(drip > 0.5);
    if y < depth { snow(x, y) } else { dirt(x, y) }
}

// Hielo: azul claro con rayas blancas en diagonal
fn ice(x: usize, y: usize) -> Color {
    let streak = (x + y) % 7 == 0 || (x + y) % 11 == 3;
    if streak && hash(x, y, 62) > 0.3 {
        Color::from_rgb(214, 232, 252)
    } else {
        pick(0.6 * hash(x / 2, y / 2, 63) + 0.4 * hash(x, y, 64), &ICE)
    }
}

fn spruce_side(x: usize, y: usize) -> Color {
    pick(0.65 * hash(x, 0, 63) + 0.35 * hash(x, y / 3, 64), &SPRUCE_BARK)
}

fn spruce_top(x: usize, y: usize) -> Color {
    let (dx, dy) = (x as f32 - 7.5, y as f32 - 7.5);
    let ring = dx.abs().max(dy.abs());
    if ring > 6.5 {
        spruce_side(x, y)
    } else if (ring as usize) % 2 == 0 {
        Color::from_rgb(128, 96, 60)
    } else {
        Color::from_rgb(108, 80, 48)
    }
}

// Hojas de pino: verde oscuro con huecos y un poco de nieve
fn spruce_leaves(x: usize, y: usize) -> Color {
    let roll = hash(x, y, 66);
    if roll < 0.12 {
        Color::from_rgb(20, 36, 24)
    } else if roll > 0.97 {
        SNOW[2].1
    } else {
        pick(cluster(x, y, 65), &SPRUCE_NEEDLES)
    }
}

// ---------- Cabana ----------

// Vidrio: marco claro y un par de reflejos diagonales sobre un tono casi transparente
fn glass(x: usize, y: usize) -> Color {
    let frame = x == 0 || y == 0 || x == TEXTURE_SIZE - 1 || y == TEXTURE_SIZE - 1;
    let shine = (x + y == 9 || x + y == 11) && x > 3 && x < 10;
    if frame {
        Color::from_rgb(235, 240, 245)
    } else if shine {
        Color::new(1.0, 1.0, 1.0)
    } else {
        GLASS_TINT
    }
}

fn three_faces(
    top: impl Fn(usize, usize) -> Color,
    side: impl Fn(usize, usize) -> Color,
    bottom: impl Fn(usize, usize) -> Color,
) -> BlockTextures {
    BlockTextures {
        top: Texture::from_fn(top),
        side: Texture::from_fn(side),
        bottom: Texture::from_fn(bottom),
        normal_maps: None,
    }
}

// Todas las texturas del juego. Se generan una sola vez y quedan vivas todo el
// programa, asi los materiales solo guardan una referencia (&'static)
pub struct TextureSet {
    pub grass: BlockTextures,
    pub dirt: BlockTextures,
    pub stone: BlockTextures,
    pub sand: BlockTextures,
    pub log: BlockTextures,
    pub planks: BlockTextures,
    pub cherry_leaves: BlockTextures,
    pub water: BlockTextures,
    pub obsidian: BlockTextures,
    pub portal: BlockTextures,
    pub lamp: BlockTextures,
    pub stone_bricks: BlockTextures,
    pub waterfall: BlockTextures,
    pub blue_wool: BlockTextures,
    pub white_wool: BlockTextures,
    pub netherrack: BlockTextures,
    pub magma: BlockTextures,
    pub sandstone: BlockTextures,
    pub cactus: BlockTextures,
    pub mycelium: BlockTextures,
    pub mushroom_stem: BlockTextures,
    pub red_cap: BlockTextures,
    pub brown_cap: BlockTextures,
    pub snowy_grass: BlockTextures,
    pub snow: BlockTextures,
    pub ice: BlockTextures,
    pub spruce_log: BlockTextures,
    pub spruce_leaves: BlockTextures,
    pub glass: BlockTextures,
    pub nether_bricks: BlockTextures,
    pub blackstone: BlockTextures,
    pub shroomlight: BlockTextures,
}

impl TextureSet {
    fn generate() -> Self {
        TextureSet {
            grass: BlockTextures {
                top: Texture::from_fn(grass_top),
                side: Texture::from_fn(grass_side),
                bottom: Texture::from_fn(dirt),
                normal_maps: None,
            }
            .with_normal_map(SOFT_RELIEF),
            dirt: same_on_all_faces(dirt).with_normal_map(SOFT_RELIEF),
            // La piedra y los ladrillos tienen relieve: grietas y juntas hundidas
            stone: same_on_all_faces(stone).with_normal_map(STONE_RELIEF),
            sand: same_on_all_faces(sand).with_normal_map(SOFT_RELIEF),
            log: BlockTextures {
                top: Texture::from_fn(log_top),
                side: Texture::from_fn(log_side),
                bottom: Texture::from_fn(log_top),
                normal_maps: None,
            }
            .with_normal_map(SOFT_RELIEF),
            planks: same_on_all_faces(planks).with_normal_map(SOFT_RELIEF),
            cherry_leaves: same_on_all_faces(cherry_leaves).with_normal_map(SOFT_RELIEF),
            water: same_on_all_faces(water),
            obsidian: same_on_all_faces(obsidian),
            portal: same_on_all_faces(portal),
            lamp: same_on_all_faces(lamp),
            stone_bricks: same_on_all_faces(stone_bricks).with_normal_map(BRICK_RELIEF),
            waterfall: same_on_all_faces(waterfall),
            blue_wool: same_on_all_faces(wool(WOOL_BLUE)),
            white_wool: same_on_all_faces(wool(WOOL_WHITE)),
            netherrack: same_on_all_faces(netherrack).with_normal_map(STONE_RELIEF),
            magma: same_on_all_faces(magma),
            sandstone: three_faces(sandstone_top, sandstone_side, sandstone_top)
                .with_normal_map(STONE_RELIEF),
            cactus: three_faces(cactus_top, cactus_side, cactus_top).with_normal_map(SOFT_RELIEF),
            mycelium: three_faces(mycelium_top, mycelium_side, dirt).with_normal_map(SOFT_RELIEF),
            mushroom_stem: same_on_all_faces(mushroom_stem),
            red_cap: same_on_all_faces(red_cap).with_normal_map(SOFT_RELIEF),
            brown_cap: same_on_all_faces(brown_cap).with_normal_map(SOFT_RELIEF),
            snowy_grass: three_faces(snow, snowy_grass_side, dirt).with_normal_map(SOFT_RELIEF),
            snow: same_on_all_faces(snow).with_normal_map(SOFT_RELIEF),
            ice: same_on_all_faces(ice),
            spruce_log: three_faces(spruce_top, spruce_side, spruce_top).with_normal_map(SOFT_RELIEF),
            spruce_leaves: same_on_all_faces(spruce_leaves).with_normal_map(SOFT_RELIEF),
            glass: same_on_all_faces(glass),
            nether_bricks: same_on_all_faces(nether_bricks).with_normal_map(BRICK_RELIEF),
            blackstone: same_on_all_faces(blackstone).with_normal_map(STONE_RELIEF),
            shroomlight: same_on_all_faces(shroomlight),
        }
    }
}

pub fn textures() -> &'static TextureSet {
    static TEXTURES: OnceLock<TextureSet> = OnceLock::new();
    TEXTURES.get_or_init(TextureSet::generate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_es_determinista_y_esta_en_rango() {
        for i in 0..100 {
            let value = hash(i, i * 7, 3);
            assert_eq!(value, hash(i, i * 7, 3));
            assert!((0.0..1.0).contains(&value));
        }
    }

    #[test]
    fn sample_usa_el_pixel_mas_cercano() {
        let texture = Texture::from_fn(|x, y| Color::new(x as f32, y as f32, 0.0));
        // El pixel (5, 9) esta lejos del borde, asi que no se oscurece
        let color = texture.sample(5.5 / 16.0, 9.5 / 16.0);
        assert_eq!(color, Color::new(5.0, 9.0, 0.0));
        // u = v = 1 no se sale del arreglo
        texture.sample(1.0, 1.0);
    }

    #[test]
    fn textura_plana_da_normales_planas() {
        let flat = Texture::from_fn(|_, _| Color::new(0.5, 0.5, 0.5));
        let normal_map = NormalMap::from_texture(&flat, 4.0);
        // Lejos del borde oscurecido todo es plano
        assert_eq!(normal_map.sample(0.5, 0.5), Vec3::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn la_normal_se_inclina_lejos_de_lo_alto() {
        // Mas claro (mas alto) hacia la derecha: la superficie mira hacia la izquierda (-u)
        let ramp = Texture::from_fn(|x, _| Color::new(x as f32 / 16.0, x as f32 / 16.0, x as f32 / 16.0));
        let normal = NormalMap::from_texture(&ramp, 4.0).sample(0.5, 0.5);
        assert!(normal.x < 0.0);
        assert!(normal.y.abs() < 1e-6);
        assert!((normal.length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn el_pasto_es_verde_arriba_y_tierra_abajo() {
        let grass = &textures().grass;
        let top = grass.sample(&Vec3::new(0.0, 1.0, 0.0), (0.5, 0.5));
        let bottom = grass.sample(&Vec3::new(0.0, -1.0, 0.0), (0.5, 0.5));
        assert!(top.g > top.r);
        assert!(bottom.r > bottom.g);
    }
}
