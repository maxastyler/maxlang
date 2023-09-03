use std::{cell::RefCell, fmt::Debug, ops::Deref, rc::Rc};

use crate::{
    native_function::{self, NativeFunction},
    opcode::OpCode,
    value::{Closure, Function, Object, UpValue, Value},
};

use anyhow::{anyhow, Context, Result};

pub struct Frame {
    pub depth: usize,
    pub pointer: usize,
    pub registers: Vec<Value>,
    pub upvalues: Vec<Rc<RefCell<UpValue>>>,
    pub function: Rc<Function>,
    pub return_position: usize,
}

impl Debug for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "Frame (return_pos: {:?}, pointer: {:?}, depth: {:?}){{\nregisters: {:?}\nupvalues: {:?}\nfunction: {:?}}}",
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
	    self.upvalues,
	    self.function
        ))
    }
}

impl Frame {
    pub fn new(closure: Rc<Closure>, depth: usize, return_position: usize) -> Frame {
        Frame {
            depth,
            pointer: 0,
            registers: vec![Value::Nil; closure.function.registers],
            function: closure.function.clone(),
            return_position,
            upvalues: closure.upvalues.clone(),
        }
    }
    pub fn opcode(&self) -> Result<OpCode<u8, u8>> {
        self.function
            .chunk
            .opcodes
            .get(self.pointer)
            .cloned()
            .context("Could not get opcode for pointer")
    }
}

#[derive(Default)]
pub struct VM {
    pub frames: Vec<Frame>,
    pub open_upvalues: Vec<Rc<RefCell<UpValue>>>,
}

impl Debug for VM {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("VM {{\nframes: [\n")?;
        for fr in self.frames.iter() {
            f.write_str(&format!("{:?}\n", fr))?;
        }
        f.write_str(&format!("],\nopen_upvalues: {:?}\n}}", self.open_upvalues))
    }
}

impl VM {
    pub fn step(&mut self) -> Result<Option<Value>> {
        let oc = self.last_frame()?.opcode()?;
        println!("OC: {:?}", oc);
        match oc {
            OpCode::Call(r, i) => self.call(r.into(), i.into()).map(|_| None),
            OpCode::Return(i) => self.vm_return(i.into()),
            OpCode::CloseUpValue(i) => self.close_upvalue(i.into()).map(|_| None),
            OpCode::CopyValue(from, target) => {
                self.copy_value(from.into(), target.into()).map(|_| None)
            }
            OpCode::LoadConstant(constant_index, target) => self
                .load_constant(constant_index.into(), target.into())
                .map(|_| None),
            OpCode::LoadUpValue(uv_slot, target) => self
                .load_upvalue(uv_slot.into(), target.into())
                .map(|_| None),
            OpCode::CloseValue(i) => self.close_value(i.into()).map(|_| None),
            OpCode::TailCall(r) => self.tail_call(r.into()),
            OpCode::CreateClosure(function_index, return_register) => self
                .create_closure(function_index.into(), return_register.into())
                .map(|_| None),
            OpCode::CaptureUpValueFromLocal(_) | OpCode::CaptureUpValueFromNonLocal(_) => Err(
                anyhow!("Bytecode incorrect. Tried to capture upvalue without creating a closure"),
            ),
            OpCode::CallArgument(_) => Err(anyhow!(
                "Bytecode incorrect. Tried to have a call argument without a previous call"
            )),
            OpCode::Jump(offset) => self.jump(offset.into()).map(|_| None),
            OpCode::JumpToOffsetIfFalse(boolean_register, offset) => self
                .jump_to_offset_if_false(boolean_register.into(), offset.into())
                .map(|_| None),
            OpCode::Crash => Err(anyhow!("CRASH")),
            OpCode::InsertNativeFunction(native_fn, position) => self
                .insert_native_function(native_fn, position.into())
                .map(|_| None),
        }
    }

    fn insert_native_function(&mut self, native_fn: NativeFunction, position: usize) -> Result<()> {
        self.last_frame_mut()?.registers[position] = Value::NativeFunction(native_fn);
        self.increase_pointer(1)?;
        Ok(())
    }

    fn jump(&mut self, offset: usize) -> Result<()> {
        self.increase_pointer(offset)
    }

    fn jump_to_offset_if_false(&mut self, boolean_register: usize, offset: usize) -> Result<()> {
        match self.last_frame()?.registers[boolean_register] {
            Value::Bool(b) => {
                if b {
                    self.increase_pointer(1)
                } else {
                    self.increase_pointer(offset)
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
                    self.last_frame_mut()?.registers[return_position] = value;
                    Ok(None)
                };
            }
            Value::Object(Object::Closure(closure)) => {
                let mut new_frame = Frame::new(
                    closure.clone(),
                    self.last_frame()?.depth + 1,
                    self.last_frame()?.return_position,
                );
                let mut index = 0;
                self.increase_pointer(1)?;
                loop {
                    match self.last_frame()?.opcode()? {
                        OpCode::CallArgument(argument_register) => {
                            let value: Value =
                                self.last_frame()?.registers[argument_register as usize].clone();
                            new_frame.registers[index] = value;
                            self.increase_pointer(1)?;
                            index += 1;
                        }
                        _ => break,
                    }
                }
                if index != new_frame.function.arity {
                    Err(anyhow!("Called function with wrong number of arguments"))
                } else {
                    self.frames.pop();
                    self.frames.push(new_frame);
                    Ok(None)
                }
            }
            _ => Err(anyhow!("Not a function")),
        }
    }

    fn capture_upvalue_from_local(
        &mut self,
        closure: &mut Closure,
        local_position: usize,
    ) -> Result<()> {
        if let Some(uv) = self.find_open_upvalue(self.last_frame()?.depth, local_position) {
            closure.upvalues.push(uv);
        } else {
            let uv = Rc::new(RefCell::new(UpValue::Open {
                frame_number: self.last_frame()?.depth,
                register: local_position,
            }));
            self.open_upvalues.push(uv.clone());
            closure.upvalues.push(uv);
        }
        Ok(())
    }

    fn capture_upvalue_from_nonlocal(
        &mut self,
        closure: &mut Closure,
        upvalue_position: usize,
    ) -> Result<()> {
        closure
            .upvalues
            .push(self.last_frame()?.upvalues[upvalue_position].clone());
        Ok(())
    }

    fn create_closure(&mut self, function_index: usize, result_index: usize) -> Result<()> {
        let function = self.last_frame()?.function.chunk.functions[function_index].clone();
        let mut closure = Closure {
            function,
            upvalues: vec![],
        };
        self.increase_pointer(1)?;
        loop {
            match self.last_frame()?.opcode()? {
                OpCode::CaptureUpValueFromLocal(index) => {
                    self.capture_upvalue_from_local(&mut closure, index.into())?
                }
                OpCode::CaptureUpValueFromNonLocal(index) => {
                    self.capture_upvalue_from_nonlocal(&mut closure, index.into())?
                }
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
    ) -> Result<()> {
        let mut new_frame = Frame::new(closure, self.last_frame()?.depth + 1, result_slot);
        let mut index = 0;
        loop {
            match self.last_frame()?.opcode()? {
                OpCode::CallArgument(argument_register) => {
                    let value: Value =
                        self.last_frame()?.registers[argument_register as usize].clone();
                    new_frame.registers[index] = value;
                    self.increase_pointer(1);
                    index += 1;
                }
                _ => break,
            }
        }
        if index != new_frame.function.arity {
            Err(anyhow!("Called function with wrong number of arguments"))
        } else {
            self.frames.push(new_frame);
            Ok(())
        }
    }

    fn call_native_function(&mut self, function: NativeFunction) -> Result<Value> {
        self.increase_pointer(1)?;
        let mut arguments: Vec<Value> = vec![];
        loop {
            match self.last_frame()?.opcode()? {
                OpCode::CallArgument(argument_register) => {
                    let value: Value =
                        self.last_frame()?.registers[argument_register as usize].clone();
                    arguments.push(value);
                    self.increase_pointer(1);
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
                self.create_and_push_new_frame(closure, result_slot)?
            }
            _ => return Err(anyhow!("Tried to call something that's not a function")),
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

    fn close_upvalue(&mut self, register_position: usize) -> Result<()> {
        let last_frame = self.frames.last_mut().context("Couldn't get last frame")?;
        let depth = last_frame.depth;
        self.open_upvalues.retain_mut(|x| {
            if matches!(*x.borrow(),
			UpValue::Open{frame_number, register}
			if depth == frame_number && register == register_position)
            {
                x.replace(UpValue::Closed(
                    last_frame
                        .registers
                        .splice(register_position..(register_position + 1), [Value::Nil])
                        .next()
                        .unwrap(),
                ));
                false
            } else {
                true
            }
        });
        self.increase_pointer(1)
    }

    fn close_value(&mut self, register_position: usize) -> Result<()> {
        self.last_frame_mut()?
            .registers
            .splice(register_position..(register_position + 1), [Value::Nil])
            .next()
            .context("Could not close value")?;
        self.increase_pointer(1)
    }

    fn find_open_upvalue(&self, depth: usize, position: usize) -> Option<Rc<RefCell<UpValue>>> {
        self.open_upvalues
            .iter()
            .find(|x| match *x.borrow() {
                UpValue::Open {
                    frame_number,
                    register,
                } => depth == frame_number && register == position,
                UpValue::Closed(_) => {
                    unreachable!("There should never be a closed upvalue in the open upvalues list")
                }
            })
            .cloned()
    }

    fn copy_value(&mut self, from: usize, target: usize) -> Result<()> {
        let f = self.last_frame_mut()?;
        f.registers[target] = f.registers[from].clone();
        self.increase_pointer(1)?;
        Ok(())
    }

    fn load_constant(&mut self, constant_index: usize, target: usize) -> Result<()> {
        let f = self.last_frame_mut()?;
        f.registers[target] = f.function.chunk.constants[constant_index].clone();
        self.increase_pointer(1)
    }

    fn load_upvalue(&mut self, upvalue_slot: usize, target: usize) -> Result<()> {
        let upvalue = &self.last_frame()?.upvalues[upvalue_slot];
        let value = match upvalue.borrow().clone() {
            UpValue::Open {
                frame_number,
                register,
            } => self.frames[frame_number].registers[register].clone(),
            UpValue::Closed(v) => v.clone(),
        };
        self.last_frame_mut()?.registers[target] = value;
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
