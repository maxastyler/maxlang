use anyhow::{anyhow, Result};
use std::{cell::RefCell, rc::Rc};

use crate::{expression::Literal, native_function::NativeFunction, opcode::OpCode};

#[derive(Debug, Clone)]
pub struct Chunk {
    pub opcodes: Vec<OpCode<u8, u8>>,
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
    Open {
        frame_number: usize,
        register: usize,
    },
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
    NativeFunction(NativeFunction),
    Object(Object),
}

impl From<Literal> for Value {
    fn from(value: Literal) -> Self {
        match value {
            Literal::Int(i) => Value::Integer(i),
            Literal::Bool(b) => Value::Bool(b),
        }
    }
}

impl Value {
    pub fn int(&self) -> Result<i64> {
	println!("The integer is {:?}", self);
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
