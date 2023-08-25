use std::{cell::RefCell, rc::Rc};
use anyhow::{Result, anyhow};

use crate::opcode::OpCode;

#[derive(Debug, Clone)]
pub struct Chunk {
    pub opcodes: Vec<OpCode<u8>>,
    pub constants: Vec<Value>,
    pub functions: Vec<Rc<Function>>,
}

#[derive(Debug)]
pub struct Function {
    pub chunk: Chunk,
    pub arity: usize,
    pub registers: usize,
}

#[derive(Debug, Clone)]
pub struct Closure {
    pub function: Rc<Function>,
    pub upvalues: Vec<Rc<RefCell<UpValue>>>,
}

#[derive(Debug, Clone)]
pub enum UpValue {
    Open{frame_number: usize, register: usize},
    Closed(Value),
}

#[derive(Debug, Clone)]
pub enum Object {
    Closure(Rc<Closure>),
    UpValue(Rc<RefCell<UpValue>>),
}

#[derive(Debug, Clone)]
pub enum Value {
    Integer(i64),
    Bool(bool),
    Nil,
    Object(Object),
}

impl Value {
    pub fn int(&self) -> Result<i64> {
        match self {
            Value::Integer(i) => Ok(*i),
            _ => Err(anyhow!("Not an integer!")),
        }
    }

    pub fn closure(&self) -> Result<Rc<Closure>> {
        match self {
            Value::Object(Object::Closure(c)) => Ok(c.clone()),
            _ => Err(anyhow!("Not a closure!")),
        }
    }
}
