use std::rc::Rc;

use crate::vm::OpCode;

#[derive(Debug)]
pub struct Chunk {
    pub opcodes: Vec<OpCode>,
    pub constants: Vec<Value>,
    pub functions: Vec<Rc<Function>>,
}

#[derive(Debug)]
pub struct Function {
    pub chunk: Chunk,
    pub registers: usize,
}

#[derive(Debug, Clone)]
pub struct Closure {
    pub function: Rc<Function>,
}

#[derive(Debug, Clone)]
pub enum UpValue {
    Open(usize),
    Closed(Value),
}

#[derive(Debug, Clone)]
pub enum Object {
    Closure(Rc<Closure>),
    UpValue(Rc<UpValue>),
}

#[derive(Debug, Clone)]
pub enum Value {
    Integer(i64),
    Nil,
    Object(Object),
}

impl Value {
    pub fn int(&self) -> Result<i64, &'static str> {
        match self {
            Value::Integer(i) => Ok(*i),
            _ => Err("Not an integer"),
        }
    }

    pub fn closure(&self) -> Result<Rc<Closure>, &'static str> {
        match self {
            Value::Object(Object::Closure(c)) => Ok(c.clone()),
            _ => Err("Not a closure"),
        }
    }
}
