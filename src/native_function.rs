use std::rc::Rc;

use crate::{
    expression::Symbol,
    value::{Closure, ClosureType, Object, Placeholder, Value},
    vm::RuntimeError,
};

#[derive(Debug, Clone, PartialEq)]
pub enum NativeFunction {
    LessThan,
    Sum,
    Difference,
    Multiply,
}

impl NativeFunction {
    pub fn resolve_symbol(symbol: &Symbol) -> Option<NativeFunction> {
        match &symbol.0[..] {
            _ => None,
        }
    }

    /// The number of arguments
    pub fn arguments(&self) -> usize {
        match self {
            NativeFunction::LessThan => todo!(),
            NativeFunction::Sum => todo!(),
            NativeFunction::Difference => todo!(),
            NativeFunction::Multiply => todo!(),
        }
    }

    pub fn call(&self, args: Vec<Value>) -> std::result::Result<Value, RuntimeError> {
        Err(RuntimeError::TooManyArguments)
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
