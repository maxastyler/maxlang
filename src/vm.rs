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
    opcode::{FunctionIndex, OpCode, RegisterIndex, ValueIndex},
    value::{Closure, Function, Object, Placeholder, Value, ClosureType},
};

pub enum RuntimeError {
    NoMoreOpCodes,
    NotAFunction,
    NotABoolean,
    NoLastFrame,
    ValueNotSet,
    TooManyArguments,
    NotEnoughArguments,
}

pub enum CallType {
    Tail,
    NonTail(RegisterIndex),
}

type Result<T> = std::result::Result<T, RuntimeError>;

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
    pub fn from_bare_function(f: Function) -> Self {
        let closure = Rc::new(Closure {
            function: Rc::new(f),
            captures: vec![],
            arguments: vec![],
        });
        let mut vm = VM::default();
        vm.frames.push(Frame::new_from_closure(closure, 0));
        vm
    }

    pub fn get_call_arguments(&mut self, limit: Option<usize>) -> Result<Vec<Value>> {
        let mut args = vec![];
        while let Some(OpCode::CallArgument(index)) = self.last_frame()?.opcode() {
            if limit.map(|l| args.len() >= l).unwrap_or(false) {
                break;
            } else {
                args.push(self.last_frame()?.get_value_index(index));
                self.increase_pointer(1)?;
            }
        }
        Ok(args)
    }

    pub fn run_tail_call(&mut self, function_index: ValueIndex) -> Result<Option<Value>> {
        let args = self.get_call_arguments(None)?;
        let function = self.last_frame()?.get_value_index(function_index);
        let position = self.last_frame()?.return_position;
        self.pop_frame();
        match function {
            Placeholder::Value(Value::Object(Object::Closure(closure))) => {
                let new_closure = Rc::new(closure.add_arguments(args)?);
                if new_closure.arguments.len() == new_closure.function.arity {
                    self.create_and_push_new_frame(new_closure, position, true)?;
                } else {
                    Err(RuntimeError::NotEnoughArguments)
                }
            }
            Placeholder::Value(Value::NativeFunction(nf)) => nf.call(args)?,
            Placeholder::Placeholder(_) => Err(RuntimeError::ValueNotSet),
            _ => Err(RuntimeError::NotAFunction),
        }
    }

    pub fn run_call(&mut self, function_index: ValueIndex, result_index: RegisterIndex) -> Result<Option<Value>> {
	let function = self.last_frame()?.get_value_index(function_index);
        match function {
            Placeholder::Value(Value::Object(Object::Closure(closure))) => {
		todo!()
            }
            Placeholder::Value(Value::NativeFunction(nf)) => Rc::new(Closure {
                function: ClosureType::NativeFunction(nf),
                captures: vec![],
                arguments: vec![],
            })?,
            Placeholder::Placeholder(_) => Err(RuntimeError::ValueNotSet),
            _ => Err(RuntimeError::NotAFunction),
        }	
	Ok(None)
    }

    pub fn step(&mut self) -> Result<Option<Value>> {
        match self.last_frame()?.inside_call {
            Some((function_index, Some(result_index))) => self.run_call(function_index, result_index),
            Some((function_index, None)) => self.run_tail_call(function_index),
            None => {
                let oc = self
                    .last_frame()?
                    .opcode()
                    .ok_or(RuntimeError::NoMoreOpCodes)?;
                match oc {
                    OpCode::Call(function_index, result_target) => todo!(),
                    OpCode::TailCall(_) => todo!(),
                    OpCode::DeclareRecursive(_) => todo!(),
                    OpCode::FillRecursive(_, _) => todo!(),
                    OpCode::CallArgument(_) => todo!(),
                    OpCode::Return(_) => todo!(),
                    OpCode::Jump(_) => todo!(),
                    OpCode::JumpToPositionIfFalse(_, _) => todo!(),
                    OpCode::CopyValue(_, _) => todo!(),
                    OpCode::CloseValue(_) => todo!(),
                    OpCode::CreateClosure(_, _) => todo!(),
                    OpCode::CaptureValue(_) => todo!(),
                    OpCode::Crash => todo!(),
                    OpCode::InsertNativeFunction(_, _) => todo!(),
                }
            }
        }
    }

    pub fn run_create_closure(
        &mut self,
        function_index: FunctionIndex,
        register: RegisterIndex,
    ) -> Result<()> {
        let frame = self.last_frame_mut()?;
        let func = frame.function.functions[function_index.0 as usize].clone();
        let mut captures: Vec<_> = Vec::with_capacity(func.num_captures);

        frame.pointer += 1;

        while let Some(OpCode::CaptureValue(value_index)) = frame.opcode() {
            frame.pointer += 1;
            captures.push(frame.get_value_index(value_index));
        }
        debug_assert!(captures.len() == func.num_captures);

        frame.registers[register.0 as usize] =
            Placeholder::Value(Value::Object(Object::Closure(Rc::new(Closure {
                function: func,
                captures,
                arguments: Vec::with_capacity(func.arity),
            }))));
        Ok(())
    }

    fn run_return(&mut self, return_value_index: ValueIndex) -> Result<()> {
        let value = self.last_frame()?.get_value_index(return_value_index);
        let return_pos = self.last_frame()?.return_position;
        // TODO: will this break garbage collection? No root to value inbetween these calls
        self.pop_frame();
        self.last_frame_mut()?.registers[return_pos] = value;
        Ok(())
    }

    fn call_native_function(
        &mut self,
        native_function: NativeFunction,
        call_type: CallType,
    ) -> Result<()> {
        let result_position = match call_type {
            CallType::Tail => {
                let ret_pos = self.last_frame()?.return_position;
                let args = self.get_function_arguments(native_function.arguments)?;
                self.pop_frame();
                ret_pos
            }
            CallType::NonTail(return_position) => return_position.0 as usize,
        };
        self.pop_frame();
    }

    fn run_call(&mut self, call_type: CallType, closure_index: ValueIndex) -> Result<()> {
        self.increase_pointer(1);
        match self.last_frame()?.get_value_index(closure_index) {
            Placeholder::Value(Value::NativeFunction(nf)) => {
                self.call_native_function(nf, call_type)
            }
            Placeholder::Value(Value::Object(Object::Closure(closure))) => todo!(),
            _ => Err(RuntimeError::NotAFunction),
        }
    }

    // pub fn step(&mut self) -> Result<Option<Value>> {
    //     let oc = self
    //         .last_frame()?
    //         .opcode()
    //         .ok_or(RuntimeError::NoMoreOpCodes)?;
    //     match oc {
    //         OpCode::Call(r, i) => self.call(r.into(), i.into()).map(|_| None),
    //         OpCode::Return(i) => self.vm_return(i.into()),
    //         OpCode::CopyValue(from, target) => {
    //             self.copy_value(from.into(), target.into()).map(|_| None)
    //         }

    //         OpCode::CloseValue(i) => {
    //             self.close_value(i.into());
    //             Ok(None)
    //         }

    //         OpCode::TailCall(r) => self.tail_call(r.into()),
    //         OpCode::CreateClosure(function_index, return_register) => self
    //             .create_closure(function_index.into(), return_register.into())
    //             .map(|_| None),
    //         OpCode::CallArgument(_) => Err(anyhow!(
    //             "Bytecode incorrect. Tried to have a call argument without a previous call"
    //         )),
    //         OpCode::Jump(position) => self.jump(position.into()).map(|_| None),
    //         OpCode::JumpToPositionIfFalse(boolean_register, position) => self
    //             .jump_to_position_if_false(boolean_register.into(), position.into())
    //             .map(|_| None),
    //         OpCode::Crash => Err(anyhow!("CRASH")),
    //         OpCode::InsertNativeFunction(native_fn, position) => self
    //             .insert_native_function(native_fn, position.into())
    //             .map(|_| None),
    //         OpCode::CaptureValue(_) => Err(anyhow!(
    //             "Bytecode incorrect. Tried to capture a value without a previous closure"
    //         )),
    //         OpCode::DeclareRecursive(_) => todo!(),
    //         OpCode::FillRecursive(_, _) => todo!(),
    //     }
    // }

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

    fn jump(&mut self, position: i16) -> Result<()> {
        self.last_frame_mut()?.pointer = self
            .last_frame_mut()?
            .pointer
            .saturating_add_signed(position as isize);
        Ok(())
    }

    fn jump_to_position_if_false(&mut self, boolean_register: usize, position: i16) -> Result<()> {
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
            _ => Err(RuntimeError::NotABoolean),
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
            x => Err(RuntimeError::NotAFunction),
        }
    }

    fn create_closure(&mut self, function_index: usize, result_index: usize) -> Result<()> {
        let function = self.last_frame()?.function.functions[function_index].clone();
        let mut closure = Closure {
            function: function.clone(),
            captures: Vec::new(),
            arguments: Vec::new(),
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
        storage.ensure_filled(frame.register_offset + closure.function.num_registers);
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
                self.last_frame()?.register_offset + self.last_frame()?.function.num_registers;
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
        match self
            .value_in_last_frame(function_register)
            .unwrap_recursive()
            .clone()
        {
            Value::NativeFunction(nf) => {
                let value = self.call_native_function(nf)?;
                *self.value_in_last_frame(result_slot) = value;
            }
            Value::Object(Object::Closure(closure)) => {
                self.create_and_push_new_frame(closure, result_slot, false)?
            }
            x => return Err(RuntimeError::NotAFunction),
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
        self.frames.last_mut().ok_or(RuntimeError::NoLastFrame)
    }

    fn last_frame(&self) -> Result<&Frame> {
        self.frames.last().ok_or(RuntimeError::NoLastFrame)
    }
}
