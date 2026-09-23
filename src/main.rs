mod framebuffer;

use raylib::prelude::*;

use framebuffer::Framebuffer;

const WINDOW_WIDTH: i32 = 800;
const WINDOW_HEIGHT: i32 = 600;
const TITLE: &str = "Diorama Minecraft - Raytracer";

fn render_gradient(framebuffer: &mut Framebuffer) {
    let width = framebuffer.width as f32;
    let height = framebuffer.height as f32;

    for y in 0..framebuffer.height {
        for x in 0..framebuffer.width {
            let r = (x as f32 / width * 255.0) as u8;
            let g = (y as f32 / height * 255.0) as u8;
            let b = 64;

            framebuffer.set_pixel(x, y, [r, g, b]);
        }
    }
}

fn main() {
    let (mut window, raylib_thread) = raylib::init()
        .size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .title(TITLE)
        .log_level(TraceLogLevel::LOG_WARNING)
        .build();

    window.set_target_fps(60);

    let mut framebuffer = Framebuffer::new(
        &mut window,
        &raylib_thread,
        WINDOW_WIDTH as usize,
        WINDOW_HEIGHT as usize,
    );

    render_gradient(&mut framebuffer);

    while !window.window_should_close() {
        framebuffer.present(&mut window, &raylib_thread);
    }
}
