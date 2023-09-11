use crate::{
    expression::Expression,
    tokeniser::{Token, TokenData},
};

#[derive(Debug, PartialEq)]
pub enum ParseError {
    NoMoreTokens,
    UndefinedError,
}

type Result<Success> = std::result::Result<Success, ParseError>;

trait Take {
    type Item;
    fn take_matching(&self, item: Self::Item) -> Option<(&Self, &Self::Item)>;
}

impl<'a> Take for [Token<'a>] {
    type Item = TokenData<'a>;

    fn take_matching(&self, item: Self::Item) -> Option<(&Self, &Self::Item)> {
        self.get(0).and_then(|i| {
            if matches!(&i.data, item) {
                Some((&self[1..], &i.data))
            } else {
                None
            }
        })
    }
}

pub fn parse_expression<'a>(tokens: &[Token<'a>]) -> Option<(&'a [Token<'a>], Expression)> {
    todo!()
}

fn parse_function<'a>(tokens: &'a [Token<'a>]) -> Option<(&'a [Token<'a>], Expression)> {
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

fn parse_symbol<'a>(tokens: &'a [Token<'a>]) -> Option<(&'a [Token<'a>], Expression)> {
    tokens.take_matching(TokenData::Symbol(""))
}

#[cfg(test)]
mod test {
    use crate::parser::Take;
    use crate::tokeniser::{Token, TokenData};

    #[test]
    fn take_works() {
        let tokens = Token::tokenise_source("a|a| a", "")
            .map(|i| i.unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            tokens.take_matching(TokenData::Symbol("")).map(|x| x.1),
            Some(&TokenData::Symbol("a"))
        );
    }
}
