use std::rc::Rc;

use crate::{
    value::{Chunk, Function, Value},
    vm::OpCode,
};

#[derive(PartialEq, Clone)]
pub struct Symbol(String);

pub struct Literal(i64);

pub enum Expression {
    Call(Box<Expression>, Vec<Expression>),
    Assign(Symbol, Box<Expression>),
    Function(Vec<Symbol>, Box<Expression>),
    Block(Vec<Expression>, Box<Expression>),
    Literal(Literal),
    Symbol(Symbol),
}

#[derive(Clone)]
pub struct Named {
    pub name: Symbol,
    pub depth: usize,
    pub captured: bool,
}

#[derive(Clone)]
pub enum Local {
    Named(Named),
    Temporary,
}

#[derive(PartialEq)]
pub struct CUpValue {
    position: u8,
    local: bool,
}

pub struct Compiler {
    pub locals: Vec<Local>,
    pub upvalues: Vec<CUpValue>,
    pub depth: usize,
    pub previous: Option<Box<Compiler>>,
    pub chunk: Chunk,
}

impl Compiler {
    fn new(previous: Option<Box<Compiler>>) -> Self {
        Self {
            locals: vec![Local::Temporary; u8::MAX as usize],
            upvalues: vec![],
            depth: previous.map(|x| x.depth + 1).unwrap_or(0),
            previous,
            chunk: Chunk {
                constants: vec![],
                opcodes: vec![],
                functions: vec![],
            },
        }
    }

    /// Compile the given expression, putting the result in the register result_position
    pub fn compile(&mut self, expression: Expression, result_position: usize) {
        let result_position = result_position as u8;
        match expression {
            Expression::Call(function, arguments) => {
                self.call(*function, arguments, result_position as u8)
            }
            Expression::Assign(sym, e) => self.assign(sym, *e, result_position),
            Expression::Function(args, exp) => self.function(args, *exp, result_position),
            Expression::Block(ignored, result) => self.block(ignored, *result, result_position),
            Expression::Literal(lit) => self.literal(lit, result_position),
            Expression::Symbol(sym) => self.symbol(sym, result_position),
        };
    }

    fn call(&mut self, function: Expression, arguments: Vec<Expression>, result_position: u8) {
        let function_index = self.free_register_indices().next().unwrap();
        self.compile(function, function_index);
        self.chunk.opcodes.push(OpCode::Save(function_index as u8));
        for arg in arguments {
            let a_index = self.free_register_indices().next().unwrap();
            self.compile(arg, a_index);
            self.chunk.opcodes.push(OpCode::Save(a_index as u8));
        }
        self.chunk.opcodes.push(OpCode::Call(result_position))
    }

    fn symbol(&mut self, symbol: Symbol, result_position: u8) {
        let pos = self.get_symbol(symbol).unwrap();
        self.chunk
            .opcodes
            .push(OpCode::CopyValue(pos, result_position))
    }

    /// Get the value associated to the given symbol, and return the slot it is in
    /// If the value is an upvalue, emit an instruction to move the upvalue into a slot,
    /// then return the slot index
    fn get_symbol(&mut self, symbol: Symbol) -> Option<u8> {
        if let Some(i) = self.find_local_symbol(&symbol) {
            Some(i)
        } else if let Some(i) = self.find_nonlocal_symbol(&symbol) {
            let out_index = self.free_register_indices().next()?;
            self.chunk
                .opcodes
                .push(OpCode::LoadUpValue(i, out_index as u8));
            Some(out_index as u8)
        } else {
            None
        }
    }

    fn find_nonlocal_symbol(&mut self, symbol: &Symbol) -> Option<u8> {
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

    fn add_upvalue(&mut self, upvalue: CUpValue) -> u8 {
        self.upvalues
            .iter()
            .enumerate()
            .find_map(|(i, uv)| if *uv == upvalue { Some(i as u8) } else { None })
            .unwrap_or_else(|| {
                self.upvalues.push(upvalue);
                (self.upvalues.len() - 1) as u8
            })
    }

    fn find_local_symbol(&self, symbol: &Symbol) -> Option<u8> {
        self.locals
            .iter()
            .enumerate()
            .rev()
            .find_map(|(i, l)| match l {
                Local::Named(Named { name, .. }) => {
                    if *name == *symbol {
                        Some(i as u8)
                    } else {
                        None
                    }
                }
                Local::Temporary => None,
            })
    }

    fn block(&mut self, ignored: Vec<Expression>, result: Expression, result_position: u8) {
        self.depth += 1;
        for expr in ignored {
            let pos = self.free_register_indices().next().unwrap();
            self.compile(expr, pos);
        }
        self.compile(result, result_position as usize);
        self.chunk.opcodes.push(OpCode::Save(result_position));
        self.remove_locals_to_depth();
        self.depth -= 1;
        self.chunk.opcodes.push(OpCode::Dump(0, result_position))
    }

    fn remove_locals_to_depth(&mut self) {
        self.locals
            .iter_mut()
            .enumerate()
            .for_each(|(i, v)| match v {
                Local::Named(n) if n.depth >= self.depth => {
                    if n.captured {
                        self.chunk.opcodes.push(OpCode::CloseUpValue(i as u8))
                    } else {
                        self.chunk.opcodes.push(OpCode::CloseValue(i as u8))
                    };
                    *v = Local::Temporary;
                }
                _ => (),
            });
    }

    fn function(&mut self, args: Vec<Symbol>, body: Expression, result_position: u8) {
        let mut new_compiler = Compiler::new(Some(Box::new(*self)));
        for (i, s) in args.iter().enumerate() {
            new_compiler.locals[i] = Local::Named(Named {
                captured: false,
                depth: new_compiler.depth,
                name: *s,
            });
        }
        new_compiler.depth += 1;
        new_compiler.compile(body, 0);
        new_compiler.chunk.opcodes.push(OpCode::Return(0));
        new_compiler.depth -= 1;
        new_compiler.remove_locals_to_depth();
        self.chunk.functions.push(Rc::new(Function {
            chunk: new_compiler.chunk,
            registers: u8::MAX as usize,
        }));
        let f_index = self.chunk.functions.len() - 1;
        self.chunk
            .opcodes
            .push(OpCode::CreateClosure(f_index as u8, result_position));
        for u in new_compiler.upvalues {
            if u.local {
                self.chunk
                    .opcodes
                    .push(OpCode::CaptureUpValueFromLocal(u.position))
            } else {
                self.chunk
                    .opcodes
                    .push(OpCode::CaptureUpValueFromNonLocal(u.position))
            }
        }
    }

    fn literal(&mut self, literal: Literal, result_position: u8) {
        self.chunk.constants.push(Value::Integer(literal.0));
        self.chunk.opcodes.push(OpCode::LoadConstant(
            (self.chunk.constants.len() - 1) as u8,
            result_position,
        ));
        self.replace_local(result_position, Local::Temporary)
    }

    fn replace_local(&mut self, local_index: u8, new_local: Local) {
        match self.locals[local_index as usize] {
            Local::Named(Named { captured: true, .. }) => {
                self.chunk.opcodes.push(OpCode::CloseUpValue(local_index))
            }
            Local::Named(Named {
                captured: false, ..
            }) => self.chunk.opcodes.push(OpCode::CloseValue(local_index)),
            _ => (),
        }
        self.locals[local_index as usize] = new_local;
    }

    fn find_symbol_to_replace(&self, symbol: &Symbol, depth: usize) -> Option<(usize, &Local)> {
        self.locals.iter().enumerate().rev().find(|(_, l)| match l {
            Local::Named(Named { name, depth: d, .. }) => name == symbol && *d == depth,
            _ => false,
        })
    }

    fn assign(&mut self, symbol: Symbol, expression: Expression, result_position: u8) {
        let i = if let Some((i, l)) = self.find_symbol_to_replace(&symbol, self.depth) {
            // We have a matching symbol previously defined at the current depth.
            // If it is captured, we should close the upvalue

            i
        } else {
            self.free_register_indices().next().unwrap()
        };
        self.replace_local(
            i as u8,
            Local::Named(Named {
                captured: false,
                depth: self.depth,
                name: symbol,
            }),
        );

        self.compile(expression, i);
        self.chunk
            .opcodes
            .push(OpCode::CopyValue(i as u8, result_position))
    }

    fn free_register_indices<'a>(&'a self) -> impl Iterator<Item = usize> + 'a {
        self.locals.iter().enumerate().filter_map(|(i, x)| {
            if matches!(x, Local::Temporary) {
                Some(i)
            } else {
                None
            }
        })
    }
}
