use std::io;
use std::io::Read;
use std::time;

mod display;
mod keypad;
mod rng;
mod stack;

const DISPLAY_WIDTH: usize = 64;
const DISPLAY_HEIGHT: usize = 32;
const MEM_SIZE: usize = 4096;
const NUM_REGS: usize = 16;
const REG_WIDTH: u8 = 8;
const NUM_OPCODE_TYPES: usize = 16;
const ROM_START_ADDR: usize = 512;
const INSTR_PER_FRAME: u8 = 10; // Online consensus for ~10 instructions/frame
const SPRITE_WIDTH: usize = 8;

// 5 bytes per hex character
const FONTS: [u8; 16 * 5] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

// Maps first half-byte of opcode (index) to function
const OPCODE_FUNCS: [fn(&mut VM); NUM_OPCODE_TYPES] = [
    VM::nib_0,
    VM::nib_1,
    VM::nib_2,
    VM::nib_3,
    VM::nib_4,
    VM::nib_5,
    VM::nib_6,
    VM::nib_7,
    VM::nib_8,
    VM::nib_9,
    VM::nib_a,
    VM::nib_b,
    VM::nib_c,
    VM::nib_d,
    VM::nib_e,
    VM::nib_f,
];

pub struct VM {
    display: display::Display,
    keypad: keypad::Keypad,
    mem: [u8; MEM_SIZE],
    regs: [u8; NUM_REGS],
    pc: u16,
    index: u16,
    sound_timer: u8,
    delay_timer: u8,
    stack: stack::Stack,
    opcode: u16,
    rng: rng::Rng,
    beeper: Option<rodio::Sink>,
    should_draw: bool,
    legacy_mode: bool,
    _debug_mode: bool, // Unused in release builds
}

impl VM {
    pub fn new(
        rom_reader: io::BufReader<std::fs::File>,
        legacy_mode: bool,
        debug_mode: bool,
    ) -> Self {
        let mut machine = VM {
            display: display::Display::new(),
            keypad: keypad::Keypad::new(),
            mem: [0; MEM_SIZE],
            regs: [0; NUM_REGS],
            index: 0,
            pc: ROM_START_ADDR as u16, // First 512 bytes reserved for internal use
            sound_timer: 0,
            delay_timer: 0,
            stack: stack::Stack::new(),
            opcode: 0,
            rng: rng::Rng::new(),
            beeper: None, // Set this up in run()
            should_draw: false,
            legacy_mode,
            _debug_mode: debug_mode,
        };

        // Init fonts
        machine.mem[..FONTS.len()].copy_from_slice(&FONTS);

        // Load ROM
        for (i, byte) in rom_reader.bytes().enumerate() {
            machine.mem[ROM_START_ADDR + i] = byte.unwrap();
        }

        machine
    }

    pub fn run(mut self) {
        // Acquire stdout lock continuously for slight performance gain
        let _handle = io::stdout().lock();

        // Used to time each frame to get ~60Hz runtime
        const FRAME_LENGTH: time::Duration = time::Duration::new(0, 1_000_000_000 / 60);

        // Do audio setup here to avoid lifetimes
        let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
        let sink = rodio::Sink::try_new(&stream_handle).unwrap();

        // Tune to the "correct" A4 ;)
        let source = rodio::source::SineWave::new(442.);
        sink.append(source);
        sink.pause();
        self.beeper = Some(sink);

        while !self.keypad.got_sigint() {
            let start_time = time::Instant::now();

            // Read next key input, and decrement key down timers
            self.keypad.cycle();

            // One instruction per frame is sluggish
            for _ in 0..INSTR_PER_FRAME {
                self.print_state(); // No-op in release builds
                self.exec_instr();
            }

            self.decrement_timers();

            if self.should_draw {
                self.should_draw = false;
                self.display.draw();
            }

            // Wait for end of frame to enforce 60Hz refresh rate
            let end_time = time::Instant::now();
            let wait_time = FRAME_LENGTH
                .checked_sub(end_time.saturating_duration_since(start_time))
                .unwrap_or(time::Duration::new(0, 0));
            std::thread::sleep(wait_time);
        }
    }

    fn exec_instr(&mut self) {
        // Opcodes are 2 bytes long
        let pc = self.pc as usize;
        self.opcode = ((self.mem[pc] as u16) << 8) | (self.mem[pc + 1] as u16);
        let op_type = ((self.mem[pc] & 0xF0) >> 4) as usize;
        self.pc += 2;

        debug_assert!(op_type <= NUM_OPCODE_TYPES, "Unknown opcode!");
        OPCODE_FUNCS[op_type](self);
    }

    fn decrement_timers(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        if self.sound_timer > 0 {
            self.sound_timer -= 1;

            // Beep while sound timer is nonzero
            if self.sound_timer == 0 {
                self.beeper.as_ref().unwrap().pause();
            }
        }
    }

    // 00E0: clear display
    // 00EE: return from subroutine
    // (Not supporting machine code routines with 0NNN)
    fn nib_0(&mut self) {
        match self.opcode {
            0x00E0 => {
                self.display
                    .frame_buffer
                    .fill([display::OFF_PIXEL; DISPLAY_WIDTH]);
            }
            0x00EE => self.pc = self.stack.pop(),
            _ => panic!("{}: Unsupported opcode!", self.opcode),
        }
    }

    // 1NNN: goto address NNN
    fn nib_1(&mut self) {
        let addr = self.opcode & 0x0FFF;
        debug_assert!(
            (addr as usize) >= ROM_START_ADDR && (addr as usize) < MEM_SIZE,
            "Invalid address!",
        );
        self.pc = addr;
    }

    // 2NNN: call subroutine at address NNN
    fn nib_2(&mut self) {
        self.stack.push(self.pc);
        self.nib_1();
    }

    // 3XNN: if (Vx != NN) skip next instruction
    fn nib_3(&mut self) {
        let reg_num = ((self.opcode & 0x0F00) >> 8) as usize;
        let val = (self.opcode & 0x00FF) as u8;
        debug_assert!(reg_num < NUM_REGS, "Invalid register!");

        if self.regs[reg_num] == val {
            self.pc += 2;
        }
    }

    // 4XNN: if (Vx != NN) skip next instruction
    fn nib_4(&mut self) {
        let reg_num = ((self.opcode & 0x0F00) >> 8) as usize;
        let val = (self.opcode & 0x00FF) as u8;
        debug_assert!(reg_num < NUM_REGS, "Invalid register!");

        if self.regs[reg_num] != val {
            self.pc += 2;
        }
    }

    // 5XY0: if (Vx == Vy) skip next instruction
    fn nib_5(&mut self) {
        let reg_x = ((self.opcode & 0x0F00) >> 8) as usize;
        let reg_y = ((self.opcode & 0x00F0) >> 4) as usize;
        debug_assert!(reg_x < NUM_REGS, "Invalid register X!");
        debug_assert!(reg_y < NUM_REGS, "Invalid register Y!");

        if self.regs[reg_x] == self.regs[reg_y] {
            self.pc += 2;
        }
    }

    // 6XNN: Vx = NN
    fn nib_6(&mut self) {
        let reg_num = ((self.opcode & 0x0F00) >> 8) as usize;
        let val = (self.opcode & 0x00FF) as u8;
        debug_assert!(reg_num < NUM_REGS, "Invalid register!");
        self.regs[reg_num] = val;
    }

    // 7XNN: Vx += NN
    fn nib_7(&mut self) {
        let reg_num = ((self.opcode & 0x0F00) >> 8) as usize;
        let val = (self.opcode & 0x00FF) as u8;
        debug_assert!(reg_num < NUM_REGS, "Invalid register!");
        self.regs[reg_num] = self.regs[reg_num].wrapping_add(val);
    }

    // 8XY[0-7, E]: Set Vx via operation with Vy
    fn nib_8(&mut self) {
        let reg_x = ((self.opcode & 0x0F00) >> 8) as usize;
        let reg_y = ((self.opcode & 0x00F0) >> 4) as usize;
        let op_index = (self.opcode & 0x000F) as usize;
        debug_assert!(reg_x < NUM_REGS, "Invalid register X!");
        debug_assert!(reg_y < NUM_REGS, "Invalid register Y!");

        const OPS: [fn(&mut VM, usize, usize); 9] = [
            // 8XY0
            |vm, reg_x, reg_y| vm.regs[reg_x] = vm.regs[reg_y],
            // 8XY1
            |vm, reg_x, reg_y| {
                vm.regs[reg_x] |= vm.regs[reg_y];

                // Reset Vf flag
                vm.regs[0xF] = 0;
            },
            // 8XY2
            |vm, reg_x, reg_y| {
                vm.regs[reg_x] &= vm.regs[reg_y];

                // Reset Vf flag
                vm.regs[0xF] = 0;
            },
            // 8XY3
            |vm, reg_x, reg_y| {
                vm.regs[reg_x] ^= vm.regs[reg_y];

                // Reset Vf flag
                vm.regs[0xF] = 0;
            },
            // 8XY4
            |vm, reg_x, reg_y| {
                let x = vm.regs[reg_x];
                vm.regs[reg_x] = x.wrapping_add(vm.regs[reg_y]);

                // Set carry flag
                vm.regs[0xF] = ((x as u16) + (vm.regs[reg_y] as u16) > 0xff) as u8;
            },
            // 8XY5
            |vm, reg_x, reg_y| {
                let x = vm.regs[reg_x];
                vm.regs[reg_x] = x.wrapping_sub(vm.regs[reg_y]);

                // Set borrow flag
                vm.regs[0xF] = (x >= vm.regs[reg_y]) as u8;
            },
            // 8XY6
            |vm, reg_x, reg_y| {
                // In original spec, set Vx = Vy first
                if vm.legacy_mode {
                    vm.regs[reg_x] = vm.regs[reg_y];
                }

                // Store least-significant bit of Vx in Vf
                let least_sig_bit = vm.regs[reg_x] & 1;
                vm.regs[reg_x] >>= 1;
                vm.regs[0xF] = least_sig_bit;
            },
            // 8XY7
            |vm, reg_x, reg_y| {
                let x = vm.regs[reg_x];
                vm.regs[reg_x] = vm.regs[reg_y].wrapping_sub(x);

                // Set borrow flag
                vm.regs[0xF] = (vm.regs[reg_y] >= x) as u8;
            },
            // 8XYE
            |vm, reg_x, reg_y| {
                // In original spec, set Vx = Vy first
                if vm.legacy_mode {
                    vm.regs[reg_x] = vm.regs[reg_y];
                }

                // Store most-significant bit of Vx in Vf
                let most_sig_bit = vm.regs[reg_x] >> (REG_WIDTH - 1);
                vm.regs[reg_x] <<= 1;
                vm.regs[0xF] = most_sig_bit;
            },
        ];

        if op_index < (OPS.len() - 1) {
            OPS[op_index](self, reg_x, reg_y);
        } else {
            // 8XYE is the odd one out, can't just index OPS
            debug_assert!(op_index == 0xE, "Invalid opcode!");
            OPS.last().unwrap()(self, reg_x, reg_y);
        }
    }

    // 9XY0: if (Vx != Vy) skip next instruction
    fn nib_9(&mut self) {
        let reg_x = ((self.opcode & 0x0F00) >> 8) as usize;
        let reg_y = ((self.opcode & 0x00F0) >> 4) as usize;
        debug_assert!(reg_x < NUM_REGS, "Invalid register X!");
        debug_assert!(reg_y < NUM_REGS, "Invalid register Y!");

        if self.regs[reg_x] != self.regs[reg_y] {
            self.pc += 2;
        }
    }

    // ANNN: I = NNN
    fn nib_a(&mut self) {
        self.index = self.opcode & 0x0FFF;
    }

    // BNNN: PC = V0 + NNN
    fn nib_b(&mut self) {
        let target = (self.opcode & 0x0FFF) + (self.regs[0] as u16);
        debug_assert!(
            (target as usize) >= ROM_START_ADDR && (target as usize) < MEM_SIZE,
            "Invalid address!",
        );
        self.pc = target;
    }

    // CXNN: Vx = rand[0, 255] & NN
    fn nib_c(&mut self) {
        let reg_num = ((self.opcode & 0x0F00) >> 8) as usize;
        let val = (self.opcode & 0x00FF) as u8;
        let rand_val = self.rng.get_byte();
        debug_assert!(reg_num < NUM_REGS, "Invalid register!");

        self.regs[reg_num] = rand_val & val;
    }

    // DXYN: Display sprite at [I] starting at (Vx, Vy)
    fn nib_d(&mut self) {
        let reg_x = ((self.opcode & 0x0F00) >> 8) as usize;
        let reg_y = ((self.opcode & 0x00F0) >> 4) as usize;
        let num_rows = (self.opcode & 0x000F) as usize;
        debug_assert!(reg_x < NUM_REGS, "Invalid register X!");
        debug_assert!(reg_y < NUM_REGS, "Invalid register Y!");

        let x_coord = (self.regs[reg_x] as usize) % DISPLAY_WIDTH;
        let y_coord = (self.regs[reg_y] as usize) % DISPLAY_HEIGHT;
        let sprite_addr = self.index as usize;

        // Vf = 1 if redraw turns off at least one pixel; init to 0
        self.regs[0xF] = 0;

        // DISPLAY_HEIGHT.min(y_coord + num_rows): don't write beyond bottom edge of display
        for (row_offset, row) in (y_coord..DISPLAY_HEIGHT.min(y_coord + num_rows)).enumerate() {
            let mut sprite_row = self.mem[sprite_addr + row_offset];

            // don't write beyond right edge of display
            for col in x_coord..DISPLAY_WIDTH.min(x_coord + SPRITE_WIDTH) {
                let pixel_state = sprite_row >> (SPRITE_WIDTH - 1);
                sprite_row <<= 1;

                if pixel_state == 1 {
                    if self.display.frame_buffer[row][col] == display::ON_PIXEL {
                        self.display.frame_buffer[row][col] = display::OFF_PIXEL;
                        // Vf = 1 if redraw turns off a pixel
                        self.regs[0xF] = 1;
                    } else {
                        self.display.frame_buffer[row][col] = display::ON_PIXEL;
                    }
                }
            }
        }

        self.should_draw = true;
    }

    // EX9E: if (key() == Vx) skip next instruction
    // EXA1: if (key() != Vx) skip next instruction
    fn nib_e(&mut self) {
        let reg_num = ((self.opcode & 0x0F00) >> 8) as usize;
        let op_type = self.opcode & 0x00FF;

        debug_assert!(reg_num < NUM_REGS, "Invalid register!");
        let reg_val = self.regs[reg_num];
        let is_pressed = self.keypad.is_key_pressed(reg_val as usize);

        match op_type {
            0x9E => self.pc += (is_pressed as u16) * 2,
            0xA1 => self.pc += (!is_pressed as u16) * 2,
            _ => panic!("{}: Unsupported opcode!", self.opcode),
        }
    }

    // FX__: Misc instructions
    fn nib_f(&mut self) {
        let reg_num = ((self.opcode & 0x0F00) >> 8) as usize;
        let op_type = self.opcode & 0x00FF;
        debug_assert!(reg_num < NUM_REGS, "Invalid register!");

        match op_type {
            // FX07: Vx = delay timer
            0x07 => self.regs[reg_num] = self.delay_timer,
            // FX0A: Vx = next key down; blocks for user input
            0x0A => {
                // Keep looping instruction until key is pressed and released
                match self.keypad.get_input() {
                    Some(key) => self.regs[reg_num] = key,
                    None => self.pc -= 2,
                }
            }
            // FX15: delay timer = Vx
            0x15 => self.delay_timer = self.regs[reg_num],
            // FX18: sound timer = Vx; beep while sound timer > 0
            0x18 => {
                self.sound_timer = self.regs[reg_num];
                if self.sound_timer > 0 {
                    self.beeper.as_ref().unwrap().play();
                }
            }
            // FX1E: I += Vx
            0x1E => self.index += self.regs[reg_num] as u16,
            // FX29: I = address of sprite in Vx
            0x29 => {
                // Sprite address = sprite number * 5 (5 bytes per sprite)
                debug_assert!(self.regs[reg_num] <= 0xF, "Invalid sprite!");
                self.index = (self.regs[reg_num] * 5) as u16;
            }
            // FX33: mem[I..I+3] = binary-encoded decimal form of Vx
            0x33 => {
                let reg_val = self.regs[reg_num];
                let index = self.index as usize;
                debug_assert!(
                    (index + 2) < MEM_SIZE,
                    "Index register reading out-of-bounds memory!",
                );
                self.mem[index] = reg_val / 100; // Hundredths place
                self.mem[index + 1] = (reg_val / 10) % 10; // Tenths place
                self.mem[index + 2] = reg_val % 10; // Ones place
            }
            // FX55: store regs V0 to Vx in memory
            0x55 => {
                let index = self.index as usize;
                debug_assert!(
                    (index + reg_num) < MEM_SIZE,
                    "Index register reading out-of-bounds memory!",
                );
                self.mem[index..index + reg_num + 1].copy_from_slice(&self.regs[..reg_num + 1]);

                // Index is incremented only in original CHIP-8 spec
                if self.legacy_mode {
                    self.index += (reg_num as u16) + 1;
                }
            }
            // FX65: load regs V0 to Vx from memory
            0x65 => {
                let index = self.index as usize;
                debug_assert!(
                    (index + reg_num) < MEM_SIZE,
                    "Index register reading out-of-bounds memory!",
                );
                self.regs[..reg_num + 1].copy_from_slice(&self.mem[index..index + reg_num + 1]);

                // Index is incremented only in original CHIP-8 spec
                if self.legacy_mode {
                    self.index += (reg_num as u16) + 1;
                }
            }
            _ => panic!("{}: Unsupported opcode!", self.opcode),
        }
    }

    #[cfg(debug_assertions)]
    fn print_state(&mut self) {
        if self._debug_mode {
            use std::io::Write;
            use termion::event::Key;

            // This will draw an empty frame if nothing has been written
            // to the frame buffer yet, ensuring the debug output doesn't
            // jump down several lines due to later frame renders
            self.display.draw();

            // Share stdout handle into alternate screen
            let output = self.display.borrow_output_buf();

            write!(
                output,
                "Next opcode: 0x{:X}, PC: 0x{:X}, Index register: 0x{:X}\r\n",
                self.opcode, self.pc, self.index
            )
            .unwrap();

            if (self.index as usize) < FONTS.len() && self.index % 5 == 0 {
                write!(
                    output,
                    "(Index register pointing to sprite {:X})\r\n",
                    self.index / 5
                )
                .unwrap();
            }

            write!(
                output,
                "Delay timer: 0x{:X}, Sound timer: 0x{:X}\r\n\n",
                self.delay_timer, self.sound_timer
            )
            .unwrap();

            write!(output, "Registers: {:X?}\r\n", self.regs).unwrap();
            write!(output, "Stack: {:X?}\r\n", self.stack).unwrap();

            if (self.pc as usize) < MEM_SIZE {
                let upper_bound = MEM_SIZE.min((self.pc + 16) as usize);
                write!(
                    output,
                    "Memory snippet [PC, 0x{:X}): {:X?}\r\n\n",
                    upper_bound,
                    &self.mem[(self.pc as usize)..upper_bound]
                )
                .unwrap();
            } else {
                write!(output, "PC out of memory bounds\r\n\n").unwrap();
            }

            write!(output, "Press 's' to step or c' to continue\r\n").unwrap();
            output.flush().unwrap();

            loop {
                if let Some(Ok(key)) = self.keypad.read_stdin() {
                    match key {
                        Key::Char('s') => break,
                        Key::Char('c') => {
                            self._debug_mode = false;
                            break;
                        }
                        _ => {}
                    }
                }
            }

            // Clear debug output
            write!(output, "{}", termion::clear::All).unwrap();
            output.flush().unwrap();
        }
    }

    // No-op in release builds
    #[cfg(not(debug_assertions))]
    fn print_state(&self) {}
}
