use std::fmt::Debug;

use crate::tokeniser::Location;

#[derive(PartialEq, Clone, Debug, Hash, Eq)]
pub struct Symbol(pub String);

#[derive(Debug, Clone, PartialEq)]
pub enum Literal<'a> {
    Number(f64),
    Nil,
    Bool(bool),
    String(String),
    Quoted(Symbol),
    List(Vec<LocatedExpression<'a>>),
    Dictionary(Vec<(LocatedExpression<'a>, LocatedExpression<'a>)>),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Let<'a> {
    pub recursive: bool,
    pub pairs: Vec<(Symbol, LocatedExpression<'a>)>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Block<'a> {
    pub scope_introducing: bool,
    pub ignored: Vec<LocatedExpression<'a>>,
    pub last: Box<LocatedExpression<'a>>,
}

#[derive(Clone, PartialEq)]
pub struct LocatedExpression<'a> {
    pub expression: Expression<'a>,
    pub location: Location<'a>,
}

impl<'a> Debug for LocatedExpression<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("[{:?}, {:?}]", self.expression, self.location))
    }
}

#[derive(Clone, PartialEq)]
pub enum Expression<'a> {
    Condition(
        Vec<(LocatedExpression<'a>, LocatedExpression<'a>)>,
        Box<LocatedExpression<'a>>,
    ),
    Call(Box<LocatedExpression<'a>>, Vec<LocatedExpression<'a>>),
    Let(Let<'a>),
    Function(Vec<Symbol>, Box<LocatedExpression<'a>>),
    Block(Block<'a>),
    Literal(Literal<'a>),
    Symbol(Symbol),
}

impl<'a> From<&str> for Expression<'a> {
    fn from(value: &str) -> Self {
        Expression::Symbol(Symbol(value.into()))
    }
}

impl<'a> Expression<'a> {
    pub fn with_location(self, location: Location<'a>) -> LocatedExpression<'a> {
        LocatedExpression {
            expression: self,
            location,
        }
    }
}

impl<'a> Debug for Expression<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expression::Condition(arms, else_arm) => {
                f.write_fmt(format_args!("Cond {:?}, else {:?}", arms, else_arm))
            }
            Expression::Call(fun, args) => f.write_fmt(format_args!("Call({:?}, {:?})", fun, args)),
            Expression::Let(pairs) => f.write_fmt(format_args!("Let({:?})", pairs)),
            Expression::Function(args, res) => {
                f.write_fmt(format_args!("Fun({:?}) {:?}", args, res))
            }
            Expression::Block(block) => block.fmt(f),
            Expression::Literal(l) => l.fmt(f),
            Expression::Symbol(s) => s.fmt(f),
        }
    }
}
