mod chip8;

use std::env;
use std::fs;
use std::io;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 || args.len() > 4 {
        eprintln!("USAGE: cargo run [--release] -- <ROM path> [--legacy] [--debug]");
        return;
    }

    // Many ROMs expect slightly different implementations for some opcodes.
    // Legacy mode enforces the original CHIP-8 specification, in lieu of modern interpretations.
    let legacy_mode = args.contains(&String::from("--legacy"));

    // Debug mode allows stepping through the ROM instruction-by-instruction,
    // displaying the emulator's current state (memory, registers, etc.).
    let debug_mode = args.contains(&String::from("--debug"));

    let rom_file = fs::File::open(&args[1]).expect("Cannot open ROM file!");
    let rom_reader = io::BufReader::new(rom_file);
    let vm = chip8::VM::new(rom_reader, legacy_mode, debug_mode);
    vm.run();
}
