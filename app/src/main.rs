use std::thread;
use std::time::Duration;
use emu::{Canvas, NesBus, Ppu};
use sdl3::event::Event;
use sdl3::keyboard::Keycode;
use sdl3::pixels::Color;

const WIDTH: u32 = 256;
const HEIGHT: u32 = 240;
const SCALING_FACTOR: u16 = 4;

fn main() -> Result<(), anyhow::Error> {
    let sdl_context = sdl3::init()?;
    let video_subsystem = sdl_context.video()?;

    let mut window = video_subsystem
        .window(
            "nes",
            WIDTH * u32::from(SCALING_FACTOR),
            HEIGHT * u32::from(SCALING_FACTOR),
        )
        .position_centered()
        .high_pixel_density()
        .build()?;

    // Fix window sizing on macOS. See https://wiki.libsdl.org/SDL3/README/highdpi.
    let integer_pixel_density = {
        let p = window.pixel_density();
        if p.fract() != 0.0 {
            eprintln!("warning: pixel density {p} is non-integer.");
        }
        p as u32
    };
    let (w, h) = window.size();
    window.set_size(w / integer_pixel_density, h / integer_pixel_density)?;

    let mut canvas = window.into_canvas();
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();
    canvas.set_scale(f32::from(SCALING_FACTOR), f32::from(SCALING_FACTOR))?;

    let bus = NesBus::new(&std::fs::read(
        "./emu/resources/Donkey Kong (World) (Rev 1).nes",
    )?)?;
    let ppu = Ppu::new(&bus);
    let mut event_pump = sdl_context.event_pump()?;

    loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => return Ok(()),
                _ => {}
            }
        }

        let mut canvas_for_ppu = MyCanvas(canvas);
        ppu.draw_tiles(&mut canvas_for_ppu);
        MyCanvas(canvas) = canvas_for_ppu;
        canvas.present();

        thread::sleep(Duration::from_secs(1) / 60);
    }
}

struct MyCanvas(sdl3::render::WindowCanvas);

impl Canvas for MyCanvas {
    fn draw_point(&mut self, x: u32, y: u32, (r, g, b): (u8, u8, u8)) {
        self.0.set_draw_color((r, g, b));
        self.0
            .draw_point((x, y))
            .expect("sdl3 drawing point should work");
    }
}
