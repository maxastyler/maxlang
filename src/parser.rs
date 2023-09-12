use nom::CompareResult;

use crate::{
    expression::{Block, Expression, Let, Literal, LocatedExpression, Symbol},
    tokeniser::{Location, Token, TokenData},
};

#[derive(Debug, PartialEq)]
pub enum ParseErrorType<'a> {
    NoMoreTokens,
    TokenDoesntMatch(TokenData<'a>),
    CouldNotMatchSymbol,
    CouldNotMatchNumber,
    CouldNotMatchString,
    IncompleteInfix,
    NoArgumentsToCall,
}

impl<'a> ParseErrorType<'a> {
    fn with_location(self, location: Location<'a>) -> ParseError<'a> {
        ParseError {
            error_type: self,
            location: Some(location),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct ParseError<'a> {
    error_type: ParseErrorType<'a>,
    location: Option<Location<'a>>,
}

impl<'a> From<ParseErrorType<'a>> for ParseError<'a> {
    fn from(value: ParseErrorType<'a>) -> Self {
        ParseError {
            error_type: value,
            location: None,
        }
    }
}

type Result<'a, Success> = std::result::Result<Success, ParseError<'a>>;

trait Take {
    type Output;
    type Check;
    fn take_matching(&self, item: Self::Check) -> Result<(&Self, &Self::Output)>;
}

impl<'a> Take for [Token<'a>] {
    type Output = Token<'a>;
    type Check = TokenData<'a>;

    fn take_matching(&self, item: Self::Check) -> Result<(&Self, &Self::Output)> {
        self.get(0)
            .map(|i| {
                if std::mem::discriminant(&i.data) == std::mem::discriminant(&item) {
                    Ok((&self[1..], i))
                } else {
                    Err(ParseErrorType::TokenDoesntMatch(item).with_location(i.location.clone()))
                }
            })
            .unwrap_or(Err(ParseErrorType::NoMoreTokens.into()))
    }
}

pub fn parse_no_arg_call<'a>(
    tokens: &'a [Token<'a>],
) -> Result<(&'a [Token<'a>], LocatedExpression<'a>)> {
    let (t, e) = parse_non_left_recursive_expression(tokens)?;
    let (t, f) = t.take_matching(TokenData::ExclamationMark)?;
    Ok((
        t,
        LocatedExpression {
            expression: Expression::Call(Box::new(e.clone()), vec![]),
            location: Location::between(&e.location, &f.location),
        },
    ))
}

pub fn infix_call_inner<'a>(
    tokens: &'a [Token<'a>],
) -> Result<(
    &'a [Token<'a>],
    (LocatedExpression<'a>, Vec<LocatedExpression<'a>>),
)> {
    let mut t;
    let mut first_call;
    t = tokens;
    let (new_t, first) = t.take_matching(TokenData::Apostrophe)?;
    t = new_t;
    (t, first_call) = parse_left_recursive_expression_1(t)?;
    let mut args = vec![];
    while let Ok((new_t, e)) = parse_left_recursive_expression_1(t) {
        t = new_t;
        args.push(e);
    }
    first_call.location = Location::between(&first.location, &first_call.location);
    Ok((t, (first_call, args)))
}

pub fn parse_infix_call<'a>(
    tokens: &'a [Token<'a>],
) -> Result<(&'a [Token<'a>], LocatedExpression<'a>)> {
    let (t, first) = parse_left_recursive_expression_1(tokens)?;
    let mut t = t;
    let mut rest = vec![];
    while let Ok((new_t, e)) = infix_call_inner(t) {
        t = new_t;
        rest.push(e);
    }
    if rest.len() == 0 {
        Err(ParseErrorType::IncompleteInfix.with_location(first.location))
    } else {
        Ok((
            t,
            rest.into_iter().fold(first, |a, (func, other_args)| {
                let location =
                    Location::between(&a.location, &other_args.last().unwrap_or(&func).location);
                let mut args = vec![a];
                args.extend(other_args.into_iter());

                LocatedExpression {
                    expression: Expression::Call(Box::new(func), args),
                    location,
                }
            }),
        ))
    }
}

pub fn parse_normal_call<'a>(
    tokens: &'a [Token<'a>],
) -> Result<(&'a [Token<'a>], LocatedExpression<'a>)> {
    let (t, func) = parse_left_recursive_expression_2(tokens)?;
    let mut args = vec![];
    let mut t = t;
    while let Ok((new_t, e)) = parse_left_recursive_expression_2(t) {
        args.push(e);
        t = new_t;
    }

    args.last()
        .map(|last| {
            let location = Location::between(&func.location, &last.location);
            (
                t,
                LocatedExpression {
                    expression: Expression::Call(Box::new(func.clone()), args.clone()),
                    location,
                },
            )
        })
        .ok_or(ParseErrorType::NoArgumentsToCall.with_location(func.location))
}

pub fn parse_non_left_recursive_expression<'a>(
    tokens: &'a [Token<'a>],
) -> Result<(&'a [Token<'a>], LocatedExpression<'a>)> {
    parse_literal(tokens)
        .or_else(|_| parse_symbol(tokens))
        .or_else(|_| parse_function(tokens))
        .or_else(|_| parse_cond_block(tokens))
        .or_else(|_| parse_scoped_block(tokens))
        .or_else(|_| parse_unscoped_block(tokens))
        .or_else(|_| parse_assignment(tokens))
}

pub fn parse_left_recursive_expression_1<'a>(
    tokens: &'a [Token<'a>],
) -> Result<(&'a [Token<'a>], LocatedExpression<'a>)> {
    parse_no_arg_call(tokens).or_else(|_| parse_non_left_recursive_expression(tokens))
}

pub fn parse_left_recursive_expression_2<'a>(
    tokens: &'a [Token<'a>],
) -> Result<(&'a [Token<'a>], LocatedExpression<'a>)> {
    parse_infix_call(tokens).or_else(|_| parse_left_recursive_expression_1(tokens))
}

pub fn parse_expression<'a>(
    tokens: &'a [Token<'a>],
) -> Result<(&'a [Token<'a>], LocatedExpression<'a>)> {
    parse_normal_call(tokens).or_else(|_| parse_left_recursive_expression_2(tokens))
}

fn parse_assignment_pair<'a>(
    tokens: &'a [Token<'a>],
) -> Result<(&'a [Token<'a>], (Symbol, LocatedExpression<'a>))> {
    let (t, symbol) = tokens.take_matching(TokenData::Symbol(""))?;
    let (t, exp) = parse_expression(t)?;
    match symbol.data {
        TokenData::Symbol(sym) => Ok((t, (Symbol(sym.into()), exp))),
        _ => Err(ParseErrorType::CouldNotMatchSymbol.with_location(symbol.location.clone())),
    }
}

fn parse_assignment<'a>(
    tokens: &'a [Token<'a>],
) -> Result<(&'a [Token<'a>], LocatedExpression<'a>)> {
    let mut t;
    let (new_t, start, recursive) = tokens
        .take_matching(TokenData::Let)
        .map(|(t, token)| (t, token, false))
        .or_else(|_| {
            tokens
                .take_matching(TokenData::LetRec)
                .map(|(t, token)| (t, token, true))
        })?;
    t = new_t;
    let mut pairs = vec![];

    while let Ok((new_t, e)) = parse_assignment_pair(t) {
        pairs.push(e);
        t = new_t;
        if let Ok((new_t, _)) = t.take_matching(TokenData::Comma) {
            t = new_t
        } else {
            break;
        }
    }
    let location = Location::between(
        &start.location,
        pairs.last().map_or(&start.location, |(_, e)| &e.location),
    );
    Ok((
        t,
        LocatedExpression {
            expression: Expression::Let(Let { recursive, pairs }),
            location,
        },
    ))
}

fn parse_function<'a>(tokens: &'a [Token<'a>]) -> Result<(&'a [Token<'a>], LocatedExpression<'a>)> {
    let mut t = tokens;
    let (new_t, start) = t.take_matching(TokenData::Pipe)?;
    t = new_t;
    let mut arguments = vec![];
    while let Ok((new_t, i)) = t.take_matching(TokenData::Symbol("")) {
        arguments.push(match i.data {
            TokenData::Symbol(s) => Ok(Symbol(s.into())),
            _ => Err(ParseErrorType::CouldNotMatchSymbol.with_location(i.location.clone())),
        }?);
        t = new_t
    }
    (t, _) = t.take_matching(TokenData::Pipe)?;

    let (new_t, body) = parse_expression(t)?;
    let location = Location::between(&start.location, &body.location);
    Ok((
        new_t,
        LocatedExpression {
            expression: Expression::Function(arguments, Box::new(body)),
            location,
        },
    ))
}

fn parse_symbol<'a>(tokens: &'a [Token<'a>]) -> Result<(&'a [Token<'a>], LocatedExpression<'a>)> {
    let (t, s) = tokens.take_matching(TokenData::Symbol(""))?;
    match s.data {
        TokenData::Symbol(sym) => Ok((
            t,
            LocatedExpression {
                expression: Expression::Symbol(Symbol(sym.into())),
                location: s.location.clone(),
            },
        )),
        _ => Err(ParseErrorType::CouldNotMatchSymbol.with_location(s.location.clone())),
    }
}

fn parse_number<'a>(tokens: &'a [Token<'a>]) -> Result<(&'a [Token<'a>], LocatedExpression<'a>)> {
    let (t, s) = tokens.take_matching(TokenData::Number(""))?;
    match s.data {
        TokenData::Number(num) => Ok((
            t,
            LocatedExpression {
                expression: Expression::Literal(Literal::Number(num.parse().unwrap())),
                location: s.location.clone(),
            },
        )),
        _ => Err(ParseErrorType::CouldNotMatchNumber.with_location(s.location.clone())),
    }
}

fn parse_list<'a>(tokens: &'a [Token<'a>]) -> Result<(&'a [Token<'a>], LocatedExpression<'a>)> {
    let mut t = tokens;
    let (new_t, open_b) = t.take_matching(TokenData::OpenSquareBracket)?;

    t = new_t;
    let mut elements = vec![];

    while let Ok((new_t, e)) = parse_expression(t) {
        elements.push(e);
        t = new_t;
        if let Ok((new_t, _)) = t.take_matching(TokenData::Comma) {
            t = new_t
        } else {
            break;
        }
    }
    let (t, close_b) = t.take_matching(TokenData::CloseSquareBracket)?;
    Ok((
        t,
        LocatedExpression {
            expression: Expression::Literal(Literal::List(elements)),
            location: Location::between(&open_b.location, &close_b.location),
        },
    ))
}

fn dict_element<'a>(
    tokens: &'a [Token<'a>],
) -> Result<(
    &'a [Token<'a>],
    (LocatedExpression<'a>, LocatedExpression<'a>),
)> {
    let (t, k) = parse_expression(tokens)?;
    let (t, _) = t.take_matching(TokenData::Colon)?;
    let (t, v) = parse_expression(t)?;
    Ok((t, (k, v)))
}

fn parse_dict<'a>(tokens: &'a [Token<'a>]) -> Result<(&'a [Token<'a>], LocatedExpression<'a>)> {
    let mut t = tokens;
    let (new_t, open_b) = t.take_matching(TokenData::OpenAngleBracket)?;
    t = new_t;
    let mut elements = vec![];
    while let Ok((new_t, kv)) = dict_element(t) {
        elements.push(kv);
        t = new_t;
        if let Ok((new_t, _)) = t.take_matching(TokenData::Comma) {
            t = new_t
        } else {
            break;
        }
    }
    let (t, close_b) = t.take_matching(TokenData::CloseAngleBracket)?;
    Ok((
        t,
        Expression::Literal(Literal::Dictionary(elements))
            .with_location(Location::between(&open_b.location, &close_b.location)),
    ))
}

fn parse_string<'a>(tokens: &'a [Token<'a>]) -> Result<(&'a [Token<'a>], LocatedExpression<'a>)> {
    tokens
        .get(0)
        .map(|t| match t.data {
            TokenData::String(s) => Ok((
                &tokens[1..],
                Expression::Literal(Literal::String(s.into())).with_location(t.location.clone()),
            )),
            _ => Err(ParseErrorType::CouldNotMatchString.with_location(t.location.clone())),
        })
        .unwrap_or(Err(ParseErrorType::NoMoreTokens.into()))
}

fn parse_literal<'a>(tokens: &'a [Token<'a>]) -> Result<(&'a [Token<'a>], LocatedExpression<'a>)> {
    let f = || {
        for (token_data, literal) in [
            (TokenData::Nil, Literal::Nil),
            (TokenData::False, Literal::Bool(false)),
            (TokenData::True, Literal::Bool(true)),
        ] {
            if let Ok((t, tok)) = tokens.take_matching(token_data) {
                return Ok((
                    t,
                    LocatedExpression {
                        expression: Expression::Literal(literal),
                        location: tok.location.clone(),
                    },
                ));
            }
        }
        Err(ParseErrorType::CouldNotMatchSymbol.into())
    };
    f().or_else(|_: ParseError| parse_list(tokens))
        .or_else(|_| parse_dict(tokens))
        .or_else(|_| parse_number(tokens))
        .or_else(|_| parse_string(tokens))
        .or_else(|_| parse_quoted_symbol(tokens))
}

fn parse_quoted_symbol<'a>(
    tokens: &'a [Token<'a>],
) -> Result<(&'a [Token<'a>], LocatedExpression<'a>)> {
    let (t, d) = tokens.take_matching(TokenData::Dollar)?;
    let (t, s) = parse_symbol(t)?;
    let location = Location::between(&d.location, &s.location);
    match s.expression {
        Expression::Symbol(sym) => Ok((
            t,
            Expression::Literal(Literal::Quoted(sym)).with_location(location),
        )),
        _ => Err(ParseErrorType::CouldNotMatchSymbol.with_location(location)),
    }
}

fn parse_block<'a>(
    tokens: &'a [Token<'a>],
    (delim_1, delim_2): (TokenData<'a>, TokenData<'a>),
    scope_introducing: bool,
) -> Result<'a, (&'a [Token<'a>], LocatedExpression<'a>)> {
    let mut t = tokens;
    let (new_t, open_b) = t.take_matching(delim_1)?;
    t = new_t;
    let mut elements = vec![];
    while let Ok((new_t, e)) = parse_expression(t) {
        elements.push(e);
        t = new_t;
        if let Ok((new_t, _)) = t.take_matching(TokenData::Comma) {
            t = new_t
        } else {
            break;
        }
    }
    let (t, close_b) = t.take_matching(delim_2)?;
    let location = Location::between(&open_b.location, &close_b.location);
    let block = if let Some((last, rest)) = elements.split_last() {
        Block {
            scope_introducing,

            ignored: rest.iter().map(|x| x.clone()).collect(),
            last: Box::new(last.clone()),
        }
    } else {
        Block {
            scope_introducing,
            ignored: vec![],
            last: Box::new(Expression::Literal(Literal::Nil).with_location(location.clone())),
        }
    };

    Ok((t, Expression::Block(block).with_location(location)))
}

fn parse_scoped_block<'a>(
    tokens: &'a [Token<'a>],
) -> Result<(&'a [Token<'a>], LocatedExpression<'a>)> {
    parse_block(
        tokens,
        (TokenData::OpenCurlyBracket, TokenData::CloseCurlyBracket),
        true,
    )
}

fn parse_unscoped_block<'a>(
    tokens: &'a [Token<'a>],
) -> Result<(&'a [Token<'a>], LocatedExpression<'a>)> {
    parse_block(tokens, (TokenData::OpenParen, TokenData::CloseParen), false)
}

fn parse_condition<'a>(
    tokens: &'a [Token<'a>],
) -> Result<(
    &'a [Token<'a>],
    (LocatedExpression<'a>, LocatedExpression<'a>),
)> {
    let (t, c) = parse_expression(tokens)?;
    let (t, _) = t.take_matching(TokenData::Tilde)?;
    let (t, r) = parse_expression(t)?;
    let (t, _) = t.take_matching(TokenData::Comma)?;
    Ok((t, (c, r)))
}

fn parse_cond_block<'a>(
    tokens: &'a [Token<'a>],
) -> Result<(&'a [Token<'a>], LocatedExpression<'a>)> {
    let mut t = tokens;
    let (new_t, start) = t.take_matching(TokenData::Cond)?;
    t = new_t;
    (t, _) = t.take_matching(TokenData::OpenCurlyBracket)?;
    let mut conditions = vec![];
    while let Ok((new_t, c)) = parse_condition(t) {
        t = new_t;
        conditions.push(c);
    }
    let (t, _) = t.take_matching(TokenData::Else)?;
    let (t, else_exp) = parse_expression(t)?;
    let (t, close) = t.take_matching(TokenData::CloseCurlyBracket)?;
    Ok((
        t,
        LocatedExpression {
            expression: Expression::Condition(conditions, Box::new(else_exp)),
            location: Location::between(&start.location, &close.location),
        },
    ))
}

#[cfg(test)]
mod test {
    use crate::expression::{Expression, Literal, LocatedExpression, Symbol};
    use crate::parser::{parse_list, Take};
    use crate::tokeniser::{Location, Token, TokenData};

    use super::parse_symbol;

    #[test]
    fn take_works() {
        let tokens = Token::tokenise_source("a|a| a", "")
            .map(|i| i.unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            tokens
                .take_matching(TokenData::Symbol(""))
                .map(|x| &x.1.data),
            Ok(&TokenData::Symbol("a"))
        );
        assert!(tokens.take_matching(TokenData::OpenSquareBracket).is_err());
    }

    #[test]
    fn parse_symbol_works() {
        assert_eq!(
            parse_symbol(
                &Token::tokenise_source("sym`", "")
                    .map(|i| i.unwrap())
                    .collect::<Vec<_>>(),
            )
            .map(|x| x.1),
            Ok(LocatedExpression {
                expression: Expression::Symbol(Symbol("sym".into())),
                location: Location {
                    file: "",
                    source: "sym`",
                    start_pos: 0,
                    end_pos: 3
                }
            })
        );
    }

    #[test]
    fn parse_list_works() {
        let source = "[a, b, c]";
        assert_eq!(
            parse_list(
                &Token::tokenise_source(source, "")
                    .map(|i| i.unwrap())
                    .collect::<Vec<_>>()
            )
            .map(|x| x.1),
            Ok(LocatedExpression {
                expression: Expression::Literal(Literal::List(vec![])),
                location: Location {
                    file: "",
                    source,
                    start_pos: 0,
                    end_pos: 3
                }
            })
        )
    }
}
