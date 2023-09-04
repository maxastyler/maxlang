use std::{
    cell::RefCell,
    fmt::Debug,
    iter::repeat,
    ops::{Deref, RangeBounds},
    rc::Rc,
};

use crate::{
    frame::Frame,
    native_function::{self, NativeFunction},
    opcode::OpCode,
    value::{Closure, Function, Object, Value},
};

use anyhow::{anyhow, Context, Result};

#[derive(Default)]
pub struct Storage {
    register_storage: Vec<Value>,
    temporary_storage: Vec<Value>,
    /// The point where the register is filled up to
    register_fill_point: usize,
}

impl Storage {
    /// ensure that the storage has slots filled up to the given length, increasing capacity if not
    fn ensure_filled(&mut self, length: usize) {
        if self.register_storage.len() < length {
            self.register_storage
                .extend(repeat(Value::Uninit).take(length - self.register_storage.len()))
        }
    }
    fn splice<R, I>(&mut self, range: R, replace_with: I)
    where
        R: RangeBounds<usize>,
        I: IntoIterator<Item = Value>,
    {
        self.register_storage.splice(range, replace_with);
    }
}

#[derive(Default)]
pub struct VM {
    pub frames: Vec<Frame>,
    pub storage: Storage,
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
    pub fn create_new_frame(
        &mut self,
        closure: Rc<Closure>,
        starting_register: usize,
        return_position: usize,
    ) -> Frame {
        self.storage
            .ensure_filled(starting_register + closure.function.registers);
        for (i, c) in closure.captures.iter().enumerate() {
            self.storage.register_storage[starting_register + i] = c.clone();
        }

        Frame {
            pointer: 0,
            register_offset: starting_register,
            function: closure.function.clone(),
            return_position,
        }
    }

    pub fn from_bare_function(f: Function) -> Self {
        let closure = Rc::new(Closure {
            function: Rc::new(f),
            captures: vec![],
        });
        let mut vm = VM::default();
        let frame = vm.create_new_frame(closure, 0, 0);
        vm.frames.push(frame);
        vm
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
            OpCode::LoadConstant(constant_index, target) => {
                self.load_constant(constant_index.into(), target.into());
                Ok(None)
            }
            OpCode::CloseValue(i) => {
                self.close_value(i.into());
                Ok(None)
            }
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

    /// Get a mutable reference to a value in the last frame's registers
    fn value_in_last_frame(&mut self, position: usize) -> &mut Value {
        let pos = self.last_frame_mut().unwrap().register_offset + position;
        self.storage.register_storage.get_mut(pos).unwrap()
    }

    fn insert_native_function(&mut self, native_fn: NativeFunction, position: usize) -> Result<()> {
        *self.value_in_last_frame(position) = Value::NativeFunction(native_fn);
        self.increase_pointer(1);
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
        match self.value_in_last_frame(boolean_register) {
            Value::Bool(b) => {
                if *b {
                    {
                        self.increase_pointer(1);
                        Ok(())
                    }
                } else {
                    self.jump(position)
                }
            }
            _ => Err(anyhow!("The given register didn't contain a boolean")),
        }
    }

    fn tail_call(&mut self, function_register: usize) -> Result<Option<Value>> {
        match self.value_in_last_frame(function_register).clone() {
            Value::NativeFunction(native_function) => {
                let value = self.call_native_function(native_function)?;

                return if self.frames.len() <= 1 {
                    Ok(Some(value))
                } else {
                    let return_position = self.last_frame()?.return_position;
                    self.frames.pop().unwrap();
                    *self.value_in_last_frame(return_position) = value;
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
        self.increase_pointer(1);
        loop {
            match self.last_frame()?.opcode() {
                Some(OpCode::CaptureValue(i)) => closure
                    .captures
                    .push(self.value_in_last_frame(i as usize).clone()),
                _ => break,
            }
            self.increase_pointer(1);
        }
        *self.value_in_last_frame(result_index) = Value::Object(Object::Closure(Rc::new(closure)));
        Ok(())
    }

    fn transform_last_frame_for_tail_call(&mut self, closure: Rc<Closure>) {
        loop {
            match self.last_frame().unwrap().opcode() {
                Some(OpCode::CallArgument(argument_register)) => {
                    let value: Value = self.value_in_last_frame(argument_register as usize).clone();
                    self.storage.temporary_storage.push(value);
                    self.increase_pointer(1);
                }
                _ => break,
            }
        }
        debug_assert!(self.storage.temporary_storage.len() == closure.function.arity);
        let frame = self.frames.last_mut().unwrap();
        let storage = &mut self.storage;
        storage.ensure_filled(frame.register_offset + closure.function.registers);
        for (i, c) in closure.captures.iter().enumerate() {
            storage.register_storage[frame.register_offset + i] = c.clone();
        }

        for (i, t) in storage.temporary_storage.iter().enumerate() {
            storage.register_storage[frame.register_offset + closure.captures.len() + i] =
                t.clone();
        }

        frame.function = closure.function.clone();
        frame.pointer = 0;

        self.storage.temporary_storage.clear()
    }

    /// Creates a new frame from a closure and arguments in the VM's temporary storage
    /// Pushes the new frame onto the current stack of frames
    fn create_and_push_new_frame(
        &mut self,
        closure: Rc<Closure>,
        result_slot: usize,
        tail_call: bool,
    ) -> Result<()> {
        self.increase_pointer(1);
        if tail_call {
            self.transform_last_frame_for_tail_call(closure);
            Ok(())
        } else {
            let capture_offset = closure.function.capture_offset;
            let mut index = capture_offset;
            let register_offset =
                self.last_frame()?.register_offset + self.last_frame()?.function.registers;
            let mut new_frame = self.create_new_frame(closure, register_offset, result_slot);
            loop {
                match self.last_frame()?.opcode() {
                    Some(OpCode::CallArgument(argument_register)) => {
                        let value: Value =
                            self.value_in_last_frame(argument_register as usize).clone();
                        self.storage.register_storage[index + register_offset] = value;
                        self.increase_pointer(1);
                        index += 1;
                    }
                    _ => break,
                }
            }

            if index != new_frame.function.arity + capture_offset {
                Err(anyhow!("Called function with wrong number of arguments"))
            } else {
                self.frames.push(new_frame);
                Ok(())
            }
        }
    }

    fn call_native_function(&mut self, function: NativeFunction) -> Result<Value> {
        self.increase_pointer(1);
        loop {
            match self.last_frame()?.opcode() {
                Some(OpCode::CallArgument(argument_register)) => {
                    let value: Value = self.value_in_last_frame(argument_register as usize).clone();

                    self.storage.temporary_storage.push(value);
                    self.increase_pointer(1);
                }
                _ => break,
            }
        }
        let result = function.call(&self.storage.temporary_storage);
        self.storage.temporary_storage.clear();
        result
    }

    fn call(&mut self, function_register: usize, result_slot: usize) -> Result<()> {
        match self.value_in_last_frame(function_register).clone() {
            Value::NativeFunction(nf) => {
                let value = self.call_native_function(nf)?;
                *self.value_in_last_frame(result_slot) = value;
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

    fn pop_frame(&mut self) {
        let f = self.last_frame().unwrap();
        let range = f.register_range();
        self.storage.register_storage[range].fill(Value::Uninit);
        self.frames.pop();
    }

    fn vm_return(&mut self, return_value_position: usize) -> Result<Option<Value>> {
        let value = self.value_in_last_frame(return_value_position).clone();
        let return_position = self.last_frame()?.return_position;
        self.pop_frame();
        if self.frames.is_empty() {
            Ok(Some(value))
        } else {
            *self.value_in_last_frame(return_position) = value;
            Ok(None)
        }
    }

    fn close_value(&mut self, register_position: usize) {
        *self.value_in_last_frame(register_position) = Value::Uninit;
        self.increase_pointer(1);
    }

    fn copy_value(&mut self, from: usize, target: usize) -> Result<()> {
        let f = self.last_frame_mut()?;
        *self.value_in_last_frame(target) = self.value_in_last_frame(from).clone();
        self.increase_pointer(1);
        Ok(())
    }

    fn load_constant(&mut self, constant_index: usize, target: usize) {
        *self.value_in_last_frame(target) =
            self.last_frame().unwrap().function.constants[constant_index].clone();
        self.increase_pointer(1);
    }

    fn increase_pointer(&mut self, amount: usize) {
        self.last_frame_mut().unwrap().pointer += amount;
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
