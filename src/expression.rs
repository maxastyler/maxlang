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

#[derive(Debug, Clone, PartialEq)]
pub struct LocatedExpression<'a> {
    pub expression: Expression<'a>,
    pub location: Location<'a>,
}

#[derive(Debug, Clone, PartialEq)]
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
