const STACK_SIZE: usize = 12;

#[derive(Debug)]
pub struct Stack {
    mem: [u16; STACK_SIZE],
    ptr: usize,
}

impl Stack {
    pub fn new() -> Stack {
        Stack {
            mem: [0; STACK_SIZE],
            ptr: 0,
        }
    }

    pub fn push(self: &mut Stack, val: u16) {
        debug_assert!(self.ptr < STACK_SIZE, "Stack overflow!");
        self.mem[self.ptr] = val;
        self.ptr += 1;
    }

    pub fn pop(self: &mut Stack) -> u16 {
        debug_assert!(self.ptr != 0, "Stack underflow!");
        let val = self.mem[self.ptr - 1];
        self.ptr -= 1;
        val
    }
}
