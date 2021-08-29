extern crate rand;
extern crate sdl2;

use std::fs;

mod chip8;
mod display;
mod fontset;

fn main() {
    let mut chip8 = chip8::Chip8::new(&fontset::FONT_SET);
    let path = std::env::args().nth(1);
    if path.is_none() {
        panic!("No game defined!");
    }

    let data = fs::read(path.unwrap());
    if data.is_err() {
        panic!("Game not found!");
    }

    chip8.load_rom(&data.unwrap());
    chip8.start();
}
