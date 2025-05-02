use std::thread;
use std::time::{Duration, Instant};

use emu::{Canvas, Cpu, NesBus};
use sdl3::event::Event;
use sdl3::keyboard::Keycode;
use sdl3::pixels::Color;

const WIDTH: u32 = 256;
const HEIGHT: u32 = 240;
const SCALING_FACTOR: u16 = 4;

// 1s / 60
const FRAME_TIME: Duration = Duration::from_nanos(16_666_667);

fn main() -> Result<(), anyhow::Error> {
    let mut bus = NesBus::new(&std::fs::read("./resources/dk.nes")?)?;
    let cpu = Cpu::new(&mut bus);
    let mut emu = Emulator {
        bus,
        cpu,
        ppu_cycle_count: 0,
    };

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
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
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
    canvas.set_scale(f32::from(SCALING_FACTOR), f32::from(SCALING_FACTOR))?;
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();

    let mut start_time: Instant;
    let mut event_pump = sdl_context.event_pump()?;
    loop {
        start_time = Instant::now();

        emu.bus.controllers[0] = 0;
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => return Ok(()),

                Event::KeyDown {
                    keycode: Some(Keycode::Right),
                    ..
                } => emu.bus.controllers[0] |= 1 << 0,
                Event::KeyDown {
                    keycode: Some(Keycode::Left),
                    ..
                } => emu.bus.controllers[0] |= 1 << 1,
                Event::KeyDown {
                    keycode: Some(Keycode::Down),
                    ..
                } => emu.bus.controllers[0] |= 1 << 2,
                Event::KeyDown {
                    keycode: Some(Keycode::Up),
                    ..
                } => emu.bus.controllers[0] |= 1 << 3,
                Event::KeyDown {
                    keycode: Some(Keycode::C),
                    ..
                } => emu.bus.controllers[0] |= 1 << 4,
                Event::KeyDown {
                    keycode: Some(Keycode::V),
                    ..
                } => emu.bus.controllers[0] |= 1 << 5,
                Event::KeyDown {
                    keycode: Some(Keycode::Z),
                    ..
                } => emu.bus.controllers[0] |= 1 << 6,
                Event::KeyDown {
                    keycode: Some(Keycode::X),
                    ..
                } => emu.bus.controllers[0] |= 1 << 7,

                _ => {}
            }
        }

        let mut canvas_for_ppu = MyCanvas(canvas);
        loop {
            emu.tick(&mut canvas_for_ppu);
            if emu.bus.frame_complete() {
                emu.bus.set_frame_complete(false);
                break;
            }
        }
        MyCanvas(canvas) = canvas_for_ppu;
        canvas.present();

        let elapsed = start_time.elapsed();
        if FRAME_TIME.saturating_sub(elapsed) > Duration::ZERO {
            thread::sleep(FRAME_TIME - elapsed);
        }
    }
}

#[derive(Debug)]
struct Emulator {
    bus: NesBus,
    cpu: Cpu,
    ppu_cycle_count: u64,
}

impl Emulator {
    fn tick(&mut self, canvas: &mut impl Canvas) {
        self.bus.cycle(canvas);
        if self.ppu_cycle_count % 3 == 0 {
            self.cpu.cycle(&mut self.bus);
        }
        if self.bus.ack_nmi() {
            self.cpu.nmi(&mut self.bus);
        }

        self.ppu_cycle_count += 1;
    }
}

struct MyCanvas(sdl3::render::WindowCanvas);

impl Canvas for MyCanvas {
    fn draw_point(&mut self, x: u32, y: u32, rgb: (u8, u8, u8)) {
        self.0.set_draw_color(rgb);
        self.0
            .draw_point((x, y))
            .expect("sdl3 drawing point should work");
    }
}
