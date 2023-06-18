use std::io;
use std::io::Write;

use termion::raw::IntoRawMode;

const DISPLAY_HEIGHT: usize = 32;
const DISPLAY_WIDTH: usize = 64;
pub const OFF_PIXEL: char = ' ';
pub const ON_PIXEL: char = '█'; // U+2588 FULL BLOCK

pub struct Display {
    pub frame_buffer: [[char; DISPLAY_WIDTH]; DISPLAY_HEIGHT],
    output: io::BufWriter<termion::raw::RawTerminal<io::Stdout>>,
}

impl Display {
    pub fn new() -> Display {
        Display {
            frame_buffer: [[OFF_PIXEL; DISPLAY_WIDTH]; DISPLAY_HEIGHT],
            // Use buffer when writing to stdout to avoid unnecessary syscalls
            // (BufWriter uses a default size of 8 KB at the time of writing,
            // which is big enough to avoid flushing early for our purposes)
            output: io::BufWriter::new(io::stdout().into_raw_mode().unwrap()),
        }
    }

    pub fn draw(&mut self) {
        // Reset cursor to space within border
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
}
