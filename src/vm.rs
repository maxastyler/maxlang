use std::{cell::RefCell, fmt::Debug, ops::Deref, rc::Rc};

use crate::{
    frame::Frame,
    native_function::{self, NativeFunction},
    opcode::OpCode,
    value::{Closure, Function, Object, Value},
};

use anyhow::{anyhow, Context, Result};

#[derive(Default)]
pub struct VM {
    pub frames: Vec<Frame>,
}

impl Debug for VM {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("VM {{\nframes: [\n")?;
        for fr in self.frames.iter() {
            f.write_str(&format!("{:?}\n", fr))?;
        }
        Ok(())
    }
}

impl VM {
    pub fn from_function(f: Function) -> Self {
        Self {
            frames: vec![Frame::new(
                Rc::new(Closure {
                    function: Rc::new(f),
                    captures: vec![],
                }),
                0,
                0,
            )],
        }
    }

    pub fn step(&mut self) -> Result<Option<Value>> {
        let oc = self
            .last_frame()?
            .opcode()
            .context(format!("Could not get value for opcode"))?;
        match oc {
            OpCode::Call(r, i) => self.call(r.into(), i.into()).map(|_| None),
            OpCode::Return(i) => self.vm_return(i.into()),
            OpCode::CopyValue(from, target) => {
                self.copy_value(from.into(), target.into()).map(|_| None)
            }
            OpCode::LoadConstant(constant_index, target) => self
                .load_constant(constant_index.into(), target.into())
                .map(|_| None),
            OpCode::CloseValue(i) => self.close_value(i.into()).map(|_| None),
            OpCode::TailCall(r) => self.tail_call(r.into()),
            OpCode::CreateClosure(function_index, return_register) => self
                .create_closure(function_index.into(), return_register.into())
                .map(|_| None),
            OpCode::CallArgument(_) => Err(anyhow!(
                "Bytecode incorrect. Tried to have a call argument without a previous call"
            )),
            OpCode::Jump(position) => self.jump(position.into()).map(|_| None),
            OpCode::JumpToPositionIfFalse(boolean_register, position) => self
                .jump_to_position_if_false(boolean_register.into(), position.into())
                .map(|_| None),
            OpCode::Crash => Err(anyhow!("CRASH")),
            OpCode::InsertNativeFunction(native_fn, position) => self
                .insert_native_function(native_fn, position.into())
                .map(|_| None),
            OpCode::CaptureValue(_) => Err(anyhow!(
                "Bytecode incorrect. Tried to capture a value without a previous closure"
            )),
        }
    }

    fn insert_native_function(&mut self, native_fn: NativeFunction, position: usize) -> Result<()> {
        self.last_frame_mut()?.registers[position] = Value::NativeFunction(native_fn);
        self.increase_pointer(1)?;
        Ok(())
    }

    fn jump(&mut self, position: usize) -> Result<()> {
        self.last_frame_mut()?.pointer = position;
        Ok(())
    }

    fn jump_to_position_if_false(
        &mut self,
        boolean_register: usize,
        position: usize,
    ) -> Result<()> {
        match self.last_frame()?.registers[boolean_register] {
            Value::Bool(b) => {
                if b {
                    self.increase_pointer(1)
                } else {
                    self.jump(position)
                }
            }
            _ => Err(anyhow!("The given register didn't contain a boolean")),
        }
    }

    fn tail_call(&mut self, function_register: usize) -> Result<Option<Value>> {
        match self.last_frame()?.registers[function_register].clone() {
            Value::NativeFunction(native_function) => {
                let value = self.call_native_function(native_function)?;

                return if self.frames.len() <= 1 {
                    Ok(Some(value))
                } else {
                    let return_position = self.last_frame()?.return_position;
                    self.frames.pop().unwrap();
                    self.last_frame_mut()?.registers[return_position] = value;
                    Ok(None)
                };
            }
            Value::Object(Object::Closure(closure)) => {
                self.create_and_push_new_frame(
                    closure.clone(),
                    self.last_frame()?.return_position,
                    true,
                )?;
                Ok(None)
            }
            x => Err(anyhow!("Not a function: {:?}", x)),
        }
    }

    fn create_closure(&mut self, function_index: usize, result_index: usize) -> Result<()> {
        let function = self.last_frame()?.function.functions[function_index].clone();
        let mut closure = Closure {
            function: function.clone(),
            captures: Vec::with_capacity(function.capture_offset),
        };
        self.increase_pointer(1)?;
        loop {
            match self.last_frame()?.opcode() {
                Some(OpCode::CaptureValue(i)) => closure
                    .captures
                    .push(self.last_frame()?.registers[i as usize].clone()),
                _ => break,
            }
            self.increase_pointer(1)?;
        }
        self.last_frame_mut()?.registers[result_index] =
            Value::Object(Object::Closure(Rc::new(closure)));
        Ok(())
    }

    /// Creates a new frame from a closure and arguments in the VM's temporary storage
    /// Pushes the new frame onto the current stack of frames
    fn create_and_push_new_frame(
        &mut self,
        closure: Rc<Closure>,
        result_slot: usize,
        drop_last: bool,
    ) -> Result<()> {
        self.increase_pointer(1)?;
        let offset = closure.function.capture_offset;
        let mut index = offset;
        let mut new_frame = Frame::new(closure, self.last_frame()?.depth + 1, result_slot);
        loop {
            match self.last_frame()?.opcode() {
                Some(OpCode::CallArgument(argument_register)) => {
                    let value: Value =
                        self.last_frame()?.registers[argument_register as usize].clone();
                    new_frame.registers[index] = value;
                    self.increase_pointer(1)?;
                    index += 1;
                }
                _ => break,
            }
        }

        if index != new_frame.function.arity + offset {
            Err(anyhow!("Called function with wrong number of arguments"))
        } else {
            if drop_last {
                self.frames.pop();
            }
            self.frames.push(new_frame);
            Ok(())
        }
    }

    fn call_native_function(&mut self, function: NativeFunction) -> Result<Value> {
        self.increase_pointer(1)?;
        let mut arguments: Vec<Value> = vec![];
        loop {
            match self.last_frame()?.opcode() {
                Some(OpCode::CallArgument(argument_register)) => {
                    let value: Value =
                        self.last_frame()?.registers[argument_register as usize].clone();
                    arguments.push(value);
                    self.increase_pointer(1)?;
                }
                _ => break,
            }
        }
        function.call(&arguments)
    }

    fn call(&mut self, function_register: usize, result_slot: usize) -> Result<()> {
        match self.last_frame()?.registers[function_register].clone() {
            Value::NativeFunction(nf) => {
                let value = self.call_native_function(nf)?;
                self.last_frame_mut()?.registers[result_slot] = value;
            }
            Value::Object(Object::Closure(closure)) => {
                self.create_and_push_new_frame(closure, result_slot, false)?
            }
            x => {
                return Err(anyhow!(
                    "Tried to call something that's not a function: {:?}",
                    x
                ))
            }
        };
        Ok(())
    }

    fn vm_return(&mut self, return_value_position: usize) -> Result<Option<Value>> {
        let value = self
            .last_frame_mut()?
            .registers
            .swap_remove(return_value_position);
        let return_position = self.last_frame()?.return_position;
        self.frames.pop();
        if self.frames.is_empty() {
            Ok(Some(value))
        } else {
            self.last_frame_mut()?.registers[return_position] = value;
            Ok(None)
        }
    }

    fn close_value(&mut self, register_position: usize) -> Result<()> {
        self.last_frame_mut()?
            .registers
            .splice(register_position..(register_position + 1), [Value::Nil])
            .next()
            .context("Could not close value")?;
        self.increase_pointer(1)
    }

    fn copy_value(&mut self, from: usize, target: usize) -> Result<()> {
        let f = self.last_frame_mut()?;
        f.registers[target] = f.registers[from].clone();
        self.increase_pointer(1)?;
        Ok(())
    }

    fn load_constant(&mut self, constant_index: usize, target: usize) -> Result<()> {
        let f = self.last_frame_mut()?;
        f.registers[target] = f.function.constants[constant_index].clone();
        self.increase_pointer(1)
    }

    fn increase_pointer(&mut self, amount: usize) -> Result<()> {
        self.last_frame_mut()?.pointer += amount;
        Ok(())
    }

    fn last_frame_mut(&mut self) -> Result<&mut Frame> {
        self.frames
            .last_mut()
            .context("Could not get last frame as mutable reference")
    }

    fn last_frame(&self) -> Result<&Frame> {
        self.frames.last().context("Could not get last frame")
    }
}
