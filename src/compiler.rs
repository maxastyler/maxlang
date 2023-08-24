use std::rc::Rc;

use anyhow::{anyhow, Context, Result};

use crate::{
    opcode::OpCode,
    value::{Chunk, Function, Value},
};

#[derive(PartialEq, Clone, Debug)]
pub struct Symbol(pub String);

#[derive(Debug, Clone)]
pub struct Literal(pub i64);

#[derive(Debug, Clone)]
pub enum Expression {
    Call(Box<Expression>, Vec<Expression>),
    Assign(Symbol, Box<Expression>),
    Function(Vec<Symbol>, Box<Expression>),
    Block(Vec<Expression>, Box<Expression>),
    Literal(Literal),
    Symbol(Symbol),
}

impl From<i64> for Expression {
    fn from(value: i64) -> Self {
        Expression::Literal(Literal(value))
    }
}

impl From<(Expression, Vec<Expression>)> for Expression {
    fn from(value: (Expression, Vec<Expression>)) -> Self {
        Expression::Call(value.0.into(), value.1)
    }
}

impl From<(&str, Expression)> for Expression {
    fn from(value: (&str, Expression)) -> Self {
        Expression::Assign(Symbol(value.0.into()), value.1.into())
    }
}

impl From<(Vec<&str>, Expression)> for Expression {
    fn from(value: (Vec<&str>, Expression)) -> Self {
        Expression::Function(
            value.0.into_iter().map(|x| Symbol(x.into())).collect(),
            value.1.into(),
        )
    }
}

impl From<Vec<Expression>> for Expression {
    fn from(value: Vec<Expression>) -> Self {
        match &value[..] {
            [ref ignored @ .., last] => Expression::Block(
                ignored.into_iter().map(|x| (*x).clone()).collect(),
                last.clone().into(),
            ),
            _ => panic!("Block expression vector should have 1 or more elements"),
        }
    }
}

impl From<&str> for Expression {
    fn from(value: &str) -> Self {
        Expression::Symbol(Symbol(value.into()))
    }
}

#[derive(Clone, Debug)]
pub struct Named {
    pub name: Symbol,
    pub depth: usize,
    pub captured: bool,
}

#[derive(Clone, Debug)]
pub enum Local {
    Named(Named),
    /// A temporary variable at the given depth
    Temporary,
    None,
}

#[derive(PartialEq, Clone, Debug)]
pub struct CUpValue {
    position: usize,
    local: bool,
}

#[derive(Clone, Debug)]
pub struct Compiler<const N: usize> {
    pub locals: [Local; N],
    pub upvalues: Vec<CUpValue>,
    pub depth: usize,
    pub previous: Option<Box<Compiler<N>>>,
    pub chunk: Chunk,
}

impl<const N: usize> Compiler<N> {
    pub fn new(previous: Option<Box<Compiler<N>>>) -> Self {
        Self {
            locals: std::array::from_fn(|_| Local::None),
            upvalues: vec![],
            depth: previous.as_ref().map(|x| x.depth + 1).unwrap_or(0),
            previous,
            chunk: Chunk {
                constants: vec![],
                opcodes: vec![],
                functions: vec![],
            },
        }
    }

    pub fn compile_expression(&mut self, expression: Expression) -> Result<usize> {
        println!("COMPILER_STATE: {:?}", self);
        println!("EXPRESSION: {:?}\n\n", expression);
        let expression_position = match expression {
            Expression::Call(function, arguments) => self.compile_call(*function, arguments)?,
            Expression::Assign(symbol, expression) => self.compile_assign(symbol, *expression)?,
            Expression::Function(arguments, body) => self.compile_function(arguments, *body)?,
            Expression::Block(ignored_expressions, result_expression) => {
                self.compile_block(ignored_expressions, *result_expression)?
            }
            Expression::Literal(literal) => self.compile_literal(literal)?,
            Expression::Symbol(symbol) => self.compile_symbol(symbol)?,
        };
        Ok(expression_position)
    }

    fn compile_call(&mut self, function: Expression, arguments: Vec<Expression>) -> Result<usize> {
        let function_index = self.compile_expression(function)?;
        self.chunk
            .opcodes
            .push(OpCode::Save(function_index.try_into()?));
        self.clear_all_to_depth()?;
        for arg in arguments {
            let a_index = self.compile_expression(arg)?;
            self.chunk.opcodes.push(OpCode::Save(a_index.try_into()?));
            self.clear_all_to_depth()?;
        }
        let result_index = self.find_free_register()?;
        self.locals[result_index] = Local::Temporary;
        self.chunk
            .opcodes
            .push(OpCode::Call(result_index.try_into()?));
        Ok(result_index)
    }

    /// Get the value associated to the given symbol, and return the slot it is in
    /// If the value is an upvalue, emit an instruction to move the upvalue into a slot,
    /// then return the slot index
    fn compile_symbol(&mut self, symbol: Symbol) -> Result<usize> {
        if let Some(i) = self.find_local_symbol(&symbol) {
            Ok(i)
        } else if let Some(i) = self.find_nonlocal_symbol(&symbol) {
            let out_index = self.find_free_register()?;
            self.chunk
                .opcodes
                .push(OpCode::LoadUpValue(i.try_into()?, out_index.try_into()?));
            self.locals[out_index] = Local::Temporary;
            Ok(out_index)
        } else {
            Err(anyhow!("Couldn't get a symbol"))
        }
    }

    fn find_nonlocal_symbol(&mut self, symbol: &Symbol) -> Option<usize> {
        if let Some(p) = &mut self.previous {
            if let Some(l_pos) = p.find_local_symbol(symbol) {
                match p.locals.get_mut(l_pos as usize) {
                    Some(Local::Named(n)) => n.captured = true,
                    _ => panic!("Find nonlocal captured bad value"),
                };
                Some(self.add_upvalue(CUpValue {
                    local: true,
                    position: l_pos,
                }))
            } else if let Some(nl_pos) = self.find_nonlocal_symbol(symbol) {
                Some(self.add_upvalue(CUpValue {
                    local: false,
                    position: nl_pos,
                }))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn add_upvalue(&mut self, upvalue: CUpValue) -> usize {
        self.upvalues
            .iter()
            .enumerate()
            .find_map(|(i, uv)| if *uv == upvalue { Some(i) } else { None })
            .unwrap_or_else(|| {
                self.upvalues.push(upvalue);
                self.upvalues.len() - 1
            })
    }

    /// Finds the symbol with the same name at the highest depth <= self.depth
    fn find_local_symbol(&self, symbol: &Symbol) -> Option<usize> {
        self.locals
            .iter()
            .enumerate()
            .filter_map(|(i, l)| match l {
                Local::Named(n @ Named { name, depth, .. })
                    if *name == *symbol && *depth <= self.depth =>
                {
                    Some((i, n))
                }
                _ => None,
            })
            .max_by_key(|(_, l)| l.depth)
            .map(|(i, _)| i)
    }

    fn compile_block(&mut self, ignored: Vec<Expression>, result: Expression) -> Result<usize> {
        self.depth += 1;
        for expr in ignored {
            self.compile_expression(expr)?;
            self.clear_all_to_depth()?;
        }
        let last_exp_position = self.compile_expression(result)?;
        // Hoist the result up a position
        self.depth -= 1;
        Ok(last_exp_position)
    }

    // /// Hoist a local up a depth, moving a named into a temporary
    // /// If it was a named local, and captured - copy the value and close the upvalue
    // fn hoist_local(&mut self, local_position: usize) -> Result<usize> {
    //     match self.locals.get(local_position).cloned() {
    //         Some(Local::Named(Named {
    //             depth, captured, ..
    //         })) => {
    //             if captured {
    //                 let new_position = self.find_free_register()?;
    //                 self.chunk.opcodes.push(OpCode::CopyValue(
    //                     local_position.try_into()?,
    //                     new_position.try_into()?,
    //                 ));
    //                 self.chunk
    //                     .opcodes
    //                     .push(OpCode::CloseUpValue(local_position.try_into()?));
    //                 self.locals[local_position] = Local::None;
    //                 self.locals[new_position] = Local::Temporary(depth - 1);
    //                 Ok(new_position)
    //             } else {
    //                 self.locals[local_position] = Local::Temporary(depth - 1);
    //                 Ok(local_position)
    //             }
    //         }
    //         Some(Local::Temporary(depth)) => {
    //             self.locals[local_position] = Local::Temporary(depth - 1);
    //             Ok(local_position)
    //         }
    //         _ => Err(anyhow!("Tried to hoist a local that was none")),
    //     }
    // }

    fn compile_function_in_new_compiler(
        &mut self,
        args: Vec<Symbol>,
        body: Expression,
    ) -> Result<()> {
        for (i, s) in args.iter().enumerate() {
            self.locals[i] = Local::Named(Named {
                captured: false,
                depth: self.depth,
                name: (*s).clone(),
            });
        }
        self.depth += 1;
        let body_result_position = self.compile_expression(body)?;
        self.chunk
            .opcodes
            .push(OpCode::Return(body_result_position.try_into()?));
        // After the function returns, it's not really necessary to do these...
        // new_compiler.depth -= 1;
        // new_compiler.clear_locals_to_depth();
        Ok(())
    }

    fn compile_function(&mut self, args: Vec<Symbol>, body: Expression) -> Result<usize> {
        // probably ok lol?
        let parent_box = unsafe { Box::from_raw(self) };

        let arity = args.len();

        let mut new_compiler = Compiler::new(Some(parent_box));
        new_compiler.compile_function_in_new_compiler(args, body)?;

        self.chunk.functions.push(Rc::new(Function {
            arity,
            chunk: new_compiler.chunk,
            registers: N,
        }));
        let f_index = self.chunk.functions.len() - 1;
        let closure_index = self.find_free_register()?;
        self.locals[closure_index] = Local::Temporary;
        self.chunk.opcodes.push(OpCode::CreateClosure(
            f_index.try_into()?,
            closure_index.try_into()?,
        ));
        for u in new_compiler.upvalues {
            if u.local {
                self.chunk
                    .opcodes
                    .push(OpCode::CaptureUpValueFromLocal(u.position.try_into()?))
            } else {
                self.chunk
                    .opcodes
                    .push(OpCode::CaptureUpValueFromNonLocal(u.position.try_into()?))
            }
        }

        // get back the reference to parent, so it's not dropped
        Box::into_raw(new_compiler.previous.unwrap());
        Ok(closure_index)
    }

    fn compile_literal(&mut self, literal: Literal) -> Result<usize> {
        self.chunk.constants.push(Value::Integer(literal.0));
        let position = self.find_free_register()?;
        self.chunk.opcodes.push(OpCode::LoadConstant(
            (self.chunk.constants.len() - 1).try_into()?,
            position.try_into()?,
        ));
        self.locals[position] = Local::Temporary;
        Ok(position)
    }

    fn clear_local(&mut self, local_index: usize) -> Result<()> {
        match self
            .locals
            .get(local_index)
            .context("Could not replace local at index")?
        {
            Local::Named(Named { captured, .. }) => self.chunk.opcodes.push(if *captured {
                OpCode::CloseUpValue(local_index.try_into()?)
            } else {
                OpCode::CloseValue(local_index.try_into()?)
            }),
            Local::Temporary => self
                .chunk
                .opcodes
                .push(OpCode::CloseValue(local_index.try_into()?)),
            _ => (),
        }
        self.locals[local_index] = Local::None;

        Ok(())
    }

    /// Get the previous position of this symbol
    fn find_symbol_to_replace(&self, symbol: &Symbol, scope_depth: usize) -> Option<usize> {
        self.locals.iter().position(|l| {
            matches!(l, Local::Named(Named {name: n, depth: d, ..})
		     if n == symbol && *d == scope_depth)
        })
    }

    fn compile_assign(&mut self, symbol: Symbol, expression: Expression) -> Result<usize> {
        let assign_location = self.compile_expression(expression)?;
        // If the compiled expression is a temporary, then change it to named
        // If the compiled expression is named and it has the same name, then do nothing
        // If the compiled expression is named and it has a different name,
        // then copy the value to a new register, and give this new slot a name
        match self
            .locals
            .get(assign_location)
            .context("Could not get local")?
        {
            Local::Temporary => {
                self.locals[assign_location] = Local::Named(Named {
                    captured: false,
                    depth: self.depth,
                    name: symbol,
                });
                Ok(assign_location)
            }
            Local::Named(n) => {
                if n.name != symbol {
                    let new_register = self.find_free_register()?;
                    self.chunk.opcodes.push(OpCode::CopyValue(
                        assign_location.try_into()?,
                        new_register.try_into()?,
                    ));
                    self.locals[new_register] = Local::Named(Named {
                        name: symbol,
                        depth: self.depth,
                        captured: false,
                    });
                    Ok(new_register)
                } else {
                    if n.depth < self.depth {
                        return Err(anyhow!(
                            "Tried to assign to a child local with a depth less than current"
                        ));
                    }
                    self.locals[assign_location] = Local::Named(Named {
                        name: symbol,
                        depth: self.depth,
                        captured: n.captured,
                    });
                    Ok(assign_location)
                }
            }
            Local::None => Err(anyhow!(
                "There should not be none in the location of an expression"
            )),
        }
    }

    fn find_free_register(&self) -> Result<usize> {
        self.find_free_register_indices()
            .next()
            .context("Could not find free register")
    }

    /// Get an iterator over all local indices which have nothing in them
    fn find_free_register_indices<'a>(&'a self) -> impl Iterator<Item = usize> + 'a {
        self.locals.iter().enumerate().filter_map(|(i, x)| {
            if matches!(x, Local::None) {
                Some(i)
            } else {
                None
            }
        })
    }

    fn clear_all_to_depth(&mut self) -> Result<()> {
        let locations: Vec<_> = self
            .locals
            .iter()
            .enumerate()
            .filter_map(|(i, l)| match l {
                Local::Named(Named { depth, .. }) if *depth > self.depth => Some(i),
                Local::Temporary => Some(i),
                _ => None,
            })
            .collect();
        for index in locations {
            self.clear_local(index)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_symbol_to_replace() {
        let mut c: Compiler<200> = Compiler::new(None);
        let s = Symbol("Hi".into());
        c.locals[0] = Local::Named(Named {
            captured: true,
            depth: 2,
            name: s.clone(),
        });
        c.locals[2] = Local::Named(Named {
            captured: false,
            depth: 1,
            name: s.clone(),
        });
        c.locals[3] = Local::Named(Named {
            captured: false,
            depth: 3,
            name: Symbol("No hi".into()),
        });
        assert_eq!(Some(2), c.find_symbol_to_replace(&s, 1));
        assert_eq!(Some(0), c.find_symbol_to_replace(&s, 2));
        assert!(c.find_symbol_to_replace(&s, 0).is_none());
    }

    #[test]
    fn test_compiler_working() {}
}
