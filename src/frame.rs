use std::{cell::RefCell, fmt::Debug, iter::repeat, rc::Rc};

use anyhow::{anyhow, Context, Result};

use crate::{
    opcode::OpCode,
    value::{Closure, Function, Value},
};

pub struct Frame {
    pub pointer: usize,
    pub starting_register: usize,
    pub function: Rc<Function>,
    pub return_position: usize,
}

impl<'a> Debug for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "Frame (return_pos: {:?}, pointer: {:?}, depth: {:?}){{\nfunction: {:?}}}",
            self.return_position,
            self.pointer,
            // self.registers
            //     .iter()
            //     .enumerate()
            //     .filter_map(|(i, x)| if matches!(x, Value::Nil) {
            //         None
            //     } else {
            //         Some(format!("({:?}: {:?})", i, x))
            //     })
            //     .collect::<Vec<_>>()
            //     .join(" ||| "),
            self.starting_register,
            self.function
        ))
    }
}

impl Frame {
    pub fn new(
        closure: Rc<Closure>,
        starting_register: usize,
        registers: &mut [Value],
        return_position: usize,
    ) -> Frame {
        let mut f = Frame {
            pointer: 0,
            starting_register,
            function: closure.function.clone(),
            return_position,
        };
        for (i, c) in closure.captures.iter().enumerate() {
            registers[i] = c.clone();
        }
        f
    }

    pub fn transform_for_tail_call(&mut self, closure: Rc<Closure>) -> Result<()> {
        let mut new_arguments = Vec::with_capacity(closure.captures.len() + closure.function.arity);
        new_arguments.extend(closure.captures.clone());
        loop {
            match self.opcode() {
                Some(OpCode::CallArgument(argument_register)) => {
                    let value: Value = self.registers[argument_register as usize].clone();
                    new_arguments.push(value);
                    self.pointer += 1;
                }
                _ => break,
            }
        }
        if new_arguments.len() != closure.function.arity + closure.function.capture_offset {
            Err(anyhow!(
                "Called function with wrong number of arguments: arity is: {:?}",
                closure.function.arity
            ))
        } else {
            self.function = closure.function.clone();
            self.pointer = 0;
            self.registers.reserve(closure.function.registers);
            self.registers.extend(
                repeat(Value::Nil).take(
                    closure
                        .function
                        .registers
                        .saturating_sub(self.registers.len()),
                ),
            );
            self.registers.splice(0..new_arguments.len(), new_arguments);
            Ok(())
        }
    }

    pub fn opcode(&self) -> Option<OpCode<u8, u8>> {
        self.function.opcodes.get(self.pointer).cloned()
    }
}
