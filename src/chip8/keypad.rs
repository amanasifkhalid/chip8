use termion::event::Key;
use termion::input::TermRead;

const NUM_KEYS: usize = 16;
const KEY_TIMEOUT: u8 = 16; // Length of each key press in frames

// Mapping of COSMAC VIP keyboard (0-F keys, represented by index) to QWERTY layout
const KEYS: [Key; NUM_KEYS] = [
    Key::Char('x'),
    Key::Char('1'),
    Key::Char('2'),
    Key::Char('3'),
    Key::Char('q'),
    Key::Char('w'),
    Key::Char('e'),
    Key::Char('a'),
    Key::Char('s'),
    Key::Char('d'),
    Key::Char('z'),
    Key::Char('c'),
    Key::Char('4'),
    Key::Char('r'),
    Key::Char('f'),
    Key::Char('v'),
];

pub struct Keypad {
    input: termion::input::Keys<termion::AsyncReader>,
    key_states: [u8; NUM_KEYS],
    queued_key: Option<u8>,
    waiting_for_input: bool,
    got_sigint: bool,
}

impl Keypad {
    pub fn new() -> Keypad {
        Keypad {
            input: termion::async_stdin().keys(),
            key_states: [0; NUM_KEYS],
            queued_key: None,
            waiting_for_input: false,
            got_sigint: false,
        }
    }

    pub fn cycle(&mut self) {
        self.decrement_key_timers();
        let input = self.input.next();

        if let Some(Ok(next_key)) = input {
            match next_key {
                Key::Ctrl('c') => {
                    self.got_sigint = true;
                }
                _ => {
                    if let Some(key_ind) = KEYS.iter().position(|&valid_key| next_key == valid_key)
                    {
                        if self.key_states[key_ind] == 0 {
                            self.key_states[key_ind] = KEY_TIMEOUT;
                        }

                        self.waiting_for_input = false;
                    }
                }
            }
        }
    }

    pub fn get_input(&mut self) -> Option<u8> {
        // First, wait for a key to be pressed
        if self.queued_key.is_none() {
            for (key, timer) in self.key_states.iter().enumerate() {
                // Key was pressed this frame if its timer hasn't decremented yet
                if *timer == KEY_TIMEOUT {
                    self.queued_key = Some(key as u8);
                    break;
                }
            }

            // If key hasn't been pressed, keep waiting
            // If key is pressed, now wait for it to be released
            return None;
        }

        // Don't stop blocking until queued key is released
        if self.key_states[self.queued_key.unwrap() as usize] > 0 {
            return None;
        }

        let released_key = self.queued_key;
        self.queued_key = None;
        released_key
    }

    pub fn is_key_pressed(&self, key_val: usize) -> bool {
        debug_assert!(key_val < NUM_KEYS, "Invalid keypad value!");
        self.key_states[key_val] != 0
    }

    pub fn got_sigint(&self) -> bool {
        self.got_sigint
    }

    // Used only in debug mode; not for normal ROM input
    #[cfg(debug_assertions)]
    pub fn read_stdin(&mut self) -> Option<Result<termion::event::Key, std::io::Error>> {
        self.input.next()
    }

    fn decrement_key_timers(&mut self) {
        for timer in self.key_states.iter_mut() {
            if *timer != 0 {
                *timer -= 1;
            }
        }
    }
}
