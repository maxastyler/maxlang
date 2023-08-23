use std::{cell::RefCell, rc::Rc};

use crate::{
    opcode::OpCode,
    value::{Chunk, Closure, Function, UpValue, Value},
};

use anyhow::{Context, Result};

#[derive(Debug)]
pub struct Frame {
    pub pointer: usize,
    pub stack: Vec<Value>,
    pub upvalues: Vec<Rc<RefCell<UpValue>>>,
    pub function: Rc<Function>,
    pub return_position: Option<usize>,
}

impl Frame {
    pub fn new(closure: Rc<Closure>, return_position: usize) -> Frame {
        Frame {
            pointer: 0,
            stack: vec![Value::Nil; closure.function.registers],
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

#[derive(Default, Debug)]
pub struct VM {
    pub frames: Vec<Frame>,
    pub temporary_storage: Vec<Value>,
}

impl VM {
    pub fn step(&mut self) -> Result<()> {
        let oc = self.last_frame()?.opcode()?;
        println!("OC: {:?}", oc);
        match oc {
            OpCode::Add(result, a1, a2) => self.add(result as usize, a1 as usize, a2 as usize)?,
            OpCode::Call(r) => self.call(r)?,
            OpCode::Save(p) => self.save(p as usize)?,
            OpCode::Dump(f, t) => self.dump(f as usize, t as usize)?,
            OpCode::Return(value_position) => self.return_value(value_position as usize)?,
            OpCode::CloseUpValue(_) => todo!(),
            OpCode::CopyValue(_, _) => todo!(),
            OpCode::LoadConstant(_, _) => todo!(),
            OpCode::LoadUpValue(_, _) => todo!(),
            OpCode::CloseValue(_) => todo!(),
            OpCode::CreateClosure(_, _) => todo!(),
            OpCode::CaptureUpValueFromLocal(_) => todo!(),
            OpCode::CaptureUpValueFromNonLocal(_) => todo!(),
        }
        Ok(())
    }

    fn save(&mut self, position: usize) -> Result<()> {
        let v = self.last_frame()?.stack[position].clone();
        self.temporary_storage.push(v);
        self.increase_pointer(1)?;
        Ok(())
    }

    fn dump(&mut self, pos_from: usize, pos_to: usize) -> Result<()> {
        let slice_to_copy_from = self.temporary_storage.split_off(pos_from);
        let slice_to_insert_into =
            &mut self.last_frame_mut()?.stack[pos_to..pos_to + &slice_to_copy_from.len()];
        slice_to_insert_into.clone_from_slice(&slice_to_copy_from);
        self.increase_pointer(1)?;
        Ok(())
    }

    fn add(&mut self, result_position: usize, arg1: usize, arg2: usize) -> Result<()> {
        let f = self.last_frame_mut()?;
        f.stack[result_position] = Value::Integer(f.stack[arg1].int()? + f.stack[arg2].int()?);
        self.increase_pointer(1)?;
        Ok(())
    }

    /// Call the function that's on the temporaries stack in first position,
    /// with the arguments after
    fn call(&mut self, return_position: u8) -> Result<()> {
        let c = self.temporary_storage[0].closure()?;
        let args = &self.temporary_storage[1..];
        let f = self.last_frame_mut()?;
        f.pointer += 1;
        let mut next_frame = Frame::new(c.clone(), return_position as usize);

        self.frames.push(next_frame);

        Ok(())
    }

    fn return_value(&mut self, return_value_pos: usize) -> Result<()> {
        let v = self.last_frame()?.stack[return_value_pos].clone();
        let p = self
            .last_frame()?
            .return_position
            .context("Could not get return position")?;
        self.frames.pop();
        self.last_frame_mut()?.stack[p] = v;
        Ok(())
    }

    fn increase_pointer(&mut self, amount: usize) -> Result<()> {
        self.last_frame_mut()?.pointer += amount;
        Ok(())
    }

    fn last_frame_mut(&mut self) -> Result<&mut Frame> {
        self.frames.last_mut().context("Could not get last frame as mutable reference")
    }

    fn last_frame(&self) -> Result<&Frame> {
        self.frames.last().context("Could not get last frame")
    }
}
