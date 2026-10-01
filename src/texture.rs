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
    // Solo los sprites (flores, pasto) tienen partes transparentes: false = no hay
    // nada en ese pixel y el rayo pasa de largo
    mask: Option<[bool; TEXTURE_SIZE * TEXTURE_SIZE]>,
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
        Texture { pixels, mask: None }
    }

    // Sprite con transparencia: `pixel` devuelve None donde no hay nada. Sin borde
    // oscurecido, porque no es la cara de un bloque
    fn sprite(pixel: impl Fn(usize, usize) -> Option<Color>) -> Self {
        let mut pixels = [Color::default(); TEXTURE_SIZE * TEXTURE_SIZE];
        let mut mask = [false; TEXTURE_SIZE * TEXTURE_SIZE];
        for y in 0..TEXTURE_SIZE {
            for x in 0..TEXTURE_SIZE {
                if let Some(color) = pixel(x, y) {
                    pixels[y * TEXTURE_SIZE + x] = color;
                    mask[y * TEXTURE_SIZE + x] = true;
                }
            }
        }
        Texture {
            pixels,
            mask: Some(mask),
        }
    }

    fn texel_index(u: f32, v: f32) -> usize {
        let x = ((u * TEXTURE_SIZE as f32) as usize).min(TEXTURE_SIZE - 1);
        let y = ((v * TEXTURE_SIZE as f32) as usize).min(TEXTURE_SIZE - 1);
        y * TEXTURE_SIZE + x
    }

    // Si en (u, v) hay algo; en una textura sin mascara siempre
    pub fn is_opaque(&self, u: f32, v: f32) -> bool {
        self.mask.is_none_or(|mask| mask[Self::texel_index(u, v)])
    }

    // Nearest neighbor: toma el pixel mas cercano sin mezclar, para que se
    // conserve el estilo pixelado. u y v van de 0 a 1, con v = 0 arriba
    pub fn sample(&self, u: f32, v: f32) -> Color {
        self.pixels[Self::texel_index(u, v)]
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

// Una planta usa el mismo sprite en sus dos planos cruzados
fn plant(pixel: impl Fn(usize, usize) -> Option<Color>) -> BlockTextures {
    BlockTextures {
        top: Texture::sprite(&pixel),
        side: Texture::sprite(&pixel),
        bottom: Texture::sprite(&pixel),
        normal_map: None,
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
const NETHERRACK_RED: Color = Color::from_rgb(112, 44, 44);
const MAGMA_DARK: Color = Color::from_rgb(58, 22, 16);
const MAGMA_GLOW: Color = Color::from_rgb(255, 128, 32);
const STEM_GREEN: Color = Color::from_rgb(58, 128, 40);
const TUFT_GREEN: Color = Color::from_rgb(92, 158, 58);
const ALLIUM_PURPLE: Color = Color::from_rgb(178, 110, 230);
const POPPY_RED: Color = Color::from_rgb(220, 40, 36);
const DANDELION_YELLOW: Color = Color::from_rgb(250, 214, 50);
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

// Netherrack: roca roja del Nether, grumosa, con manchas oscuras
fn netherrack(x: usize, y: usize) -> Color {
    let lump = 0.75 + 0.35 * hash(x / 2, y / 2, 31);
    let grain = 0.9 + 0.15 * hash(x, y, 32);
    let pit = if hash(x, y, 33) < 0.1 { 0.6 } else { 1.0 };
    NETHERRACK_RED * (lump * grain * pit)
}

// Magma: roca oscura con grietas de lava naranja que brillan
fn magma(x: usize, y: usize) -> Color {
    // Las grietas siguen lineas que se tuercen con ruido
    let crack = ((x as f32 * 0.8 + 3.0 * hash(x / 4, y / 4, 34)).sin()
        * (y as f32 * 0.7 + 3.0 * hash(y / 4, x / 4, 35)).cos())
    .abs();
    if crack < 0.25 {
        MAGMA_GLOW * (0.85 + 0.3 * hash(x, y, 36))
    } else {
        MAGMA_DARK * (0.8 + 0.4 * hash(x, y, 37))
    }
}

// ---------- Sprites de plantas (None = transparente) ----------

// Tallo de 2 pixeles de ancho que sube desde abajo hasta `top`
fn stem(x: usize, y: usize, top: usize) -> Option<Color> {
    ((x == 7 || x == 8) && y >= top).then(|| STEM_GREEN * (0.85 + 0.2 * hash(x, y, 41)))
}

// Allium: una bola morada arriba de un tallo alto (las flores lilas de la foto)
fn allium(x: usize, y: usize) -> Option<Color> {
    let (dx, dy) = (x as f32 - 7.5, y as f32 - 4.5);
    if dx * dx + dy * dy < 12.0 {
        Some(ALLIUM_PURPLE * (0.75 + 0.4 * hash(x, y, 42)))
    } else {
        stem(x, y, 7)
    }
}

// Amapola: petalos rojos con el centro oscuro
fn poppy(x: usize, y: usize) -> Option<Color> {
    let petals = (5..=10).contains(&x) && (5..=9).contains(&y);
    let center = (7..=8).contains(&x) && (6..=7).contains(&y);
    if center {
        Some(Color::from_rgb(40, 20, 16))
    } else if petals {
        Some(POPPY_RED * (0.8 + 0.3 * hash(x, y, 43)))
    } else {
        stem(x, y, 10)
    }
}

// Diente de leon: una cabeza amarilla pequena
fn dandelion(x: usize, y: usize) -> Option<Color> {
    let head = (6..=9).contains(&x) && (8..=10).contains(&y);
    if head {
        Some(DANDELION_YELLOW * (0.85 + 0.25 * hash(x, y, 44)))
    } else {
        stem(x, y, 11)
    }
}

// Pasto alto: hojas de distinta altura en columnas alternadas
fn grass_tuft(x: usize, y: usize) -> Option<Color> {
    let height = 5 + (hash(x, 0, 45) * 9.0) as usize;
    let is_blade = x % 2 == (x / 4) % 2 && y >= TEXTURE_SIZE - height;
    is_blade.then(|| TUFT_GREEN * (0.75 + 0.35 * hash(x, y, 46)))
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
    pub netherrack: BlockTextures,
    pub magma: BlockTextures,
    pub allium: BlockTextures,
    pub poppy: BlockTextures,
    pub dandelion: BlockTextures,
    pub grass_tuft: BlockTextures,
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
            netherrack: same_on_all_faces(netherrack).with_normal_map(STONE_RELIEF),
            magma: same_on_all_faces(magma),
            allium: plant(allium),
            poppy: plant(poppy),
            dandelion: plant(dandelion),
            grass_tuft: plant(grass_tuft),
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
    fn los_sprites_tienen_partes_transparentes() {
        let flower = &textures().allium.side;
        // La esquina de arriba esta vacia y el tallo abajo al centro existe
        assert!(!flower.is_opaque(0.02, 0.02));
        assert!(flower.is_opaque(7.5 / 16.0, 14.5 / 16.0));
        // Un bloque normal es opaco en todos lados
        assert!(textures().stone.side.is_opaque(0.02, 0.02));
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
