use crate::{
    value::{Chunk, Value},
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
pub enum Local {
    Named {
        name: Symbol,
        depth: usize,
        captured: bool,
    },
    Temporary,
}

pub enum CUpValue {
    ToRemote(u8),
    ToLocal(u8),
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
            depth: 0,
            previous,
            chunk: Chunk {
                constants: vec![],
                opcodes: vec![],
            },
        }
    }

    /// Compile the given expression, putting the result in the register result_position
    pub fn compile(&mut self, expression: Expression, result_position: usize) {
        let result_position = result_position as u8;
        match expression {
            Expression::Call(_, _) => todo!(),
            Expression::Assign(sym, e) => self.assign(sym, *e, result_position),
            Expression::Function(_, _) => todo!(),
            Expression::Block(ignored, result) => self.block(ignored, *result, result_position),
            Expression::Literal(lit) => self.literal(lit, result_position),
            Expression::Symbol(sym) => self.symbol(sym, result_position),
        };
    }
    fn symbol(&mut self, symbol: Symbol, result_position: u8) {}

    fn find_nonlocal_symbol(&mut self, symbol: &Symbol) -> Option<u8> {
        if let Some(p) = self.previous {
            if let Some(l) = p.find_local_symbol(p) {}
        }
    }

    fn find_local_symbol(&self, symbol: &Symbol) -> Option<u8> {
        self.locals.iter().enumerate().find_map(|(i, l)| match *l {
            Local::Named { name, .. } => {
                if name == *symbol {
                    Some(i as u8)
                } else {
                    None
                }
            }
            Local::Temporary => None,
        })
    }

    fn block(&mut self, ignored: Vec<Expression>, result: Expression, result_position: u8) {
        for expr in ignored {
            let pos = self.free_register_indices().next().unwrap();
            self.compile(expr, pos);
        }
        self.compile(result, result_position as usize);
    }

    fn function(&mut self, args: Vec<Symbol>, body: Expression, result_position: u8) {}

    fn literal(&mut self, literal: Literal, result_position: u8) {
        self.chunk.constants.push(Value::Integer(literal.0));
        self.chunk.opcodes.push(OpCode::LoadConstant(
            (self.chunk.constants.len() - 1) as u8,
            result_position,
        ))
    }

    fn find_symbol_to_replace(&self, symbol: &Symbol, depth: usize) -> Option<(usize, &Local)> {
        self.locals.iter().enumerate().rev().find(|(_, l)| match l {
            Local::Named { name, depth: d, .. } => name == symbol && *d == depth,
            _ => false,
        })
    }

    fn assign(&mut self, symbol: Symbol, expression: Expression, result_position: u8) {
        let i = if let Some((i, l)) = self.find_symbol_to_replace(&symbol, self.depth) {
            // We have a matching symbol previously defined at the current depth.
            // If it is captured, we should create the upvalue and dispose of it
            match *l {
                Local::Named { captured: true, .. } => {
                    self.chunk.opcodes.push(OpCode::CloseUpValue(i as u8));
                    self.upvalues.retain(|x| match x {
                        CUpValue::ToRemote(i) => todo!(),
                        CUpValue::ToLocal(i) => todo!(),
                    } != i as u8);
                }
                _ => {}
            };
            i
        } else {
            let index = self.free_register_indices().next().unwrap();
            self.locals[index] = Local::Named {
                captured: false,
                depth: self.depth,
                name: symbol,
            };
            index
        };
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
