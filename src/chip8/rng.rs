use std::time;

pub struct Rng {
    seed: u8,
}

impl Rng {
    pub fn new() -> Rng {
        Rng {
            seed: time::SystemTime::now()
                .duration_since(time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos() as u8,
        }
    }

    // Cheap PRNG from George Marsaglia paper "Xorshift RNGs" adapted for one-byte integers
    // (not looking for high-quality RNG here...)
    pub fn get_byte(self: &mut Rng) -> u8 {
        self.seed ^= self.seed << 7;
        self.seed ^= self.seed >> 5;
        self.seed ^= self.seed << 3;
        self.seed
    }
}
