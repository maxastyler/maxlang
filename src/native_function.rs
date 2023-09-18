use crate::{expression::Symbol, value::Value, vm::RuntimeError};

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
}
