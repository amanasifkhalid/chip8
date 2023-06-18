![](chip8-demo.mp4)

A lightweight CHIP-8 emulator that runs directly in your terminal -- no OpenGL required!

## Install
- From the project root, run `cargo install --path .`. You'll then be able to run the `chip8` binary directly.
- If you just want to try it without installing, run `cargo run --release -- <path to ROM>`.
  - For best results, you should use a release build, unless you want to use the debugger feature.
  - See below for optional command-line arguments.

## Features
- Due to ambiguity in the CHIP-8 specification, some older ROMs may not work out-of-the-box. Try running these in legacy mode
by running the binary with the `--legacy` flag.
- If your ROM isn't working, try stepping through it with debug mode, enabled with the `--debug` flag.
This allows you to step through each instruction and see the emulator's current state, allowing you to find
the bug in your ROM (or in my emulator...).
  - Debug mode is available only in debug builds to avoid cluttering the emulator loop with unnecessary branches.
  To use it, run the emulator with `cargo run -- <path to ROM> --debug`.
- This emulator uses just two dependencies: [termion](https://github.com/redox-os/termion) for I/O, and
[rodio](https://github.com/RustAudio/rodio) for audio output.

## Limitations
- Termion only supports ANSI-compliant terminals; minimalism was prioritized over portability here. Sorry, Windows users!
- This emulator uses the following keyboard mapping from the original COSMAC VIP layout to QWERTY keyboards,
which hasn't been tested on other layouts where it might be awkward:

```
COSMAC:        QWERTY:
1 2 3 C        1 2 3 4
4 5 6 D        Q W E R
7 8 9 E        A S D F
A 0 B F        Z X C V
```
- The CHIP-8 specification differentiates between key up and down actions, whereas ANSI terminals don't. Thus,
input-critical ROMs may run jankily.
