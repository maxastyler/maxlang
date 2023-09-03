use std::{cell::RefCell, fmt::Debug, rc::Rc};

use anyhow::{Context, Result};

use crate::{
    opcode::OpCode,
    value::{Closure, Function, Value},
};

pub struct Frame {
    pub depth: usize,
    pub pointer: usize,
    pub registers: Vec<Value>,
    pub function: Rc<Function>,
    pub return_position: usize,
}

impl Debug for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "Frame (return_pos: {:?}, pointer: {:?}, depth: {:?}){{\nregisters: {:?}\nfunction: {:?}}}",
	    self.return_position,
            self.pointer,
            self.depth,
            self.registers
                .iter()
                .enumerate()
                .filter_map(|(i, x)| if matches!(x, Value::Nil) {
                    None
                } else {
                    Some(format!("({:?}: {:?})", i, x))
                })
                .collect::<Vec<_>>()
                .join(" ||| "),
	    self.function
        ))
    }
}

impl Frame {
    pub fn new(closure: Rc<Closure>, depth: usize, return_position: usize) -> Frame {
        let mut f = Frame {
            depth,
            pointer: 0,
            registers: vec![Value::Nil; closure.function.registers],
            function: closure.function.clone(),
            return_position,
        };
        f.registers
            .splice(0..closure.function.capture_offset, closure.captures.clone());
        f
    }

    pub fn opcode(&self) -> Option<OpCode<u16, u16>> {
        self.function.opcodes.get(self.pointer).cloned()
    }
}
