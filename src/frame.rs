use std::{
    cell::RefCell,
    fmt::Debug,
    ops::{Range, RangeBounds},
    rc::Rc,
};

use crate::{
    native_function::NativeFunction,
    opcode::{FunctionIndex, OpCode, RegisterIndex, ValueIndex},
    value::{Closure, Function, Placeholder, Value},
    vm::{CallType, RuntimeError},
};

pub struct Frame {
    pub pointer: usize,
    pub inside_call: Option<(ValueIndex, Option<RegisterIndex>)>,
    pub registers: Vec<Placeholder>,
    pub function: Rc<Function>,
    pub captures: Vec<Value>,
    pub return_position: usize,
}

type Result<T> = std::result::Result<T, RuntimeError>;

impl<'a> Debug for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "Frame (return_pos: {:?}, pointer: {:?}){{\nfunction: {:?}}}",
            self.return_position, self.pointer, self.function
        ))
    }
}

impl Frame {
    pub fn new_from_closure(closure: Rc<Closure>, return_position: usize) -> Frame {
        match closure.function.clone() {
            crate::value::ClosureType::Function(func) => {
                let mut registers = vec![Placeholder::Value(Value::Uninit); func.num_registers];
                registers.splice(
                    0..func.arity,
                    closure
                        .arguments
                        .iter()
                        .map(|x| Placeholder::Value(x.clone())),
                );
                Frame {
                    pointer: 0,
                    inside_call: None,
                    registers,
                    function: func,
                    captures: closure
                        .captures
                        .iter()
                        .map(|x| match x {
                            Placeholder::Placeholder(_) => unreachable!(),
                            Placeholder::Value(v) => v.clone(),
                        })
                        .collect(),
                    return_position,
                }
            }
            crate::value::ClosureType::NativeFunction(_) => unreachable!(),
        }
    }

    pub fn register_range(&self) -> Range<usize> {
        self.function.arity..(self.function.arity + self.function.num_registers)
    }

    pub fn opcode(&self) -> Option<OpCode> {
        self.function.opcodes.get(self.pointer).cloned()
    }

    pub fn get_value_index(&self, value_index: ValueIndex) -> Placeholder {
        match value_index {
            ValueIndex::Register(reg) => self.registers[reg.0 as usize].clone(),
            ValueIndex::Constant(con) => {
                Placeholder::Value(self.function.constants[con.0 as usize].clone())
            }
            ValueIndex::Capture(cap) => Placeholder::Value(self.captures[cap.0 as usize].clone()),
        }
    }

    pub fn run_declare_recursive(&mut self, index: RegisterIndex) {
        self.registers[index.0 as usize] =
            Placeholder::Placeholder(Rc::new(RefCell::new(Value::Uninit)));
        self.pointer += 1;
    }

    pub fn run_fill_recursive(&mut self, value_index: ValueIndex, register_index: RegisterIndex) {
        self.registers[register_index.0 as usize] = self.get_value_index(value_index);
        self.pointer += 1;
    }

    pub fn run_jump(&mut self, offset: isize) {
        self.pointer.saturating_add_signed(offset);
    }

    pub fn run_jump_to_position_if_false(
        &mut self,
        check_index: ValueIndex,
        offset: isize,
    ) -> Result<()> {
        let b = match self.get_value_index(check_index).unwrap() {
            Value::Bool(b) => b,
            _ => return Err(RuntimeError::NotABoolean),
        };
        if b {
            self.run_jump(offset);
        }
        Ok(())
    }

    pub fn run_copy_value(&mut self, from_index: ValueIndex, to_index: RegisterIndex) {
        self.registers[to_index.0 as usize] = self.get_value_index(from_index);
        self.pointer += 1;
    }

    pub fn run_close_value(&mut self, index: RegisterIndex) {
        self.registers[index.0 as usize] = Placeholder::Value(Value::Uninit);
        self.pointer += 1;
    }

    pub fn run_insert_native_function(
        &mut self,
        native_function: NativeFunction,
        index: RegisterIndex,
    ) {
        self.registers[index.0 as usize] =
            Placeholder::Value(Value::NativeFunction(native_function));
        self.pointer += 1;
    }

    pub fn run_call(&mut self, call_type: CallType, function_index: ValueIndex) {
        match call_type {
            CallType::Tail => {
                self.inside_call = Some((function_index, None));
            }
            CallType::NonTail(result_index) => {
                self.inside_call = Some((function_index, Some(result_index)));
            }
        };
    }
}
