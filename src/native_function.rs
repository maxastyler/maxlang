use crate::{expression::Symbol, value::Value};
use anyhow::Result;

#[derive(Debug, Clone)]
pub enum NativeFunction {
    LessThan,
    Sum,
    Difference,
}

impl NativeFunction {
    pub fn resolve_symbol(symbol: &Symbol) -> Option<NativeFunction> {
        match &symbol.0[..] {
            "<" => Some(NativeFunction::LessThan),
            "+" => Some(NativeFunction::Sum),
            "-" => Some(NativeFunction::Difference),
            _ => None,
        }
    }

    pub fn call(&self, arguments: &[Value]) -> Result<Value> {
        match self {
            NativeFunction::LessThan => less_than(&arguments[0], &arguments[1]),
            NativeFunction::Sum => sum(&arguments[0], &arguments[1]),
            NativeFunction::Difference => difference(&arguments[0], &arguments[1]),
        }
    }
}

fn less_than(a: &Value, b: &Value) -> Result<Value> {
    Ok(Value::Bool(a.int()? < b.int()?))
}

fn sum(a: &Value, b: &Value) -> Result<Value> {
    Ok(Value::Integer(a.int()? + b.int()?))
}

fn difference(a: &Value, b: &Value) -> Result<Value> {
    Ok(Value::Integer(a.int()? - b.int()?))
}
