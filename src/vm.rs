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
    value::{Closure, ClosureType, Function, Object, Placeholder, Value, ValueError},
};

#[derive(Clone, Debug)]
pub enum RuntimeError {
    NoMoreOpCodes,
    NotAFunction,
    NotABoolean,
    NoLastFrame,
    ValueNotSet,
    TooManyArguments,
    NotEnoughArguments,
    Crash,
    ValueError(ValueError),
}

impl From<ValueError> for RuntimeError {
    fn from(value: ValueError) -> Self {
        RuntimeError::ValueError(value)
    }
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
            function: ClosureType::Function(Rc::new(f)),
            captures: vec![],
            arguments: vec![],
        });
        let mut vm = VM::default();
        vm.frames.push(Frame::new_from_closure(closure, 0));
        vm
    }

    /// Get the call arguments up to an optional limit.
    /// If there are no more call arguments, set inside call to None
    pub fn get_call_arguments(&mut self, limit: Option<usize>) -> Result<Vec<Value>> {
        let mut args = vec![];
        while let Some(OpCode::CallArgument(index)) = self.last_frame()?.opcode() {
            if limit.map(|l| args.len() >= l).unwrap_or(false) {
                return Ok(args);
            } else {
                match self.last_frame()?.get_value_index(index) {
                    Placeholder::Placeholder(_) => unreachable!(),
                    Placeholder::Value(v) => args.push(v.clone()),
                }
                self.increase_pointer(1);
            }
        }
        self.last_frame_mut()?.inside_call = None;
        Ok(args)
    }

    pub fn run_tail_call(&mut self, function_index: ValueIndex) -> Result<Option<Value>> {
        self.increase_pointer(1);
        let args = self.get_call_arguments(None)?;
        let function = match self.last_frame()?.get_value_index(function_index) {
            Placeholder::Placeholder(v) => v.borrow().clone(),
            Placeholder::Value(v) => v,
        };
        let position = self.last_frame()?.return_position;
        self.pop_frame();
        match function {
            Value::Object(Object::Closure(closure)) => {
                let new_closure = Rc::new(closure.add_arguments(args).unwrap());
                if new_closure.arguments.len() == new_closure.function.arity() {
                    match new_closure.function.clone() {
                        ClosureType::Function(_) => {
                            self.create_and_push_new_frame(new_closure, position);
                            Ok(None)
                        }
                        ClosureType::NativeFunction(nf) => {
                            let result = nf.call(new_closure.arguments.clone())?;
                            if self.frames.is_empty() {
                                Ok(Some(result))
                            } else {
                                self.last_frame_mut()?.registers[position] =
                                    Placeholder::Value(result);
                                Ok(None)
                            }
                        }
                    }
                } else {
                    Err(RuntimeError::NotEnoughArguments)
                }
            }
            Value::NativeFunction(nf) => {
                let result = nf.call(args)?;
                if self.frames.is_empty() {
                    Ok(Some(result))
                } else {
                    self.last_frame_mut()?.registers[position] = Placeholder::Value(result);
                    Ok(None)
                }
            }
            x => Err(RuntimeError::NotAFunction),
        }
    }

    pub fn run_call(
        &mut self,
        function_index: ValueIndex,
        result_index: RegisterIndex,
    ) -> Result<Option<Value>> {
        self.increase_pointer(1);
        let function = self.last_frame()?.get_value_index(function_index);
        match function {
            Placeholder::Value(Value::Object(Object::Closure(closure))) => {
                let args = self.get_call_arguments(Some(closure.arguments_needed()))?;
                let new_closure = closure.add_arguments(args).unwrap();
                if new_closure.arguments_needed() == 0 {
                    self.create_and_push_new_frame(Rc::new(new_closure), result_index.0 as usize);
                } else {
                    self.last_frame_mut()?.registers[result_index.0 as usize] =
                        Placeholder::Value(Value::Object(Object::Closure(Rc::new(new_closure))));
                }
            }
            Placeholder::Value(Value::NativeFunction(nf)) => {
                let num_args = nf.arguments();
                let args = self.get_call_arguments(Some(num_args))?;
                self.last_frame_mut()?.registers[result_index.0 as usize] =
                    nf.call_or_curry(args)?;
            }
            Placeholder::Placeholder(_) => return Err(RuntimeError::ValueNotSet),
            x => {
                return Err(RuntimeError::NotAFunction);
            }
        };
        Ok(None)
    }

    pub fn step(&mut self) -> Result<Option<Value>> {
        match self.last_frame()?.inside_call.clone() {
            Some((function_index, Some(result_index))) => {
                self.run_call(function_index, result_index)
            }
            Some((function_index, None)) => self.run_tail_call(function_index),
            None => {
                let oc = self
                    .last_frame()?
                    .opcode()
                    .ok_or(RuntimeError::NoMoreOpCodes)?;
                match oc {
                    OpCode::Call(function_index, result_target) => {
                        self.run_call(function_index, result_target)
                    }
                    OpCode::TailCall(function_index) => self.run_tail_call(function_index),
                    OpCode::DeclareRecursive(index) => {
                        self.last_frame_mut()?.run_declare_recursive(index);
                        Ok(None)
                    }
                    OpCode::FillRecursive(value_index, register_index) => {
                        self.last_frame_mut()?
                            .run_fill_recursive(value_index, register_index);
                        Ok(None)
                    }
                    OpCode::CallArgument(_) => unreachable!(),
                    OpCode::Return(value_index) => self.run_return(value_index),
                    OpCode::Jump(distance) => {
                        self.last_frame_mut()?.run_jump(distance as isize);
                        Ok(None)
                    }
                    OpCode::JumpToPositionIfFalse(check_index, jump) => {
                        self.last_frame_mut()?
                            .run_jump_to_position_if_false(check_index, jump as isize)?;
                        Ok(None)
                    }
                    OpCode::CopyValue(value_from, value_to) => {
                        self.last_frame_mut()?.run_copy_value(value_from, value_to);
                        Ok(None)
                    }
                    OpCode::CloseValue(index) => {
                        self.last_frame_mut()?.run_close_value(index);
                        Ok(None)
                    }
                    OpCode::CreateClosure(function_index, register_index) => {
                        self.run_create_closure(function_index, register_index);
                        Ok(None)
                    }
                    OpCode::CaptureValue(_) => unreachable!(),
                    OpCode::Crash => Err(RuntimeError::Crash),
                    OpCode::InsertNativeFunction(native_function, index) => {
                        self.last_frame_mut()?
                            .run_insert_native_function(native_function, index);
                        Ok(None)
                    }
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
            captures.push(frame.get_value_index(value_index).clone());
        }
        debug_assert!(captures.len() == func.num_captures);

        frame.registers[register.0 as usize] =
            Placeholder::Value(Value::Object(Object::Closure(Rc::new(Closure {
                function: ClosureType::Function(func.clone()),
                captures,
                arguments: Vec::with_capacity(func.arity),
            }))));
        Ok(())
    }

    fn run_return(&mut self, return_value_index: ValueIndex) -> Result<Option<Value>> {
        self.increase_pointer(1);
        let value = self.last_frame()?.get_value_index(return_value_index);
        let return_pos = self.last_frame()?.return_position;
        // TODO: will this break garbage collection? No root to value inbetween these calls
        self.pop_frame();
        if self.frames.is_empty() {
            match value {
                Placeholder::Placeholder(_) => unreachable!(),
                Placeholder::Value(value) => Ok(Some(value.clone())),
            }
        } else {
            self.last_frame_mut()?.registers[return_pos] = value.clone();
            Ok(None)
        }
    }

    /// Creates a new frame from a closure and arguments in the VM's temporary storage
    /// Pushes the new frame onto the current stack of frames
    fn create_and_push_new_frame(&mut self, closure: Rc<Closure>, result_slot: usize) {
        let new_frame = Frame::new_from_closure(closure, result_slot);
        self.frames.push(new_frame);
    }

    fn pop_frame(&mut self) {
        self.frames.pop();
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
