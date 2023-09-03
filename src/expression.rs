#[derive(PartialEq, Clone, Debug, Hash, Eq)]
pub struct Symbol(pub String);

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Double(f64),
    Nil,
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Condition(Vec<(Expression, Expression)>, Box<Expression>),
    Call(Box<Expression>, Vec<Expression>),
    Assign(Symbol, Box<Expression>),
    Function(Vec<Symbol>, Box<Expression>),
    Block(Vec<Expression>, Box<Expression>),
    Literal(Literal),
    Symbol(Symbol),
}

impl From<Symbol> for Expression {
    fn from(value: Symbol) -> Self {
        Expression::Symbol(value)
    }
}

impl From<i64> for Expression {
    fn from(value: i64) -> Self {
        Expression::Literal(Literal::Int(value))
    }
}

impl From<bool> for Expression {
    fn from(value: bool) -> Self {
        Expression::Literal(Literal::Bool(value))
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
