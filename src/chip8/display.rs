use std::io::{stdout, BufWriter, Stdout, Write};

use termion::raw::{IntoRawMode, RawTerminal};
use termion::screen::{AlternateScreen, IntoAlternateScreen};

const DISPLAY_HEIGHT: usize = 32;
const DISPLAY_WIDTH: usize = 64;
pub const OFF_PIXEL: char = ' ';
pub const ON_PIXEL: char = '█'; // U+2588 FULL BLOCK

pub struct Display {
    pub frame_buffer: [[char; DISPLAY_WIDTH]; DISPLAY_HEIGHT],
    // Lots going on here:
    // - Use buffer when writing to stdout to avoid unnecessary syscalls
    //   (BufWriter defaults to 8 KB buffer at time of writing, which is
    //    big enough)
    // - Terminal needs to be in raw mode to get user input correctly
    //   (i.e. user doesn't need to press enter to send input to stdin)
    // - Use termion's AlternateScreen to separate emulator output
    //   from rest of terminal history
    output: BufWriter<RawTerminal<AlternateScreen<Stdout>>>,
}

impl Display {
    pub fn new() -> Self {
        Display {
            frame_buffer: [[OFF_PIXEL; DISPLAY_WIDTH]; DISPLAY_HEIGHT],
            output: BufWriter::new(
                stdout()
                    .into_alternate_screen()
                    .unwrap()
                    .into_raw_mode()
                    .unwrap(),
            ),
        }
    }

    pub fn draw(&mut self) {
        // Reset cursor
        write!(self.output, "{}", termion::cursor::Goto(1, 1)).unwrap();

        self.draw_top_border();

        // Write frame
        for row in self.frame_buffer.iter() {
            write!(self.output, "{}{} ", ON_PIXEL, ON_PIXEL).unwrap();
            for pixel in row {
                write!(self.output, "{}", pixel).unwrap();
            }

            write!(self.output, " {}{}\r\n", ON_PIXEL, ON_PIXEL).unwrap();
        }

        self.draw_bottom_border();

        // One last carriage return
        write!(self.output, "\r\n\n").unwrap();

        // Flush the entire frame to stdout, with just one syscall
        self.output.flush().unwrap();
    }

    fn draw_top_border(&mut self) {
        // Draw top border
        for _ in 0..(DISPLAY_WIDTH + 6) {
            write!(self.output, "{}", ON_PIXEL).unwrap();
        }

        // Write extra padding below top border
        write!(self.output, "\r\n{}{} ", ON_PIXEL, ON_PIXEL).unwrap();
        for _ in 0..DISPLAY_WIDTH {
            write!(self.output, "{}", OFF_PIXEL).unwrap();
        }

        write!(self.output, " {}{}\r\n", ON_PIXEL, ON_PIXEL).unwrap();
    }

    fn draw_bottom_border(&mut self) {
        // Write extra padding above bottom border
        write!(self.output, "{}{} ", ON_PIXEL, ON_PIXEL).unwrap();
        for _ in 0..DISPLAY_WIDTH {
            write!(self.output, "{}", OFF_PIXEL).unwrap();
        }

        write!(self.output, " {}{}\r\n", ON_PIXEL, ON_PIXEL).unwrap();

        // Draw bottom border
        for _ in 0..(DISPLAY_WIDTH + 6) {
            write!(self.output, "{}", ON_PIXEL).unwrap();
        }
    }

    // Only let debug mode use the output buffer in debug builds
    #[cfg(debug_assertions)]
    pub fn borrow_output_buf(&mut self) -> &mut BufWriter<RawTerminal<AlternateScreen<Stdout>>> {
        &mut self.output
    }
}
