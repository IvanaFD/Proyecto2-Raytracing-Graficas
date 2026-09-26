use std::sync::OnceLock;

use crate::color::Color;
use crate::vec3::Vec3;

// Las texturas de Minecraft son de 16x16 pixeles
pub const TEXTURE_SIZE: usize = 16;

// Oscurece el anillo de pixeles del borde para que se note donde termina
// un bloque y empieza el siguiente
const BORDER_SHADE: f32 = 0.82;

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
    pixels: [Color; TEXTURE_SIZE * TEXTURE_SIZE],
}

impl Texture {
    // Construye la textura pixel por pixel y le oscurece el borde
    fn from_fn(pixel: impl Fn(usize, usize) -> Color) -> Self {
        let mut pixels = [Color::default(); TEXTURE_SIZE * TEXTURE_SIZE];
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

// Un bloque puede tener textura distinta arriba, a los lados y abajo (como el pasto)
#[derive(Debug, PartialEq)]
pub struct BlockTextures {
    pub top: Texture,
    pub side: Texture,
    pub bottom: Texture,
}

impl BlockTextures {
    pub fn sample(&self, normal: &Vec3, uv: (f32, f32)) -> Color {
        let texture = if normal.y > 0.5 {
            &self.top
        } else if normal.y < -0.5 {
            &self.bottom
        } else {
            &self.side
        };
        texture.sample(uv.0, uv.1)
    }
}

fn same_on_all_faces(pixel: impl Fn(usize, usize) -> Color) -> BlockTextures {
    BlockTextures {
        top: Texture::from_fn(&pixel),
        side: Texture::from_fn(&pixel),
        bottom: Texture::from_fn(&pixel),
    }
}

// ---------- Pixel art de cada bloque ----------

const GRASS_GREEN: Color = Color::from_rgb(106, 170, 64);
const DIRT_BROWN: Color = Color::from_rgb(134, 96, 67);
const STONE_GRAY: Color = Color::from_rgb(128, 128, 128);
const SAND_YELLOW: Color = Color::from_rgb(219, 207, 163);
const BARK_BROWN: Color = Color::from_rgb(102, 81, 49);
const WOOD_LIGHT: Color = Color::from_rgb(176, 144, 90);
const WOOD_DARK: Color = Color::from_rgb(146, 115, 68);
const PLANK_BROWN: Color = Color::from_rgb(162, 130, 78);
const CHERRY_PINK: Color = Color::from_rgb(242, 164, 204);
const WATER_BLUE: Color = Color::from_rgb(52, 98, 205);
const OBSIDIAN_BLACK: Color = Color::from_rgb(22, 15, 34);
const OBSIDIAN_PURPLE: Color = Color::from_rgb(66, 40, 100);
const PORTAL_PURPLE: Color = Color::from_rgb(128, 42, 214);
const LAMP_YELLOW: Color = Color::from_rgb(255, 214, 120);
const LAMP_FRAME: Color = Color::from_rgb(92, 58, 30);
const BRICK_GRAY: Color = Color::from_rgb(122, 122, 122);

fn grass_top(x: usize, y: usize) -> Color {
    GRASS_GREEN * (0.8 + 0.3 * hash(x, y, 1))
}

fn dirt(x: usize, y: usize) -> Color {
    let base = DIRT_BROWN * (0.8 + 0.3 * hash(x, y, 2));
    let speck = hash(x, y, 3);
    if speck < 0.12 {
        base * 0.7
    } else if speck > 0.93 {
        base * 1.2
    } else {
        base
    }
}

// Tierra con una franja de pasto arriba que "gotea" distinto en cada columna
fn grass_side(x: usize, y: usize) -> Color {
    let grass_depth = 2 + (hash(x, 0, 4) * 3.0) as usize;
    if y < grass_depth {
        grass_top(x, y)
    } else {
        dirt(x, y)
    }
}

// Manchas de 2x2 pixeles, grano fino y algunas grietas oscuras
fn stone(x: usize, y: usize) -> Color {
    let blotch = 0.8 + 0.25 * hash(x / 2, y / 2, 5);
    let grain = 0.95 + 0.1 * hash(x, y, 6);
    let crack = if hash(x, y, 7) < 0.08 { 0.65 } else { 1.0 };
    STONE_GRAY * (blotch * grain * crack)
}

fn sand(x: usize, y: usize) -> Color {
    SAND_YELLOW * (0.9 + 0.15 * hash(x, y, 8))
}

// Corteza: vetas verticales, cada columna con su propio tono
fn log_side(x: usize, y: usize) -> Color {
    let streak = 0.8 + 0.3 * hash(x, 0, 9);
    let groove = if hash(x, 0, 10) < 0.25 { 0.7 } else { 1.0 };
    let grain = 0.93 + 0.1 * hash(x, y, 11);
    BARK_BROWN * (streak * groove * grain)
}

// Corte del tronco: anillos concentricos y la corteza en la orilla
fn log_top(x: usize, y: usize) -> Color {
    let center = (TEXTURE_SIZE as f32 - 1.0) / 2.0;
    let dx = x as f32 - center;
    let dy = y as f32 - center;
    let radius = (dx * dx + dy * dy).sqrt();

    if radius > 6.5 {
        log_side(x, y)
    } else if (radius as usize) % 2 == 0 {
        WOOD_LIGHT
    } else {
        WOOD_DARK
    }
}

// Tablas de 4 pixeles de alto con las uniones alternadas, como un piso de madera
fn planks(x: usize, y: usize) -> Color {
    let board = y / 4;
    let seam_row = y % 4 == 3;
    let seam_column = if board % 2 == 0 { 7 } else { 15 };
    let board_tone = 0.9 + 0.15 * hash(board, 0, 12);
    let grain = 0.94 + 0.08 * hash(x / 3, y, 13);

    if seam_row || x == seam_column {
        PLANK_BROWN * 0.65
    } else {
        PLANK_BROWN * (board_tone * grain)
    }
}

// Hojas de cerezo (sakura): rosado con petalos claros y huecos mas oscuros
fn cherry_leaves(x: usize, y: usize) -> Color {
    let base = CHERRY_PINK * (0.8 + 0.3 * hash(x, y, 16));
    let petal = hash(x, y, 17);
    if petal < 0.18 {
        base * 0.62
    } else if petal > 0.9 {
        Color::new(1.0, 0.9, 0.95)
    } else {
        base
    }
}

// Agua: azul con lineas horizontales claras, como pequenas olas
fn water(x: usize, y: usize) -> Color {
    let wave = (y + x / 4) % 5 == 0 && hash(x / 2, y, 18) > 0.4;
    let base = WATER_BLUE * (0.9 + 0.15 * hash(x, y, 19));
    if wave { base * 1.35 } else { base }
}

// Obsidiana: casi negra con vetas moradas
fn obsidian(x: usize, y: usize) -> Color {
    let vein = hash(x / 2, y / 2, 20);
    if vein > 0.8 {
        OBSIDIAN_PURPLE * (0.8 + 0.3 * hash(x, y, 21))
    } else {
        OBSIDIAN_BLACK * (0.8 + 0.4 * hash(x, y, 22))
    }
}

// Portal: remolino morado (bandas diagonales que se tuercen con ruido)
fn portal(x: usize, y: usize) -> Color {
    let swirl = ((x as f32 * 0.9 + y as f32 * 0.6 + 3.0 * hash(x / 3, y / 3, 23)).sin() + 1.0) / 2.0;
    PORTAL_PURPLE * (0.6 + 0.6 * swirl)
}

// Lampara: marco de madera y un centro amarillo que brilla
fn lamp(x: usize, y: usize) -> Color {
    let frame = x < 2 || y < 2 || x > TEXTURE_SIZE - 3 || y > TEXTURE_SIZE - 3;
    let cross = x == 7 || x == 8 || y == 7 || y == 8;
    if frame {
        LAMP_FRAME * (0.85 + 0.2 * hash(x, y, 24))
    } else if cross {
        LAMP_FRAME * 1.3
    } else {
        LAMP_YELLOW * (0.85 + 0.2 * hash(x, y, 25))
    }
}

// Ladrillos de piedra: filas de 4 px de alto con las juntas alternadas
fn stone_bricks(x: usize, y: usize) -> Color {
    let row = y / 4;
    let mortar_row = y % 4 == 3;
    let mortar_column = (x + if row % 2 == 0 { 0 } else { 4 }) % 8 == 7;
    if mortar_row || mortar_column {
        BRICK_GRAY * 0.6
    } else {
        BRICK_GRAY * (0.85 + 0.2 * hash(x, y, 26))
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
}

impl TextureSet {
    fn generate() -> Self {
        TextureSet {
            grass: BlockTextures {
                top: Texture::from_fn(grass_top),
                side: Texture::from_fn(grass_side),
                bottom: Texture::from_fn(dirt),
            },
            dirt: same_on_all_faces(dirt),
            stone: same_on_all_faces(stone),
            sand: same_on_all_faces(sand),
            log: BlockTextures {
                top: Texture::from_fn(log_top),
                side: Texture::from_fn(log_side),
                bottom: Texture::from_fn(log_top),
            },
            planks: same_on_all_faces(planks),
            cherry_leaves: same_on_all_faces(cherry_leaves),
            water: same_on_all_faces(water),
            obsidian: same_on_all_faces(obsidian),
            portal: same_on_all_faces(portal),
            lamp: same_on_all_faces(lamp),
            stone_bricks: same_on_all_faces(stone_bricks),
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
    fn el_pasto_es_verde_arriba_y_tierra_abajo() {
        let grass = &textures().grass;
        let top = grass.sample(&Vec3::new(0.0, 1.0, 0.0), (0.5, 0.5));
        let bottom = grass.sample(&Vec3::new(0.0, -1.0, 0.0), (0.5, 0.5));
        assert!(top.g > top.r);
        assert!(bottom.r > bottom.g);
    }
}
