use anyhow::{anyhow, Result};
use im::HashMap;
use im::Vector;
use std::{cell::RefCell, rc::Rc};

use crate::native_function::NativeFunction;
use crate::{compiler::FrameIndex, expression::Literal, opcode::OpCode};

#[derive(Debug)]
pub struct Function {
    pub opcodes: Vec<OpCode<u8>>,
    pub constants: Vec<Value>,
    pub functions: Vec<Rc<Function>>,
    pub arity: usize,
    pub capture_offset: usize,
    pub registers: usize,
}

#[derive(Debug, Clone)]
pub struct Closure {
    pub function: Rc<Function>,
    pub captures: Vec<Value>,
}

#[derive(Debug, Clone)]
pub enum Object {
    Closure(Rc<Closure>),
    String(Rc<String>),
}

#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
    Bool(bool),
    Nil,
    Uninit,
    List(Vector<Value>),
    Dictionary(HashMap<Value, Value>),
    NativeFunction(NativeFunction),
    Object(Object),
}

impl From<Literal> for Value {
    fn from(value: Literal) -> Self {
        match value {
            Literal::Bool(b) => Value::Bool(b),
            Literal::Nil => Value::Nil,
            Literal::Number(n) => Value::Number(n),
            Literal::String(s) => Value::,
            Literal::Quoted(_) => todo!(),
            Literal::List(_) => todo!(),
            Literal::Dictionary(_) => todo!(),
        }
    }
}

impl Value {
    pub fn double(&self) -> Result<f64> {
        match self {
            Value::Double(i) => Ok(*i),
            _ => Err(anyhow!("Not a double!")),
        }
    }
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
