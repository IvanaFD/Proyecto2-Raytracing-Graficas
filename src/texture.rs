use std::sync::OnceLock;

use crate::color::Color;
use crate::vec3::Vec3;

// Las texturas de Minecraft son de 16x16 pixeles
pub const TEXTURE_SIZE: usize = 16;

// Fuerza del relieve de los mapas normales (mas alto = pendientes mas marcadas)
const STONE_RELIEF: f32 = 6.0;
const BRICK_RELIEF: f32 = 5.0;

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

// Mapa normal de 16x16: en cada pixel, hacia donde "mira" la superficie en espacio
// tangente. x va hacia donde crece u, y hacia donde crece v, y z sale de la cara.
// Un pixel plano es (0, 0, 1)
#[derive(Debug, PartialEq)]
pub struct NormalMap {
    normals: [Vec3; TEXTURE_SIZE * TEXTURE_SIZE],
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

        let mut normals = [Vec3::new(0.0, 0.0, 1.0); TEXTURE_SIZE * TEXTURE_SIZE];
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
    // Relieve de la superficie; el mismo en todas las caras
    pub normal_map: Option<NormalMap>,
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

    // Le da relieve al bloque a partir de su textura de los lados
    fn with_normal_map(self, strength: f32) -> Self {
        let normal_map = Some(NormalMap::from_texture(&self.side, strength));
        BlockTextures { normal_map, ..self }
    }
}

fn same_on_all_faces(pixel: impl Fn(usize, usize) -> Color) -> BlockTextures {
    BlockTextures {
        top: Texture::from_fn(&pixel),
        side: Texture::from_fn(&pixel),
        bottom: Texture::from_fn(&pixel),
        normal_map: None,
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
const FOAM_WHITE: Color = Color::from_rgb(214, 232, 255);
const WOOL_BLUE: Color = Color::from_rgb(52, 84, 190);
const WOOL_WHITE: Color = Color::from_rgb(234, 236, 236);
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

// Agua cayendo: vetas verticales de espuma que bajan por la cascada
fn waterfall(x: usize, y: usize) -> Color {
    let streak = hash(x, 0, 27);
    let flow = hash(x, y / 3, 28);
    let base = WATER_BLUE * (0.95 + 0.2 * hash(x, y, 29));
    if streak > 0.55 && flow > 0.35 {
        base.lerp(&FOAM_WHITE, 0.55 + 0.3 * flow)
    } else {
        base
    }
}

// Lana: tejido suave con un poco de grano, para el toldo del puesto
fn wool(color: Color) -> impl Fn(usize, usize) -> Color {
    move |x, y| {
        let weave = if (x + y) % 2 == 0 { 1.0 } else { 0.94 };
        color * (weave * (0.9 + 0.12 * hash(x, y, 30)))
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
    pub waterfall: BlockTextures,
    pub blue_wool: BlockTextures,
    pub white_wool: BlockTextures,
}

impl TextureSet {
    fn generate() -> Self {
        TextureSet {
            grass: BlockTextures {
                top: Texture::from_fn(grass_top),
                side: Texture::from_fn(grass_side),
                bottom: Texture::from_fn(dirt),
                normal_map: None,
            },
            dirt: same_on_all_faces(dirt),
            // La piedra y los ladrillos tienen relieve: grietas y juntas hundidas
            stone: same_on_all_faces(stone).with_normal_map(STONE_RELIEF),
            sand: same_on_all_faces(sand),
            log: BlockTextures {
                top: Texture::from_fn(log_top),
                side: Texture::from_fn(log_side),
                bottom: Texture::from_fn(log_top),
                normal_map: None,
            },
            planks: same_on_all_faces(planks),
            cherry_leaves: same_on_all_faces(cherry_leaves),
            water: same_on_all_faces(water),
            obsidian: same_on_all_faces(obsidian),
            portal: same_on_all_faces(portal),
            lamp: same_on_all_faces(lamp),
            stone_bricks: same_on_all_faces(stone_bricks).with_normal_map(BRICK_RELIEF),
            waterfall: same_on_all_faces(waterfall),
            blue_wool: same_on_all_faces(wool(WOOL_BLUE)),
            white_wool: same_on_all_faces(wool(WOOL_WHITE)),
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
