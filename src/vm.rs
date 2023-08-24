use std::{cell::RefCell, fmt::Debug, ops::Deref, rc::Rc};

use crate::{
    opcode::OpCode,
    value::{Chunk, Closure, Function, Object, UpValue, Value},
};

use anyhow::{anyhow, Context, Result};

pub struct Frame {
    pub depth: usize,
    pub pointer: usize,
    pub registers: Vec<Value>,
    pub upvalues: Vec<Rc<RefCell<UpValue>>>,
    pub function: Rc<Function>,
    pub return_position: Option<usize>,
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
            return_position: Some(return_position),
            upvalues: closure.upvalues.clone(),
        }
    }
    pub fn opcode(&self) -> Result<OpCode<u8>> {
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
    pub temporary_storage: Vec<Value>,
    pub open_upvalues: Vec<Rc<RefCell<UpValue>>>,
}

impl Debug for VM {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("VM {{\nframes: [\n")?;
	for fr in self.frames.iter() {
	    f.write_str(&format!("{:?}\n", fr))?;
	}
        f.write_str(&format!(
            "],\ntemporary_storage: {:?},\nopen_upvalues: {:?}\n}}",
            self.temporary_storage, self.open_upvalues
        ))
    }
}

impl VM {
    pub fn step(&mut self) -> Result<()> {
        let oc = self.last_frame()?.opcode()?;
        println!("OC: {:?}", oc);
        match oc {
            OpCode::Call(i) => self.call(i.into()),
            OpCode::Save(i) => self.save(i.into()),
            OpCode::Return(i) => self.vm_return(i.into()),
            OpCode::CloseUpValue(i) => self.close_upvalue(i.into()),
            OpCode::CopyValue(from, target) => self.copy_value(from.into(), target.into()),
            OpCode::LoadConstant(constant_index, target) => {
                self.load_constant(constant_index.into(), target.into())
            }
            OpCode::LoadUpValue(uv_slot, target) => {
                self.load_upvalue(uv_slot.into(), target.into())
            }
            OpCode::CloseValue(i) => self.close_value(i.into()),
            OpCode::CreateClosure(function_index, return_register) => {
                self.create_closure(function_index.into(), return_register.into())
            }
            _ => Err(anyhow!(
                "Bytecode incorrect. Tried to capture upvalue without creating a closure"
            )),
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
    fn create_and_push_new_frame(&mut self, result_slot: usize) -> Result<()> {
        let closure = self
            .temporary_storage
            .get(0)
            .context("Could not get closure from temporary storage")?
            .closure()?;
        let mut new_frame = Frame::new(closure, self.last_frame()?.depth + 1, result_slot);
        new_frame.registers.splice(
            0..new_frame.function.arity,
            self.temporary_storage
                .drain(1..(new_frame.function.arity + 1)),
        );
        self.frames.push(new_frame);
        self.temporary_storage.clear();
        Ok(())
    }

    fn call(&mut self, result_slot: usize) -> Result<()> {
        self.increase_pointer(1)?;
        self.create_and_push_new_frame(result_slot)?;
        Ok(())
    }

    fn save(&mut self, position: usize) -> Result<()> {
        let v = self.last_frame()?.registers[position].clone();
        self.temporary_storage.push(v);
        self.increase_pointer(1)?;
        Ok(())
    }

    fn vm_return(&mut self, return_value_position: usize) -> Result<()> {
        let value = self
            .last_frame_mut()?
            .registers
            .swap_remove(return_value_position);
        let return_position = self
            .last_frame()?
            .return_position
            .context("Could not get return position")?;
        self.frames.pop();
        self.last_frame_mut()?.registers[return_position] = value;
        Ok(())
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
