use std::{cell::RefCell, rc::Rc};

use crate::value::{Chunk, Closure, Function, UpValue, Value};

type R<X> = Result<X, &'static str>;

type REGISTER_WINDOW = u8;

#[derive(Debug, Clone)]
pub enum OpCode {
    Add(u8, u8, u8),
    /// Call the function in temporary storage, with the arguments
    Call(u8),
    /// Save the value in the given position
    /// to the VM's temporary storage
    Save(u8),
    /// Dump the values from the given position
    /// in the VM's temporary storage to the position starting with
    Dump(u8, u8),
    /// Return the value in the given register
    Return(u8),
    /// Close the upvalue in the given position
    CloseUpValue(u8),
    /// Copy the value from 0 to 1
    CopyValue(u8, u8),
    /// Load the constant from the constants array at 0 to the position 1
    LoadConstant(u8, u8),
    /// Load the upvalue in the given upvalue slot into the given slot
    LoadUpValue(u8, u8),
    /// Free the value at the given index
    CloseValue(u8),
    /// Create closure. Takes the index of the function in the current chunk,
    /// puts the result in the register .1
    CreateClosure(u8, u8),
    /// Capture an upvalue from a local in the function above
    CaptureUpValueFromLocal(u8),
    /// Capture an upvalue from the above function's upvalues
    CaptureUpValueFromNonLocal(u8),
}

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
    pub fn opcode(&self) -> R<OpCode> {
        self.function
            .chunk
            .opcodes
            .get(self.pointer)
            .cloned()
            .ok_or("Could not get opcode for pointer")
    }
}

#[derive(Default, Debug)]
pub struct VM {
    pub frames: Vec<Frame>,
    pub temporary_storage: Vec<Value>,
}

impl VM {
    pub fn step(&mut self) -> R<()> {
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

    fn save(&mut self, position: usize) -> R<()> {
        let v = self.last_frame()?.stack[position].clone();
        self.temporary_storage.push(v);
        self.increase_pointer(1)?;
        Ok(())
    }

    fn dump(&mut self, pos_from: usize, pos_to: usize) -> R<()> {
        let slice_to_copy_from = self.temporary_storage.split_off(pos_from);
        let slice_to_insert_into =
            &mut self.last_frame_mut()?.stack[pos_to..pos_to + &slice_to_copy_from.len()];
        slice_to_insert_into.clone_from_slice(&slice_to_copy_from);
        self.increase_pointer(1)?;
        Ok(())
    }

    fn add(&mut self, result_position: usize, arg1: usize, arg2: usize) -> R<()> {
        let f = self.last_frame_mut()?;
        f.stack[result_position] = Value::Integer(f.stack[arg1].int()? + f.stack[arg2].int()?);
        self.increase_pointer(1)?;
        Ok(())
    }

    /// Call the function that's on the temporaries stack in first position,
    /// with the arguments after
    fn call(&mut self, return_position: u8) -> R<()> {
        let c = self.temporary_storage[0].closure()?;
        let args = &self.temporary_storage[1..];
        let f = self.last_frame_mut()?;
        f.pointer += 1;
        let mut next_frame = Frame::new(c.clone(), return_position as usize);

        self.frames.push(next_frame);

        Ok(())
    }

    fn return_value(&mut self, return_value_pos: usize) -> R<()> {
        let v = self.last_frame()?.stack[return_value_pos].clone();
        let p = self
            .last_frame()?
            .return_position
            .ok_or("No return position for current frame")?;
        self.frames.pop();
        self.last_frame_mut()?.stack[p] = v;
        Ok(())
    }

    fn increase_pointer(&mut self, amount: usize) -> R<()> {
        self.last_frame_mut()?.pointer += amount;
        Ok(())
    }

    fn last_frame_mut(&mut self) -> R<&mut Frame> {
        self.frames.last_mut().ok_or("Could not get last frame")
    }

    fn last_frame(&self) -> R<&Frame> {
        self.frames.last().ok_or("Could not get last frame")
    }
}
