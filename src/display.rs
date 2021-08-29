use sdl2;
use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::EventPump;

const WIDTH: u32 = 64;
const HEIGHT: u32 = 32;

const PIXEL_SCALE: u32 = 8;
const SCR_WIDTH: u32 = WIDTH * PIXEL_SCALE;
const SCR_HEIGHT: u32 = HEIGHT * PIXEL_SCALE;

pub struct Display {
    canvas: Canvas<Window>,
    events: EventPump,
    audio: AudioDevice<SquareWave>,
}

impl Display {
    pub fn new() -> Self {
        let sdl_context = sdl2::init().unwrap();
        let video_subsys = sdl_context.video().unwrap();
        let window = video_subsys
            .window("Chip-8 Emulator", SCR_WIDTH, SCR_HEIGHT)
            .position_centered()
            .opengl()
            .build()
            .map_err(|e| e.to_string())
            .unwrap();

        let mut canvas = window
            .into_canvas()
            .build()
            .map_err(|e| e.to_string())
            .unwrap();

        canvas.set_draw_color(pixels::Color::RGB(0, 0, 0));
        canvas.clear();
        canvas.present();

        let audio_subsystem = sdl_context.audio().unwrap();

        let desired_spec = AudioSpecDesired {
            freq: Some(44100),
            channels: Some(1), // mono
            samples: None,     // default sample size
        };

        let device = audio_subsystem
            .open_playback(None, &desired_spec, |spec| {
                // Show obtained AudioSpec
                println!("{:?}", spec);

                // initialize the audio callback
                SquareWave {
                    phase_inc: 240.0 / spec.freq as f32,
                    phase: 0.0,
                    volume: 0.25,
                }
            })
            .unwrap();

        device.resume();

        Self {
            canvas,
            events: sdl_context.event_pump().unwrap(),
            audio: device,
        }
    }

    pub fn draw(&mut self, pixels: &[[u8; WIDTH as usize]; HEIGHT as usize]) {
        for (y, row) in pixels.iter().enumerate() {
            for (x, &col) in row.iter().enumerate() {
                let x = (x as u32) * PIXEL_SCALE;
                let y = (y as u32) * PIXEL_SCALE;

                let color = if col == 0 {
                    pixels::Color::RGB(0, 0, 0)
                } else {
                    pixels::Color::RGB(210, 210, 210)
                };

                self.canvas.set_draw_color(color);

                let _ =
                    self.canvas
                        .fill_rect(Rect::new(x as i32, y as i32, PIXEL_SCALE, PIXEL_SCALE));
            }
        }
        self.canvas.present();
    }

    pub fn update_keypad(&mut self) -> [bool; 16] {
        let mut keypad = [false; 16];

        for event in self.events.poll_iter() {
            if let Event::Quit { .. } = event {
                std::process::exit(0);
            };
        }

        let keys: Vec<Keycode> = self
            .events
            .keyboard_state()
            .pressed_scancodes()
            .filter_map(Keycode::from_scancode)
            .collect();

        for key in keys {
            let index = match key {
                Keycode::Num1 => Some(0x1),
                Keycode::Num2 => Some(0x2),
                Keycode::Num3 => Some(0x3),
                Keycode::Num4 => Some(0xc),
                Keycode::Q => Some(0x4),
                Keycode::W => Some(0x5),
                Keycode::E => Some(0x6),
                Keycode::R => Some(0xd),
                Keycode::A => Some(0x7),
                Keycode::S => Some(0x8),
                Keycode::D => Some(0x9),
                Keycode::F => Some(0xe),
                Keycode::Z => Some(0xa),
                Keycode::X => Some(0x0),
                Keycode::C => Some(0xb),
                Keycode::V => Some(0xf),
                _ => None,
            };

            if let Some(i) = index {
                keypad[i] = true;
            }
        }

        keypad
    }

    pub fn start_audio(&self) {
        self.audio.resume();
    }

    pub fn stop_audio(&self) {
        self.audio.pause();
    }
}

struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32,
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        // Generate a square wave
        for x in out.iter_mut() {
            *x = if self.phase <= 0.5 {
                self.volume
            } else {
                -self.volume
            };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}
