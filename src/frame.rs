use std::{
    cell::RefCell,
    fmt::Debug,
    iter::repeat,
    ops::{Range, RangeBounds},
    rc::Rc,
};

use anyhow::{anyhow, Context, Result};

use crate::{
    opcode::OpCode,
    value::{Closure, Function, Value},
};

pub struct Frame {
    pub pointer: usize,
    pub register_offset: usize,
    pub function: Rc<Function>,
    pub return_position: usize,
}

impl<'a> Debug for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "Frame (return_pos: {:?}, pointer: {:?}, depth: {:?}){{\nfunction: {:?}}}",
            self.return_position, self.pointer, self.register_offset, self.function
        ))
    }
}

impl Frame {
    pub fn register_range(&self) -> Range<usize> {
        self.register_offset..self.register_offset + self.function.registers
    }

    pub fn opcode(&self) -> Option<OpCode<u8, u8>> {
        self.function.opcodes.get(self.pointer).cloned()
    }
}
