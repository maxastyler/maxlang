use std::rc::Rc;

use crate::{
    expression::Symbol,
    value::{Closure, ClosureType, Object, Placeholder, Value},
    vm::RuntimeError,
};

#[derive(Debug, Clone, PartialEq)]
pub enum NativeFunction {
    LessThan,
    GreaterThan,
    Equal,
    GreaterThanEqual,
    LessThanEqual,
    Sum,
    Difference,
    Multiply,
    Quotient,
    Print,
    Index,
    Push,
    Set,
}

impl NativeFunction {
    pub fn resolve_symbol(symbol: &Symbol) -> Option<NativeFunction> {
        match &symbol.0[..] {
            "+" => Some(NativeFunction::Sum),
            "lt" => Some(NativeFunction::LessThan),
            "lte" => Some(Self::LessThanEqual),
            "gt" => Some(Self::GreaterThan),
            "gte" => Some(Self::GreaterThanEqual),
            "=" => Some(Self::Equal),
            "/" => Some(Self::Quotient),
            "*" => Some(Self::Multiply),
            "-" => Some(Self::Difference),
            "print" => Some(Self::Print),
            "ind" => Some(Self::Index),
            "push" => Some(Self::Push),
            "set" => Some(Self::Set),
            _ => None,
        }
    }

    /// The number of arguments
    pub fn arguments(&self) -> usize {
        match self {
            Self::Set => 3,
            NativeFunction::LessThan
            | Self::GreaterThan
            | NativeFunction::Difference
            | NativeFunction::Equal
            | NativeFunction::GreaterThanEqual
            | NativeFunction::LessThanEqual
            | NativeFunction::Multiply
            | NativeFunction::Quotient
            | NativeFunction::Sum
            | Self::Index
            | Self::Push => 2,
            Self::Print => 1,
        }
    }

    pub fn call(&self, args: Vec<Value>) -> std::result::Result<Value, RuntimeError> {
        match self {
            NativeFunction::LessThan => Ok(Value::Bool(args[0].number()? < args[1].number()?)),
            NativeFunction::Sum => Ok(Value::Number(args[0].number()? + args[1].number()?)),
            NativeFunction::Difference => Ok(Value::Number(args[0].number()? - args[1].number()?)),
            NativeFunction::GreaterThan => Ok(Value::Bool(args[0].number()? > args[1].number()?)),
            NativeFunction::Equal => Ok(Value::Bool(args[0] == args[1])),
            NativeFunction::GreaterThanEqual => {
                Ok(Value::Bool(args[0].number()? >= args[1].number()?))
            }
            NativeFunction::LessThanEqual => {
                Ok(Value::Bool(args[0].number()? <= args[1].number()?))
            }
            NativeFunction::Multiply => Ok(Value::Number(args[0].number()? * args[1].number()?)),
            NativeFunction::Quotient => Ok(Value::Number(args[0].number()? / args[1].number()?)),
            NativeFunction::Print => {
                println!("{:?}", args[0]);
                Ok(args[0].clone())
            }
            NativeFunction::Index => {
                let l = args[0].list()?;
                Ok(l[args[1].number()? as usize].clone())
            }
            NativeFunction::Push => {
                let mut l = args[0].list()?.clone();
                l.push_back(args[1].clone());
                Ok(Value::List(l))
            }
            NativeFunction::Set => {
                let mut l = args[0].list()?.clone();
                l.set(args[1].number()? as usize, args[2].clone());
                Ok(Value::List(l))
            }
        }
    }

    pub fn call_or_curry(
        &self,
        args: Vec<Value>,
    ) -> std::result::Result<Placeholder, RuntimeError> {
        Ok(match args.len().cmp(&args.len()) {
            std::cmp::Ordering::Less => Placeholder::Value(Value::Object(Object::Closure(
                Rc::new(self.to_closure(args)),
            ))),
            std::cmp::Ordering::Equal => Placeholder::Value(self.call(args)?),
            std::cmp::Ordering::Greater => unreachable!(),
        })
    }

    pub fn to_closure(&self, args: Vec<Value>) -> Closure {
        debug_assert!(args.len() < self.arguments());
        Closure {
            function: ClosureType::NativeFunction(self.clone()),
            captures: vec![],
            arguments: args,
        }
    }
}
