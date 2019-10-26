extern crate minifb;
extern crate rand;

use minifb::{Key, WindowOptions, Window};
use std::time::SystemTime;
use std::thread::sleep;
use rand::prelude::*;
use std::io::prelude::*;
use std::fs::File;

pub const FONT_SET: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90,
    0xF0, 0x20, 0x60, 0x20,
    0x20, 0x70, 0xF0, 0x10,
    0xF0, 0x80, 0xF0, 0xF0,
    0x10, 0xF0, 0x10, 0xF0,
    0x90, 0x90, 0xF0, 0x10,
    0x10, 0xF0, 0x80, 0xF0,
    0x10, 0xF0, 0xF0, 0x80,
    0xF0, 0x90, 0xF0, 0xF0,
    0x10, 0x20, 0x40, 0x40,
    0xF0, 0x90, 0xF0, 0x90,
    0xF0, 0xF0, 0x90, 0xF0,
    0x10, 0xF0, 0xF0, 0x90,
    0xF0, 0x90, 0x90, 0xE0,
    0x90, 0xE0, 0x90, 0xE0,
    0xF0, 0x80, 0x80, 0x80,
    0xF0, 0xE0, 0x90, 0x90,
    0x90, 0xE0, 0xF0, 0x80,
    0xF0, 0x80, 0xF0, 0xF0,
    0x80, 0xF0, 0x80, 0x80,
];

struct chip8 {
    memory: [u8; 0xFFF],
    stack: [u16; 16],
    V: [u8; 16],
    gfx: [bool; 64*32],
    delay_timer: u8,
    sound_timer: u8,
    pc: u16,
    sp: u8,
    I: u16,

    waiting_for_key: bool,
}

impl chip8 {
    fn new() -> chip8 {
        let mut ret = chip8 {
            memory: [0; 0xFFF],
            stack: [0; 16],
            V: [0; 16],
            gfx: [false; 64*32],
            delay_timer: 0,
            sound_timer: 0,
            pc: 0x200,
            sp: 0,
            I: 0,

            waiting_for_key: false,
        };

        for (i, b) in FONT_SET.iter().enumerate() {
            ret.memory[i] = *b;
        }

        ret
    }

    fn process_instruction(&mut self) {
        let nibbles = (
            self.memory[self.pc as usize] & 0xF0 >> 4,
            self.memory[self.pc as usize] & 0x0F as u8,
            self.memory[self.pc as usize + 1] & 0xF0 >> 4,
            self.memory[self.pc as usize + 1] & 0x0F as u8, 
        );

        let nnn = (self.memory[self.pc as usize] as u16 & 0x0F) << 8 & (self.memory[self.pc as usize + 1] as u16);
        let kk = self.memory[self.pc as usize + 1];
        let x = (self.memory[self.pc as usize] as u16 & 0x0F) as usize;
        let y = (self.memory[self.pc as usize + 1] as u16 & 0xF0) as usize;
        let n = self.memory[self.pc as usize + 1] as u16 & 0x0F;

        print!("{}: ", self.pc);

        match nibbles {
            (0x0, 0x0, 0x0, 0x0) => println!("NOP"),
            (0x0, 0x0, 0xE, 0x0) => {
                println!("CLS");
                for pixel in self.gfx.iter_mut() {
                    *pixel = false;
                }
            },
            (0x0, 0x0, 0xE, 0xE) => {
                println!("RET");
                self.pc = self.stack[self.sp as usize];
                self.sp -= 1;
            },
            (0x1, _, _, _) => {
                println!("JP addr");
                self.pc = nnn;
            },
            (0x2, _, _, _) => {
                println!("CALL");
                self.sp += 1;
                self.stack[self.sp as usize] = self.pc;
                self.pc = nnn;
            },
            (0x3, _, _, _) => {
                println!("SE Vx, byte");
                if self.V[x] == kk {
                    self.pc += 2;
                }
            },
            (0x4, _, _, _) => {
                println!("SNE Vx, byte");
                if self.V[x] != kk {
                    self.pc += 2;
                }
            },
            (0x5, _, _, _) => {
                println!("SE Vx, Vy");
                if self.V[x] == self.V[y] {
                    self.pc += 2;
                }
            },
            (0x6, _, _, _) => {
                println!("LD Vx, byte");
                self.V[x] = kk;
            },
            (0x7, _, _, _) => {
                println!("ADD Vx, byte");
                self.V[x] += kk;
            },
            (0x8, _, _, 0x0) => {
                println!("LD Vx, Vy");
                self.V[x] = self.V[y];
            },
            (0x8, _, _, 0x1) => {
                println!("OR Vx, Vy");
                self.V[x] |= self.V[y];
            },
            (0x8, _, _, 0x2) => {
                println!("AND Vx, Vy");
                self.V[x] &= self.V[y];
            },
            (0x8, _, _, 0x3) => {
                println!("XOR Vx, Vy");
                self.V[x] ^= self.V[y];
            },
            (0x8, _, _, 0x4) => {
                println!("ADD Vx, Vy");
                self.V[x] += match self.V[x].checked_add(self.V[y]) {
                    Some(val) => val,
                    None => {
                        self.V[0xF] = 1;
                        self.V[x].wrapping_add(self.V[y])
                    },
                }
            },
            (0x8, _, _, 0x5) => {
                println!("SUB Vx, Vy");
                self.V[0xF] = if self.V[x] > self.V[y] { 1 } else { 0 };
                self.V[x] = self.V[x].wrapping_sub(self.V[y]);
            },
            (0x8, _, _, 0x6) => {
                println!("SHR Vx {{, Vy}}");
                self.V[0xF] = if self.V[x] & 0x1 == 0 { 1 } else { 0 };
                self.V[x].overflowing_shr(1).0;
            },
            (0x8, _, _, 0x7) => {
                println!("SUBN Vx, Vy");
                self.V[0xF] = if self.V[y] > self.V[x] { 1 } else { 0 };
                self.V[x] = self.V[y].wrapping_sub(self.V[x]);
            },
            (0x8, _, _, 0xE) => {
                println!("SHL Vx {{,Vy}}");
                self.V[0xF] = if self.V[x] & 0x1 == 0 { 1 } else { 0 };
                self.V[x].overflowing_shl(1).0;
            },
            (0x9, _, _, 0x0) => {
                println!("SNE Vx, Vy");
                if self.V[x] != self.V[y] {
                    self.pc += 2;
                }
            },
            (0xA, _, _, _) => {
                println!("LD I, addr");
                self.I = nnn;
            },
            (0xB, _, _, _) => {
                println!("JP V0, addr");
                self.pc = nnn + self.V[0] as u16;
            },
            (0xC, _, _, _) => {
                println!("RND Vx, byte");
                let r = rand::random::<u8>();
                self.V[x] = r & kk;
            },
            (0xD, _, _, _) => {
                println!("DRW Vx, Vy, nibble");

                // TODO deal with collision detection
                let sprite_bytes = &self.memory[self.I .. self.I + n];
                let x = self.V[x];
                let y = self.V[y];

                for x in 0..n {
                    for pos in 0..8 {
                        pixel = self.gfx[]
                    }
                }
            },
            (0xE, _, 0x9, 0xE) => {
                println!("SKP Vx");
            },
            (0xE, _, 0xA, 0x1) => {
                println!("SKNP Vx");
            },
            (0xF, _, 0x0, 0x7) => {
                println!("LD Vx, DT");
                self.V[x] = self.delay_timer;
            },
            (0xF, _, 0x0, 0xA) => {
                println!("LD Vx, k");
                while self.waiting_for_key {};
            },
            (0xF, _, 0x1, 0x5) => {
                println!("LD DT, Vx");
                self.delay_timer = self.V[x];
            },
            (0xF, _, 0x1, 0x8) => {
                println!("LD ST, Vx");
                self.sound_timer = self.V[x];
            },
            (0xF, _, 0x1, 0xE) => {
                println!("ADD I, Vx");
                self.I += self.V[x] as u16;
            },
            (0xF, _, 0x2, 0x9) => {
                println!("LD F, Vx");
                
            },
            (0xF, _, 0x3, 0x3) => println!("LD B, Vx"),
            (0xF, _, 0x5, 0x5) => println!("LD [I], Vx"),
            (0xF, _, 0x6, 0x5) => println!("LD Vx, [I]"),

            _ => println!("Unknown opcode: {:?}", nibbles),
        }

        self.pc += 2;
    }

    fn load_rom(&mut self, filename: &str) {
        let f = File::open(filename).unwrap();

        for (i, byte) in f.bytes().enumerate() {
            self.memory[i + 0x200] = byte.unwrap();
        }
    }
}

const WIDTH: usize = 640;
const HEIGHT: usize = 320;

fn main() {
    let mut c8 = chip8::new();
    c8.load_rom("roms/demos/Maze (alt) [David Winter, 199x].ch8");

    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
    let mut window = Window::new("Chip8 - Emulator - Rust",
        WIDTH, HEIGHT, WindowOptions::default()).unwrap();
    
    let mut now = SystemTime::now();
    let mut last = now;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        now = SystemTime::now();
        let delta = now.duration_since(last).unwrap().as_millis() as f32 / 1000.0;
        last = now;

        window.set_title(&format!("{}", 1.0 / delta));

        c8.process_instruction();

        for (i, pixel) in buffer.iter_mut().enumerate() {
            let x: usize = i % WIDTH / 10;
            let y: usize = i / WIDTH / 10;

            *pixel = if c8.gfx[y * 64 + x] {0xFFFFFF} else {0}
        }

        window.get_keys().map(|keys| {
            for t in keys {
                match t {
                    Key::Up => println!("Up"),
                    Key::Down => println!("Down"),
                    Key::Left => println!("Left"),
                    Key::Right => println!("Right"),
                    _ => println!("Key: {:?}", t),
                }
            }
        });

        window.update_with_buffer(&buffer).unwrap();
    }
}
