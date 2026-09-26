use raylib::prelude::*;

// R, G, B, A
pub const CHANNELS: usize = 4;

pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
    texture: Texture2D,
    // Hay pixeles nuevos pendientes de subir a la textura
    dirty: bool,
}

impl Framebuffer {
    pub fn new(
        window: &mut RaylibHandle,
        raylib_thread: &RaylibThread,
        width: usize,
        height: usize,
    ) -> Self {
        let image = Image::gen_image_color(width as i32, height as i32, Color::BLACK);
        let texture = window
            .load_texture_from_image(raylib_thread, &image)
            .expect("no se pudo crear la textura del framebuffer");

        // Al escalar, los pixeles se ven nitidos en lugar de borrosos
        texture.set_texture_filter(raylib_thread, TextureFilter::TEXTURE_FILTER_POINT);

        Framebuffer {
            width,
            height,
            pixels: vec![255; width * height * CHANNELS],
            texture,
            dirty: true,
        }
    }

    // Acceso directo a los bytes RGBA para que el render los reparta entre hilos
    pub fn pixels_mut(&mut self) -> &mut [u8] {
        self.dirty = true;
        &mut self.pixels
    }

    #[allow(dead_code)]
    pub fn clear(&mut self, rgb: [u8; 3]) {
        for pixel in self.pixels.chunks_exact_mut(CHANNELS) {
            pixel.copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
        }
        self.dirty = true;
    }

    pub fn present(&mut self, window: &mut RaylibHandle, raylib_thread: &RaylibThread) {
        if self.dirty {
            self.texture
                .update_texture(&self.pixels)
                .expect("el tamano del framebuffer no coincide con la textura");
            self.dirty = false;
        }

        let screen_width = window.get_screen_width() as f32;
        let screen_height = window.get_screen_height() as f32;

        let mut render = window.begin_drawing(raylib_thread);
        render.clear_background(Color::BLACK);

        let source_rec = Rectangle::new(0.0, 0.0, self.width as f32, self.height as f32);
        let dest_rec = Rectangle::new(0.0, 0.0, screen_width, screen_height);

        render.draw_texture_pro(
            &self.texture,
            source_rec,
            dest_rec,
            Vector2::new(0.0, 0.0),
            0.0,
            Color::WHITE,
        );
        render.draw_fps(10, 10);
    }
}
