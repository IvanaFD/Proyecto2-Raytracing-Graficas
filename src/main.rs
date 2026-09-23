mod color;
mod framebuffer;
mod vec3;

use raylib::prelude::TraceLogLevel;

use color::Color;
use framebuffer::Framebuffer;
use vec3::Vec3;

const WINDOW_WIDTH: i32 = 800;
const WINDOW_HEIGHT: i32 = 600;
const TITLE: &str = "Diorama Minecraft - Raytracer";

const SKY_HORIZON: Color = Color::from_rgb(200, 225, 255);
const SKY_ZENITH: Color = Color::from_rgb(60, 120, 215);

fn sky_color(direction: &Vec3) -> Color {
    let t = 0.5 * (direction.y + 1.0);
    SKY_HORIZON.lerp(&SKY_ZENITH, t)
}

fn render(framebuffer: &mut Framebuffer) {
    let width = framebuffer.width as f32;
    let height = framebuffer.height as f32;
    let aspect_ratio = width / height;

    for y in 0..framebuffer.height {
        for x in 0..framebuffer.width {
            let screen_x = (2.0 * x as f32) / width - 1.0;
            let screen_y = -(2.0 * y as f32) / height + 1.0;
            let screen_x = screen_x * aspect_ratio;

            let ray_direction = Vec3::new(screen_x, screen_y, -1.0).normalize();

            let pixel_color = sky_color(&ray_direction);
            framebuffer.set_pixel(x, y, pixel_color.to_bytes());
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

    render(&mut framebuffer);

    while !window.window_should_close() {
        framebuffer.present(&mut window, &raylib_thread);
    }
}
