use rand::{thread_rng, Rng};

use std::{time::Duration, usize};

use crate::display;

const WIDTH: usize = 64;
const HEIGHT: usize = 32;
const RAM_SIZE: usize = 4096;

pub struct Chip8 {
    pc: usize,           // program counter
    op: u16,             // current opcode (two bytes)
    ir: usize,           // index register
    sp: usize,           // stack pointer
    delay_timer: u8,     // timer registers that count at 50 hz
    sound_timer: u8,     // ^
    registers: [u8; 16], // 15 general-purpose registers + carry
    keypad: [bool; 16],  // current state of each key pressed
    ram: [u8; RAM_SIZE],
    vram: [[u8; WIDTH]; HEIGHT],
    stack: [usize; 16],
    draw_flag: bool,
    display: display::Display,
}

impl Chip8 {
    pub fn new(fontset: &[u8]) -> Self {
        let mut ram = [0; RAM_SIZE];
        for i in 0..fontset.len() {
            ram[i] = fontset[i];
        }

        Self {
            pc: 0x200, // initial pc value, lower bytes are reserved for font data
            op: 0x0,
            ir: 0x0,
            sp: 0x0,
            ram,
            vram: [[0; WIDTH]; HEIGHT],
            registers: [0; 16],
            keypad: [false; 16],
            delay_timer: 0,
            sound_timer: 0,
            stack: [0; 16],
            draw_flag: false,
            display: display::Display::new(),
        }
    }

    pub fn load_rom(&mut self, data: &[u8]) {
        for (i, &byte) in data.iter().enumerate() {
            let addr_shifted = i + 0x200;
            // check if it is a valid ram location
            if addr_shifted < 4096 {
                self.ram[addr_shifted] = byte;
            }
        }
    }

    pub fn start(&mut self) {
        loop {
            self.keypad = self.display.update_keypad();
            //eprintln!("{:?}", self.keypad);

            if self.draw_flag {
                self.display.draw(&self.vram);
            }
            self.draw_flag = false;

            if self.delay_timer > 0 {
                self.delay_timer -= 1;
            }

            if self.sound_timer > 0 {
                println!("start audio");
                self.display.start_audio();
                self.sound_timer -= 1;
            } else {
                self.display.stop_audio();
            }

            self.cycle();
            std::thread::sleep(Duration::from_millis(4));
        }
    }

    fn cycle(&mut self) {
        self.fetch();
        self.decode_execute();
    }

    fn fetch(&mut self) {
        self.op = (self.ram[self.pc] as u16) << 8 | self.ram[self.pc + 1] as u16;
        //eprintln!("op: {:#x}, pc: {:#x}", self.op, self.pc);
        self.pc += 2;
    }

    fn decode_execute(&mut self) {
        let hex = (
            ((self.op & 0xF000) >> 12) as u8,
            ((self.op & 0x0F00) >> 8) as u8,
            ((self.op & 0x00F0) >> 4) as u8,
            (self.op & 0x000F) as u8,
        );

        let nnn = self.op & 0x0FFF;
        let lower_byte = (self.op & 0xFF) as u8;
        let x = hex.1 as usize;
        let y = hex.2 as usize;
        let n = hex.3 as usize;

        match hex {
            // CLS
            (0x00, 0x00, 0x0e, 0x00) => self.op_00e0(),
            // RET
            (0x00, 0x00, 0x0e, 0x0e) => self.op_00ee(),
            // JP addr
            (0x01, _, _, _) => self.op_1nnn(nnn),
            // CALL addr
            (0x02, _, _, _) => self.op_2nnn(nnn),
            // SE Vx, byte
            (0x03, _, _, _) => self.op_3xkk(x, lower_byte),
            // SNE Vx, byte
            (0x04, _, _, _) => self.op_4xkk(x, lower_byte),
            // SE Vx, Vy
            (0x05, _, _, 0x00) => self.op_5xy0(x, y),
            // LD Vx, byte
            (0x06, _, _, _) => self.op_6xkk(x, lower_byte),
            // ADD Vx, byte
            (0x07, _, _, _) => self.op_7xkk(x, lower_byte),
            // LD Vx, Vy
            (0x08, _, _, 0x00) => self.op_8xy0(x, y),
            // OR Vx, Vy
            (0x08, _, _, 0x01) => self.op_8xy1(x, y),
            // AND Vx, Vy
            (0x08, _, _, 0x02) => self.op_8xy2(x, y),
            // XOR Vx, Vy
            (0x08, _, _, 0x03) => self.op_8xy3(x, y),
            // ADD Vx, Vy
            (0x08, _, _, 0x04) => self.op_8xy4(x, y),
            // SUB Vx, Vy
            (0x08, _, _, 0x05) => self.op_8xy5(x, y),
            // SHR Vx {, Vy}
            (0x08, _, _, 0x06) => self.op_8xy6(x),
            // SUBN Vx, Vy
            (0x08, _, _, 0x07) => self.op_8xy7(x, y),
            // SHL Vx {, Vy}
            (0x08, _, _, 0x0e) => self.op_8xye(x),
            // SNE Vx, Vy
            (0x09, _, _, 0x00) => self.op_9xy0(x, y),
            // LD I, addr
            (0x0a, _, _, _) => self.op_annn(nnn),
            // JP V0, addr
            (0x0b, _, _, _) => self.op_bnnn(nnn),
            // RND Vx, byte
            (0x0c, _, _, _) => self.op_cxkk(x, lower_byte),
            // DRW Vx, Vy, nibble
            (0x0d, _, _, _) => self.op_dxyn(x, y, n),
            // SKP Vx
            (0x0e, _, 0x09, 0x0e) => self.op_ex9e(x),
            // SKNP Vx
            (0x0e, _, 0x0a, 0x01) => self.op_exa1(x),
            // LD Vx, DT
            (0x0f, _, 0x00, 0x07) => self.op_fx07(x),
            // LD Vx, K
            (0x0f, _, 0x00, 0x0a) => self.op_fx0a(x),
            // LD DT, Vx
            (0x0f, _, 0x01, 0x05) => self.op_fx15(x),
            // LD ST, Vx
            (0x0f, _, 0x01, 0x08) => self.op_fx18(x),
            // ADD I, Vx
            (0x0f, _, 0x01, 0x0e) => self.op_fx1e(x),
            // LD F, Vx
            (0x0f, _, 0x02, 0x09) => self.op_fx29(x),
            // LD B, Vx
            (0x0f, _, 0x03, 0x03) => self.op_fx33(x),
            // LD [I], Vx
            (0x0f, _, 0x05, 0x05) => self.op_fx55(x),
            // LD Vx, [I]
            (0x0f, _, 0x06, 0x05) => self.op_fx65(x),
            // NOP
            _ => (),
        }
    }

    // thanks cowgod!!! http://devernay.free.fr/hacks/chip8/C8TECH10.HTM
    // and https://github.com/starrhorne/chip8-rust/

    // Clear the display.
    fn op_00e0(&mut self) {
        for i in 0..HEIGHT {
            for j in 0..WIDTH {
                self.vram[i][j] = 0;
            }
        }
        self.draw_flag = true;
    }

    // Return from a subroutine.
    fn op_00ee(&mut self) {
        self.sp -= 1;
        self.pc = self.stack[self.sp];
    }

    // Jump to location nnn.
    fn op_1nnn(&mut self, nnn: u16) {
        self.pc = nnn as usize;
    }

    // Call subroutine at nnn.
    fn op_2nnn(&mut self, nnn: u16) {
        self.stack[self.sp] = self.pc;
        self.sp += 1;
        self.pc = nnn as usize;
    }

    // Skip next instruction if Vx = kk.
    fn op_3xkk(&mut self, x: usize, kk: u8) {
        if self.registers[x] == kk {
            self.pc += 2;
        }
    }

    // Skip next instruction if Vx != kk.
    fn op_4xkk(&mut self, x: usize, kk: u8) {
        if self.registers[x] != kk {
            self.pc += 2;
        }
    }

    // Skip next instruction if Vx = Vy.
    fn op_5xy0(&mut self, x: usize, y: usize) {
        if self.registers[x] == self.registers[y] {
            self.pc += 2;
        }
    }

    // Set Vx = kk.
    fn op_6xkk(&mut self, x: usize, kk: u8) {
        self.registers[x] = kk;
    }

    // Set Vx = Vx + kk.
    fn op_7xkk(&mut self, x: usize, kk: u8) {
        self.registers[x] = self.registers[x].wrapping_add(kk);
    }

    // Set Vx = Vy.
    fn op_8xy0(&mut self, x: usize, y: usize) {
        self.registers[x] = self.registers[y];
    }

    // Set Vx = Vx OR Vy.
    fn op_8xy1(&mut self, x: usize, y: usize) {
        self.registers[x] |= self.registers[y];
    }

    // Set Vx = Vx AND Vy.
    fn op_8xy2(&mut self, x: usize, y: usize) {
        self.registers[x] &= self.registers[y];
    }

    // Set Vx = Vx XOR Vy.
    fn op_8xy3(&mut self, x: usize, y: usize) {
        self.registers[x] ^= self.registers[y];
    }

    // Set Vx = Vx + Vy, set VF = carry.
    fn op_8xy4(&mut self, x: usize, y: usize) {
        let sum = self.registers[x] as usize + self.registers[y] as usize;
        self.registers[x] = (sum & 0xFF) as u8;
        self.registers[15] = if sum > 0xFF { 1 } else { 0 };
    }

    // If Vx > Vy, then VF is set to 1, otherwise 0. Then Vy is subtracted from Vx, and the results stored in Vx.
    fn op_8xy5(&mut self, x: usize, y: usize) {
        self.registers[15] = (self.registers[x] > self.registers[y]) as u8;
        self.registers[x] = self.registers[x].wrapping_sub(self.registers[y]);
    }

    // If the least-significant bit of Vx is 1, then VF is set to 1, otherwise 0. Then Vx is divided by 2.
    fn op_8xy6(&mut self, x: usize) {
        self.registers[15] = self.registers[x] & 0b1;
        self.registers[x] >>= 1;
    }

    // If Vy > Vx, then VF is set to 1, otherwise 0. Then Vx is subtracted from Vy, and the results stored in Vx.
    fn op_8xy7(&mut self, x: usize, y: usize) {
        self.registers[15] = (self.registers[x] < self.registers[y]) as u8;
        self.registers[x] = self.registers[y].wrapping_sub(self.registers[x]);
    }

    // If the most-significant bit of Vx is 1, then VF is set to 1, otherwise to 0. Then Vx is multiplied by 2.
    fn op_8xye(&mut self, x: usize) {
        self.registers[15] = (self.registers[x] & 0b10000000) >> 7;
        self.registers[x] <<= 1;
    }

    // Skip next instruction if Vx != Vy.
    fn op_9xy0(&mut self, x: usize, y: usize) {
        if self.registers[x] != self.registers[y] {
            self.pc += 2;
        }
    }

    // Set ir = nnn.
    fn op_annn(&mut self, nnn: u16) {
        self.ir = nnn as usize;
    }

    // Jump to location nnn + V0.
    fn op_bnnn(&mut self, nnn: u16) {
        self.pc = (nnn + self.registers[0] as u16) as usize;
    }

    // Set Vx = random byte AND kk.
    fn op_cxkk(&mut self, x: usize, kk: u8) {
        let mut rng = thread_rng();
        let n: u8 = rng.gen_range(0..=255);
        self.registers[x] = n & kk;
    }

    // The interpreter reads n bytes from memory, starting at the address stored in IR.
    // These bytes are then displayed as sprites on screen at coordinates (Vx, Vy).
    // Sprites are XORed onto the existing screen.
    // If this causes any pixels to be erased, VF is set to 1, otherwise it is set to 0.
    // If the sprite is positioned so part of it is outside the coordinates of the display,
    // it wraps around to the opposite side of the screen.
    fn op_dxyn(&mut self, x: usize, y: usize, height: usize) {
        self.registers[15] = 0;
        for i in 0..height {
            let y = (self.registers[y] as usize + i) % HEIGHT;
            for j in 0..8 {
                let x = (self.registers[x] as usize + j) % WIDTH;
                let pixel = (self.ram[self.ir + i] >> (7 - j)) & 0b1;
                self.registers[15] |= pixel & self.vram[y][x];
                self.vram[y][x] ^= pixel;
            }
        }
        self.draw_flag = true;
    }

    // Skip next instruction if key with the value of Vx is pressed.
    fn op_ex9e(&mut self, x: usize) {
        if self.keypad[self.registers[x] as usize] {
            self.pc += 2;
        }
    }

    // Skip next instruction if key with the value of Vx is not pressed.
    fn op_exa1(&mut self, x: usize) {
        if !self.keypad[self.registers[x] as usize] {
            self.pc += 2;
        }
    }

    // Set Vx = delay timer value.
    fn op_fx07(&mut self, x: usize) {
        self.registers[x] = self.delay_timer;
    }

    // Wait for a key press, store the value of the key in Vx.
    fn op_fx0a(&mut self, x: usize) {
        'halt: loop {
            for key in 0..self.keypad.len() {
                if self.keypad[key] {
                    self.registers[x] = key as u8;
                    break 'halt;
                }
            }
        }
    }

    // Set delay timer = Vx.
    fn op_fx15(&mut self, x: usize) {
        self.delay_timer = self.registers[x];
    }

    // Set sound timer = Vx.
    fn op_fx18(&mut self, x: usize) {
        self.sound_timer = self.registers[x];
    }

    // Set IR = IR + Vx.
    fn op_fx1e(&mut self, x: usize) {
        self.ir = self.ir.wrapping_add(self.registers[x] as usize);
    }

    // Set I = location of sprite for digit Vx.
    fn op_fx29(&mut self, x: usize) {
        self.ir = self.registers[x] as usize * 5;
    }

    // The interpreter takes the decimal value of Vx,
    // and places the hundreds digit in memory at location in IR,
    // the tens digit at location IR+1, and the ones digit at location IR+2.
    fn op_fx33(&mut self, x: usize) {
        let n = self.registers[x];

        self.ram[self.ir] = n / 100;
        self.ram[self.ir + 1] = (n / 10) % 10;
        self.ram[self.ir + 2] = n % 10;
    }

    // Store registers V0 through Vx in memory starting at location I.
    fn op_fx55(&mut self, x: usize) {
        for i in 0..=x {
            self.ram[self.ir + i] = self.registers[i];
        }
    }

    // Read registers V0 through Vx from memory starting at location I.

    fn op_fx65(&mut self, x: usize) {
        for i in 0..=x {
            self.registers[i] = self.ram[self.ir + i];
        }
    }
}
