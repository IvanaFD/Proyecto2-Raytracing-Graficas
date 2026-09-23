use raylib::prelude::*;

const WINDOW_WIDTH: i32 = 800;
const WINDOW_HEIGHT: i32 = 600;
const TITLE: &str = "Diorama Minecraft - Raytracer";

fn main() {
    let (mut window, raylib_thread) = raylib::init()
        .size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .title(TITLE)
        .log_level(TraceLogLevel::LOG_WARNING)
        .build();

    window.set_target_fps(60);

    while !window.window_should_close() {
        let mut render = window.begin_drawing(&raylib_thread);
        render.clear_background(Color::new(4, 12, 36, 255));
    }
}
