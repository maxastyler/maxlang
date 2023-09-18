use im::HashMap;
use im::Vector;
use std::cell::Ref;
use std::fmt::Debug;
use std::{cell::RefCell, rc::Rc};

use crate::native_function::NativeFunction;
use crate::{expression::Literal, opcode::OpCode};

enum ValueError {
    NotANumber,
    NotAClosure,
    TooManyArguments,
}

type Result<Ok> = std::result::Result<Ok, ValueError>;

#[derive(Debug)]
pub struct Function {
    pub opcodes: Vec<OpCode>,
    pub constants: Vec<Value>,
    pub functions: Vec<Rc<Function>>,
    pub arity: usize,
    pub num_captures: usize,
    pub num_registers: usize,
}

pub enum ClosureType {
    Function(Rc<Function>),
    NativeFunction(NativeFunction),
}

impl ClosureType {
    pub fn arity(&self) -> usize {
        match self {
            ClosureType::Function(f) => f.arity,
            ClosureType::NativeFunction(nf) => nf.arguments(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Closure {
    pub function: ClosureType,
    pub captures: Vec<Placeholder>,
    pub arguments: Vec<Value>,
}

impl Closure {
    pub fn add_arguments(&self, args: Vec<Value>) -> Result<Closure> {
        if args.len() + self.arguments.len() > self.function.arity() {
            Err(ValueError::TooManyArguments)
        } else {
            let mut new = self.clone();
            new.arguments.extend(args);
            Ok(new)
        }
    }
}

#[derive(Debug, Clone)]
pub enum Object {
    Closure(Rc<Closure>),
    String(Rc<String>),
}

#[derive(Clone, Debug)]
pub enum Placeholder {
    Placeholder(Rc<RefCell<Value>>),
    Value(Value),
}

impl Placeholder {
    pub fn unwrap(&self) -> Value {
        match self {
            Placeholder::Placeholder(r) => r.borrow().clone(),
            Placeholder::Value(v) => v.clone(),
        }
    }
}

#[derive(Clone)]
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

impl Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(n) => f.write_fmt(format_args!("{:?}", n)),
            Value::Bool(_) => todo!(),
            Value::Nil => todo!(),
            Value::Uninit => todo!(),
            Value::List(_) => todo!(),
            Value::Dictionary(_) => todo!(),
            Value::NativeFunction(_) => todo!(),
            Value::Object(_) => todo!(),
        }
    }
}

impl<'a> From<Literal<'a>> for Value {
    fn from(value: Literal) -> Self {
        match value {
            Literal::Bool(b) => Value::Bool(b),
            Literal::Nil => Value::Nil,
            Literal::Number(n) => Value::Number(n),
            Literal::String(s) => todo!(),
            Literal::Quoted(_) => todo!(),
            Literal::List(_) => todo!(),
            Literal::Dictionary(_) => todo!(),
        }
    }
}

impl Value {
    pub fn number(&self) -> Result<f64> {
        match self {
            Value::Number(i) => Ok(*i),
            _ => Err(ValueError::NotANumber),
        }
    }

    pub fn closure(&self) -> Result<Rc<Closure>> {
        match self {
            Value::Object(Object::Closure(c)) => Ok(c.clone()),
            _ => Err(ValueError::NotAClosure),
        }
    }
}
