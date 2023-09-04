use nom::{
    bytes::complete::{tag, take_while, take_while1},
    character::complete::{multispace0, multispace1},
    combinator::opt,
    error::ParseError,
    multi::{many0, many1, separated_list0, separated_list1},
    number::complete::double,
    sequence::{delimited, tuple},
    AsChar, IResult, InputLength, InputTakeAtPosition, Parser,
};

use crate::expression::{Expression, Literal, Symbol};

const RESERVED_STRINGS: &[&str] = &["=>", "let", "cond", "fn"];

const RESERVED_CHARS: &[char] = &['(', ')', '{', '}', '`', ';', ',', '!', '"'];

fn nil(input: &str) -> IResult<&str, Literal> {
    let (s, _) = tag("nil")(input)?;
    Ok((s, Literal::Nil))
}

fn false_lit(input: &str) -> IResult<&str, Literal> {
    let (s, _) = tag("false")(input)?;
    Ok((s, Literal::Bool(false)))
}

fn true_lit(input: &str) -> IResult<&str, Literal> {
    let (s, _) = tag("true")(input)?;
    Ok((s, Literal::Bool(true)))
}

fn number(input: &str) -> IResult<&str, Literal> {
    let (s, n) = double(input)?;
    Ok((s, Literal::Double(n)))
}

fn symbol(input: &str) -> IResult<&str, Symbol> {
    let (s, first) = take_while1(|x: char| {
        !(x.is_whitespace() | x.is_digit(10) | RESERVED_CHARS.iter().any(|&c| c == x))
    })(input)?;
    let (s, rest) =
        take_while(|x: char| !(x.is_whitespace() | RESERVED_CHARS.iter().any(|&c| c == x)))(s)?;
    let full_string = format!("{first}{rest}");
    if RESERVED_STRINGS.iter().any(|&x| x == full_string) {
        Err(nom::Err::Error(nom::error::Error::from_error_kind(
            input,
            nom::error::ErrorKind::Char,
        )))
    } else {
        Ok((s, Symbol(full_string)))
    }
}

fn surrounded_list<I, O, O1, O2, O3, E, F, G, H, J>(
    mut first: F,
    mut separator: G,
    mut value: H,
    mut last: J,
) -> impl FnMut(I) -> IResult<I, Vec<O>, E>
where
    I: InputTakeAtPosition + Clone + InputLength,
    <I as InputTakeAtPosition>::Item: AsChar + Clone,
    H: Parser<I, O, E>,
    F: Parser<I, O1, E>,
    G: Parser<I, O2, E>,
    J: Parser<I, O3, E>,
    E: ParseError<I>,
{
    move |s| {
        let (s, _) = first.parse(s)?;
        let (s, _) = multispace0(s)?;
        let (s, values) = separated_list0(
            many1(delimited(multispace0, |x| separator.parse(x), multispace0)),
            |x| value.parse(x),
        )(s)?;
        let (s, _) = multispace0(s)?;
        let (s, _) = last.parse(s)?;
        Ok((s, values))
    }
}

/// Delimite the given parser by spaces
fn s_d<I, O, E, F>(p: F) -> impl FnMut(I) -> IResult<I, O, E>
where
    I: InputTakeAtPosition + Clone + InputLength,
    <I as InputTakeAtPosition>::Item: AsChar + Clone,
    F: Parser<I, O, E>,
    E: ParseError<I>,
{
    delimited(multispace0, p, multispace0)
}

/// A function that returns a list separated and possibly delimited by the separator.
/// The separator has to occur at least once, and can occur multiple times
/// Returns the vector of values in the list, and also a boolean which is true if the
/// function ended with the separator
fn separated_delimited_list0<I, O, O2, E, F, G>(
    mut sep: G,
    mut f: F,
) -> impl FnMut(I) -> IResult<I, (Vec<O>, bool), E>
where
    I: Clone + InputLength,
    F: Parser<I, O, E>,
    G: Parser<I, O2, E>,
    E: ParseError<I>,
{
    move |s| {
        let (s, _) = opt(many1(|x| sep.parse(x)))(s)?;
        let (s, items) = separated_list0(many1(|x| sep.parse(x)), |x| f.parse(x))(s)?;
        let (s, last) = opt(many1(|x| sep.parse(x)))(s)?;
        Ok((s, (items, last.is_some())))
    }
}

fn function(input: &str) -> IResult<&str, Expression> {
    let (s, _) = tag("fn")(input)?;
    let (s, fn_symbol) = s_d(opt(symbol))(s)?;
    let (s, args) = opt(delimited(
        tag("("),
        separated_delimited_list0(s_d(tag(",")), symbol),
        tag(")"),
    ))(s)?;
    let (s, _) = multispace0(s)?;
    let (s, b) = scope_introducing_block(s)?;
    let f = Expression::Function(args.map(|(a, _)| a).unwrap_or_else(Vec::new), Box::new(b));
    Ok((
        s,
        match fn_symbol {
            Some(symbol) => Expression::Assign(symbol, Box::new(f)),
            None => f,
        },
    ))
}

fn literal(input: &str) -> IResult<&str, Literal> {
    nil.or(number).or(true_lit).or(false_lit).parse(input)
}

fn block_element(input: &str) -> IResult<&str, (Expression, bool)> {
    let (s, e) = expression(input)?;
    let (s, _) = multispace0(s)?;
    let (s, semi) = opt(tag(";"))(s)?;
    Ok((s, (e, semi.is_some())))
}

fn block(input: &str) -> IResult<&str, (Vec<Expression>, Option<Expression>)> {
    let (s, es) = many0(s_d(block_element))(input)?;
    let es_len = es.len();
    let el = es.last().map(|x| x.clone());
    if let Some((e, semi)) = el {
        if semi {
            Ok((s, (es.into_iter().map(move |(e, _)| e).collect(), None)))
        } else {
            Ok((
                s,
                (
                    es.into_iter()
                        .take(es_len - 1)
                        .map(move |(e, _)| e)
                        .collect(),
                    Some(e),
                ),
            ))
        }
    } else {
        Ok((s, (vec![], None)))
    }
}

fn scope_introducing_block(input: &str) -> IResult<&str, Expression> {
    let (s, _) = tag("{")(input)?;
    let (s, (ignored_exps, last_exp)) = s_d(block)(s)?;
    let (s, _) = tag("}")(s)?;
    Ok((
        s,
        Expression::Block(
            ignored_exps,
            Box::new(last_exp.unwrap_or(Expression::Literal(Literal::Nil))),
        ),
    ))
}

// fn non_scope_introducing_block(input: &str) -> IResult<&str, Block> {
//     let (s, _) = tag("(")(input)?;
//     let (s, (ignored_exps, last_exp)) = s_d(block)(s)?;
//     let (s, _) = tag(")")(s)?;
//     Ok((
//         s,
//         Block {
//             ignored_expressions: ignored_exps,
//             last_expression: if let Some(e) = last_exp {
//                 e
//             } else {
//                 Expression::Literal(Literal::Nil)
//             },
//             scope_introducing: false,
//         },
//     ))
// }

fn l1(input: &str) -> IResult<&str, Expression> {
    scope_introducing_block
        .map(|x| x.into())
        // .or(non_scope_introducing_block.map(|x| x.into()))
        .or(function)
        .or(literal.map(Expression::Literal))
        .or(symbol.map(Expression::Symbol))
        .parse(input)
}

fn infix_call_inner(input: &str) -> IResult<&str, (Expression, Vec<Expression>)> {
    let (s, _) = tag("`")(input)?;
    let (s, _) = multispace0(s)?;
    let (s, fun_exp) = l1.parse(s)?;
    let (s, _) = multispace0(s)?;
    let (s, rest) = separated_list0(multispace1, l1)(s)?;
    Ok((s, (fun_exp, rest)))
}

fn infix_call(input: &str) -> IResult<&str, Expression> {
    let (s, first) = l1.parse(input)?;
    let (s, _) = multispace0(s)?;
    let (s, rest) = many1(s_d(infix_call_inner))(s)?;
    Ok((
        s,
        rest.into_iter().fold(first, |a, v| {
            Expression::Call(Box::new(v.0), {
                let mut args = vec![a];
                args.extend(v.1);
                args
            })
        }),
    ))
}

fn l2(input: &str) -> IResult<&str, Expression> {
    infix_call.map(|x| x.into()).or(l1).parse(input)
}

fn normal_call(input: &str) -> IResult<&str, Expression> {
    let (s, f) = l2(input)?;
    let (s, _) = multispace1(s)?;
    let (s, args) = separated_list1(multispace1, l2)(s)?;
    Ok((s, Expression::Call(Box::new(f), args)))
}

fn l3(input: &str) -> IResult<&str, Expression> {
    normal_call.map(|x| x.into()).or(l2).parse(input)
}

fn no_arg_call(input: &str) -> IResult<&str, Expression> {
    let (s, e) = l3(input)?;
    let (s, _) = multispace0(s)?;
    let (s, _) = tag("!")(s)?;
    Ok((s, Expression::Call(Box::new(e), vec![])))
}

fn l4(input: &str) -> IResult<&str, Expression> {
    no_arg_call.map(|x| x.into()).or(l3).parse(input)
}

fn assignment(input: &str) -> IResult<&str, Expression> {
    let (s, _) = tag("let")(input)?;
    let (s, _) = multispace1(s)?;
    let (s, id) = symbol(s)?;
    let (s, _) = multispace1(s)?;
    let (s, ex) = expression(s)?;
    Ok((s, Expression::Assign(id, Box::new(ex))))
}

fn cond(input: &str) -> IResult<&str, Expression> {
    let (s, _) = tag("cond")(input)?;
    let (s, _) = s_d(tag("{"))(s)?;
    let (s, (exps, _)) = separated_delimited_list0(
        s_d(tag(",")),
        tuple((expression, s_d(tag("=>")), expression)).map(|(x1, _, x2)| (x1, x2)),
    )(s)?;
    let (s, last_expression) = opt(expression)(s)?;
    let (s, _) = multispace0(s)?;
    let (s, _) = tag("}")(s)?;
    Ok((
        s,
        Expression::Condition(
            exps,
            Box::new(last_expression.unwrap_or(Expression::Literal(Literal::Nil))),
        ),
    ))
}

fn expression(input: &str) -> IResult<&str, Expression> {
    assignment
        .map(|x| x.into())
        .or(cond.map(|x| x.into()))
        .or(l4)
        .parse(input)
}

pub fn parse_program(input: &str) -> IResult<&str, Vec<Expression>> {
    block
        .map(|(mut es, se)| {
            if let Some(e) = se {
                es.push(e);
                es
            } else {
                es
            }
        })
        .parse(input)
}

#[cfg(test)]
mod tests {

    use super::*;

    // #[test]
    // fn test_expressions() {
    //     assert_eq!(
    //         parse_program("let x 2;set y 4;"),
    //         Ok((
    //             "",
    //             vec![
    //                 Assignment {
    //                     identifier: Symbol("x".into()),
    //                     value: 2.0.into()
    //                 }
    //                 .into(),
    //                 Assignment {
    //                     identifier: Symbol("y".into()),
    //                     value: 4.0.into()
    //                 }
    //                 .into()
    //             ]
    //         ))
    //     )
    // }

    // #[test]
    // fn test_infix_calls() {
    //     assert_eq!(
    //         parse_program("a `* y"),
    //         Ok((
    //             "",
    //             vec![Call {
    //                 function: Symbol("*".into()).into(),
    //                 arguments: vec![Symbol("a".into()).into(), Symbol("y".into()).into()]
    //             }
    //             .into()]
    //         ))
    //     );

    //     assert_eq!(
    //         parse_program("x`a 2` b 3 4 `c"),
    //         Ok((
    //             "",
    //             vec![Call {
    //                 function: Symbol("c".into()).into(),
    //                 arguments: vec![Call {
    //                     function: Symbol("b".into()).into(),
    //                     arguments: vec![
    //                         Call {
    //                             function: Symbol("a".into()).into(),
    //                             arguments: vec![Symbol("x".into()).into(), 2.0.into()],
    //                         }
    //                         .into(),
    //                         3.0.into(),
    //                         4.0.into()
    //                     ],
    //                 }
    //                 .into(),],
    //             }
    //             .into()]
    //         ))
    //     );
    // }

    // #[test]
    // fn test_symbol() {
    //     assert_eq!(
    //         symbol("hello_there"),
    //         Ok(("", Symbol("hello_there".into())))
    //     );
    //     assert_eq!(symbol("a symbol"), Ok((" symbol", Symbol("a".into()))));
    //     assert!(symbol("2start").is_err());
    //     assert!(symbol("set").is_err());
    //     assert_eq!(symbol("=23"), Ok(("", Symbol("=23".into()))));
    //     assert_eq!(symbol("=2;!3"), Ok((";!3", Symbol("=2".into()))));
    // }

    // #[test]
    // fn test_surrounded_list() {
    //     assert_eq!(
    //         separated_delimited_list0(delimited(multispace0, tag(","), multispace0), number)
    //             .parse(",, , ,,,,2,, 3, 4, 5 ,,, , ,,,"),
    //         Ok((
    //             "",
    //             (
    //                 vec![2.0, 3.0, 4.0, 5.0]
    //                     .into_iter()
    //                     .map(|x| x.into())
    //                     .collect(),
    //                 true
    //             )
    //         ))
    //     );
    //     assert_eq!(
    //         separated_delimited_list0(delimited(multispace0, tag(","), multispace0), number)
    //             .parse(",,,"),
    //         Ok(("", (vec![], false)))
    //     );
    // }

    #[test]
    fn test_cond() {
        assert!(cond("cond {x => 2,    , , , 2 .+ 3 => 3,, x .* 30 }").is_ok(),)
    }
}
