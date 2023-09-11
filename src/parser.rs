use crate::{
    expression::{Expression, Literal, LocatedExpression, Symbol},
    tokeniser::{Location, Token, TokenData},
};

#[derive(Debug, PartialEq)]
pub enum ParseError {
    NoMoreTokens,
    UndefinedError,
}

type Result<Success> = std::result::Result<Success, ParseError>;

trait Take {
    type Output;
    type Check;
    fn take_matching(&self, item: Self::Check) -> Option<(&Self, &Self::Output)>;
}

impl<'a> Take for [Token<'a>] {
    type Output = Token<'a>;
    type Check = TokenData<'a>;

    fn take_matching(&self, item: Self::Check) -> Option<(&Self, &Self::Output)> {
        self.get(0).and_then(|i| {
            if std::mem::discriminant(&i.data) == std::mem::discriminant(&item) {
                Some((&self[1..], i))
            } else {
                None
            }
        })
    }
}

pub fn parse_expression<'a>(
    tokens: &'a [Token<'a>],
) -> Option<(&'a [Token<'a>], LocatedExpression<'a>)> {
    parse_list(tokens)
        .or_else(|| parse_symbol(tokens))
        .or_else(|| parse_function(tokens))
        .or_else(|| parse_number(tokens))
}

fn parse_function<'a>(tokens: &'a [Token<'a>]) -> Option<(&'a [Token<'a>], LocatedExpression<'a>)> {
    let mut t = tokens;
    (t, _) = t.take_matching(TokenData::Pipe)?;
    let mut arguments = vec![];
    while let Some((new_t, i)) = t.take_matching(TokenData::Symbol("")) {
        arguments.push(i);
        t = new_t
    }
    (t, _) = t.take_matching(TokenData::Pipe)?;

    parse_expression(t)
}

fn parse_symbol<'a>(tokens: &'a [Token<'a>]) -> Option<(&'a [Token<'a>], LocatedExpression<'a>)> {
    let (t, s) = tokens.take_matching(TokenData::Symbol(""))?;
    match s.data {
        TokenData::Symbol(sym) => Some((
            t,
            LocatedExpression {
                expression: Expression::Symbol(Symbol(sym.into())),
                location: s.location.clone(),
            },
        )),
        _ => None,
    }
}

fn parse_number<'a>(tokens: &'a [Token<'a>]) -> Option<(&'a [Token<'a>], LocatedExpression<'a>)> {
    let (t, s) = tokens.take_matching(TokenData::Number(""))?;
    match s.data {
        TokenData::Number(num) => Some((
            t,
            LocatedExpression {
                expression: Expression::Literal(Literal::Number(num.parse().unwrap())),
                location: s.location.clone(),
            },
        )),
        _ => None,
    }
}

fn parse_list<'a>(tokens: &'a [Token<'a>]) -> Option<(&'a [Token<'a>], LocatedExpression<'a>)> {
    let mut t = tokens;
    let (new_t, open_b) = t.take_matching(TokenData::OpenSquareBracket)?;

    t = new_t;
    let mut elements = vec![];

    while let Some((new_t, e)) = parse_expression(t) {
        elements.push(e);
        t = new_t;
        if let Some((new_t, _)) = t.take_matching(TokenData::Comma) {
            t = new_t
        } else {
            break;
        }
    }
    let (t, close_b) = t.take_matching(TokenData::CloseSquareBracket)?;
    Some((
        t,
        LocatedExpression {
            expression: Expression::Literal(Literal::List(elements)),
            location: Location::between(&open_b.location, &close_b.location),
        },
    ))
}

fn dict_element<'a>(
    tokens: &'a [Token<'a>],
) -> Option<(
    &'a [Token<'a>],
    (LocatedExpression<'a>, LocatedExpression<'a>),
)> {
    let (t, k) = parse_expression(tokens)?;
    let (t, _) = t.take_matching(TokenData::Colon)?;
    let (t, v) = parse_expression(t)?;
    Some((t, (k, v)))
}

fn parse_dict<'a>(tokens: &'a [Token<'a>]) -> Option<(&'a [Token<'a>], LocatedExpression<'a>)> {
    let mut t = tokens;
    let (new_t, open_b) = t.take_matching(TokenData::OpenAngleBracket)?;
    t = new_t;
    let mut elements = vec![];
    while let Some((new_t, kv)) = dict_element(t) {
        elements.push(kv);
        t = new_t;
        if let Some((new_t, _)) = t.take_matching(TokenData::Comma) {
            t = new_t
        } else {
            break;
        }
    }
    let (t, close_b) = t.take_matching(TokenData::CloseAngleBracket)?;
    Some((
        t,
        LocatedExpression {
            expression: Expression::Literal(Literal::Dictionary(elements)),
            location: Location::between(&open_b.location, &close_b.location),
        },
    ))
}

fn parse_string<'a>(tokens: &'a [Token<'a>]) -> Option<(&'a [Token<'a>], LocatedExpression<'a>)> {
    tokens.get(0).and_then(|t| match t.data {
        TokenData::String(s) => Some((
            &tokens[1..],
            LocatedExpression {
                expression: Expression::Literal(Literal::String(s.into())),
                location: t.location.clone(),
            },
        )),
        _ => None,
    })
}

fn parse_literal<'a>(tokens: &'a [Token<'a>]) -> Option<(&'a [Token<'a>], LocatedExpression<'a>)> {
    let f = (|| {
        for (token_data, literal) in [
            (TokenData::Nil, Literal::Nil),
            (TokenData::False, Literal::Bool(false)),
            (TokenData::True, Literal::Bool(true)),
        ] {
            if let Some((t, tok)) = tokens.take_matching(token_data) {
                return Some((
                    t,
                    LocatedExpression {
                        expression: Expression::Literal(literal),
                        location: tok.location.clone(),
                    },
                ));
            }
        }
        None
    });
    f().or_else(|| parse_list(tokens))
        .or_else(|| parse_dict(tokens))
        .or_else(|| parse_number(tokens))
        .or_else(|| parse_string(tokens))
        .or_else(|| parse_quoted_symbol(tokens))
}

fn parse_quoted_symbol<'a>(
    tokens: &'a [Token<'a>],
) -> Option<(&'a [Token<'a>], LocatedExpression<'a>)> {
    let (t, d) = tokens.take_matching(TokenData::Dollar)?;
    let (t, s) = parse_symbol(t)?;
    match s.expression {
        Expression::Symbol(sym) => Some((
            t,
            LocatedExpression {
                expression: Expression::Literal(Literal::Quoted(sym)),
                location: Location::between(&d.location, &s.location),
            },
        )),
        _ => None,
    }
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
            Some(&TokenData::Symbol("a"))
        );
        assert_eq!(tokens.take_matching(TokenData::OpenSquareBracket), None);
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
            Some(LocatedExpression {
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
            Some(LocatedExpression {
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
