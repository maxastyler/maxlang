use std::{error::Error, fmt::Debug};

use crate::expression::Symbol;

#[derive(PartialEq, Debug)]
pub enum TokeniserError {
    OpenString,
    NoMatch,
}

type Result<Success> = std::result::Result<Success, TokeniserError>;

#[derive(PartialEq, Clone)]
pub struct Location<'a> {
    pub file: &'a str,
    pub source: &'a str,
    pub start_pos: usize,
    pub end_pos: usize,
}

impl<'a> Debug for Location<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Loc ({} -> {})", self.start_pos, self.end_pos))
    }
}

impl<'a> Location<'a> {
    pub fn between(from: &Location<'a>, to: &Location<'a>) -> Location<'a> {
        Location {
            file: from.file,
            source: from.source,
            start_pos: from.start_pos,
            end_pos: to.end_pos,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum TokenData<'a> {
    Pipe,
    ExclamationMark,
    Apostrophe,
    Comma,
    OpenSquareBracket,
    CloseSquareBracket,
    OpenAngleBracket,
    CloseAngleBracket,
    OpenCurlyBracket,
    CloseCurlyBracket,
    SemiColon,
    OpenParen,
    CloseParen,
    Dollar,
    Cond,
    Colon,
    Let,
    LetRec,
    Extract,
    True,
    False,
    Nil,
    Else,
    Tilde,
    Number(&'a str),
    String(&'a str),
    Symbol(&'a str),
}

#[derive(Debug, PartialEq)]
pub struct Token<'a> {
    pub data: TokenData<'a>,
    pub location: Location<'a>,
}

struct TokenIterator<'a> {
    source: &'a str,
    file: &'a str,
    pos: usize,
}

impl<'a> TokenIterator<'a> {
    fn new(source: &'a str, file: &'a str) -> Self {
        TokenIterator {
            source,
            file,
            pos: 0,
        }
    }
}

impl<'a> Iterator for TokenIterator<'a> {
    type Item = Result<Token<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        match Token::get_token_from_string(self.source.get(self.pos..)?) {
            Ok(Some((t, start, end))) => {
                let token = Token {
                    data: t,
                    location: Location {
                        file: self.file,
                        source: self.source,
                        start_pos: self.pos + start,
                        end_pos: self.pos + end,
                    },
                };
                self.pos += end;
                Some(Ok(token))
            }
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

enum Offset {
    Columns(usize),
    LinesAndColumns { lines: usize, columns: usize },
}

#[derive(Debug, PartialEq)]
pub enum StringMatch<'a> {
    Closed(TokenData<'a>, usize),
    Open,
}

impl<'a> Token<'a> {
    /// Match a single character from a stream
    fn match_single(source: &'a str) -> Option<TokenData<'a>> {
        match source.get(0..1)? {
            "`" => Some(TokenData::Apostrophe),
            "$" => Some(TokenData::Dollar),
            "|" => Some(TokenData::Pipe),
            "," => Some(TokenData::Comma),
            "(" => Some(TokenData::OpenParen),
            ")" => Some(TokenData::CloseParen),
            "[" => Some(TokenData::OpenSquareBracket),
            "]" => Some(TokenData::CloseSquareBracket),
            "{" => Some(TokenData::OpenCurlyBracket),
            "}" => Some(TokenData::CloseCurlyBracket),
            "<" => Some(TokenData::OpenAngleBracket),
            ">" => Some(TokenData::CloseAngleBracket),
            ":" => Some(TokenData::Colon),
            "!" => Some(TokenData::ExclamationMark),
            "~" => Some(TokenData::Tilde),
            ";" => Some(TokenData::SemiColon),
            _ => None,
        }
    }

    /// Matches a whole token string again the keywords
    fn matches_keyword(token: &'a str) -> Option<TokenData<'a>> {
        match token {
            "let" => Some(TokenData::Let),
            "letrec" => Some(TokenData::LetRec),
            "extract" => Some(TokenData::Extract),
            "else" => Some(TokenData::Else),
            "cond" => Some(TokenData::Cond),
            "true" => Some(TokenData::True),
            "false" => Some(TokenData::False),
            "nil" => Some(TokenData::Nil),
            _ => None,
        }
    }

    fn match_string(source: &'a str) -> Result<Option<(&str, usize)>> {
        match source.get(0..1) {
            Some("\"") => {
                let mut offset = 0;
                loop {
                    if let Some(c) = source.get(offset + 1..offset + 2) {
                        if c == "\"" && source.get(offset..offset + 1).unwrap() != "\\" {
                            break;
                        } else {
                            offset += 1
                        }
                    } else {
                        return Err(TokeniserError::OpenString);
                    }
                }
                Ok(Some((&source[..offset + 2], offset + 2)))
            }
            _ => Ok(None),
        }
    }

    fn match_symbol(source: &'a str) -> Option<(&str, usize)> {
        let mut offset = 0;
        loop {
            if let Some(s) = source.get(offset..) {
                if Self::match_single(s).is_some()
                    || Self::is_whitespace(s)
                    || s.get(0..1).map(|c| c == "\"").unwrap_or(true)
                {
                    break;
                } else {
                    offset += 1;
                }
            } else {
                break;
            }
        }
        if offset == 0 {
            None
        } else {
            Some((&source[..offset], offset))
        }
    }

    fn match_digits(source: &'a str) -> Option<usize> {
        fn is_digit(s: &str) -> bool {
            match s {
                "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => true,
                _ => false,
            }
        }
        let mut offset = 0;
        loop {
            if let Some(d) = source.get(offset..offset + 1) {
                if is_digit(d) {
                    offset += 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        if offset == 0 {
            None
        } else {
            Some(offset)
        }
    }

    fn match_sign(source: &'a str) -> bool {
        match source.get(0..1) {
            Some("+") | Some("-") => true,
            _ => false,
        }
    }

    fn match_exponent(source: &'a str) -> Option<usize> {
        let mut offset = 0;
        match source.get(0..1)? {
            "e" | "E" => {
                offset += 1;
                if Self::match_sign(source.get(offset..)?) {
                    offset += 1;
                }
                Some(offset + Self::match_digits(source.get(offset..)?)?)
            }
            _ => None,
        }
    }

    fn match_number(source: &'a str) -> Option<(&str, usize)> {
        let mut offset = 0;
        if Self::match_sign(source) {
            offset += 1;
        };
        let n = Self::match_digits(&source.get(offset..)?)?;
        offset += n;
        if source.get(offset..offset + 1) == Some(".") {
            offset += 1;
            if let Some(n) = &source.get(offset..).and_then(|x| Self::match_digits(x)) {
                offset += n;
            }
        }
        if let Some(n) = source.get(offset..).and_then(|x| Self::match_exponent(x)) {
            offset += n;
        };
        Some((&source[..offset], offset))
    }

    /// Returns true if the next character in the string is whitespace
    fn is_whitespace(source: &'a str) -> bool {
        if let Some(c) = source.get(0..1) {
            match c {
                "\t" | "\n" | "\r" | " " => true,
                _ => false,
            }
        } else {
            false
        }
    }

    /// Skip all forms of whitespace in the source until the next non-whitespace character,
    /// and return a tuple of (string after skipping, chars skipped)
    fn remove_whitespace(source: &'a str) -> (&'a str, usize) {
        let mut offset = 0;
        while source
            .get(offset..offset + 1)
            .map(|s| Self::is_whitespace(s))
            .unwrap_or(false)
        {
            offset += 1;
        }
        (source.get(offset..).unwrap(), offset)
    }

    /// Get the next token as a string, returning a tuple of
    /// (token, offset to start, offset to end)
    fn get_token_from_string(source: &'a str) -> Result<Option<(TokenData<'a>, usize, usize)>> {
        let (s, start_offset) = Self::remove_whitespace(source);
        if s.is_empty() {
            Ok(None)
        } else if let Some(s) = Self::match_single(s) {
            Ok(Some((s, start_offset, start_offset + 1)))
        } else if let Some((string_token, string_offset)) = Self::match_string(s)? {
            Ok(Some((
                TokenData::String(string_token),
                start_offset,
                start_offset + string_offset,
            )))
        } else if let Some((number_token, string_offset)) = Self::match_number(s) {
            Ok(Some((
                TokenData::Number(number_token),
                start_offset,
                start_offset + string_offset,
            )))
        } else if let Some((symbol_token, string_offset)) = Self::match_symbol(s) {
            if let Some(kw) = Self::matches_keyword(symbol_token) {
                Ok(Some((kw, start_offset, start_offset + string_offset)))
            } else {
                Ok(Some((
                    TokenData::Symbol(symbol_token),
                    start_offset,
                    start_offset + string_offset,
                )))
            }
        } else {
            Err(TokeniserError::NoMatch)
        }
    }

    pub fn tokenise_source(
        source: &'a str,
        file: &'a str,
    ) -> impl Iterator<Item = Result<Token<'a>>> {
        TokenIterator {
            source,
            file,
            pos: 0,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::tokeniser::{StringMatch, TokenData, TokeniserError};

    use super::Token;

    #[test]
    fn remove_whitespace_works() {
        assert_eq!(Token::remove_whitespace("   hi"), ("hi", 3));
        assert_eq!(Token::remove_whitespace("\n\n    \nhi"), ("hi", 7));
        assert_eq!(Token::remove_whitespace(""), ("", 0));
        assert_eq!(Token::remove_whitespace("a b"), ("a b", 0));
        assert_eq!(Token::remove_whitespace(" b c "), ("b c ", 1));
    }

    #[test]
    fn match_string_works() {
        assert_eq!(Token::match_string("\"this\""), Ok(Some(("\"this\"", 6))));
        assert_eq!(
            Token::match_string("\"open"),
            Err(TokeniserError::OpenString)
        );
        assert_eq!(
            Token::match_string("\"\\\"hi\\\"\""),
            Ok(Some(("\"\\\"hi\\\"\"", 8)))
        );
        assert_eq!(Token::match_string("none"), Ok(None));
    }

    #[test]
    fn match_symbol_works() {
        assert_eq!(Token::match_symbol("sym"), Some(("sym", 3)));
        assert_eq!(Token::match_symbol("sym`"), Some(("sym", 3)));
        assert_eq!(Token::match_symbol("sym  "), Some(("sym", 3)));
        assert_eq!(Token::match_symbol("sym\""), Some(("sym", 3)));
        assert_eq!(Token::match_symbol(""), None);
    }

    #[test]
    fn match_digits_works() {
        assert_eq!(Token::match_digits("123"), Some(3));
        assert_eq!(Token::match_digits("1 3"), Some(1));
        assert_eq!(Token::match_digits("a123"), None);
    }

    #[test]
    fn match_decimal_works() {
        assert_eq!(Token::match_number("2.3"), Some(("2.3", 3)));
        assert_eq!(Token::match_number("-2.3"), Some(("-2.3", 4)));
        assert_eq!(Token::match_number("101202 "), Some(("101202", 6)));
        assert_eq!(Token::match_number(" "), None);
        assert_eq!(Token::match_number("-2.e-10"), Some(("-2.e-10", 7)));
        assert_eq!(Token::match_number("-2.E10"), Some(("-2.E10", 6)));
        assert_eq!(Token::match_number("-2.e+1"), Some(("-2.e+1", 6)));
        assert_eq!(Token::match_number("-2e1"), Some(("-2e1", 4)));
        assert_eq!(Token::match_number("+2.1e+10"), Some(("+2.1e+10", 8)));
    }

    #[test]
    fn get_token_from_string_works() {
        assert_eq!(
            Token::get_token_from_string("`2"),
            Ok(Some((TokenData::Apostrophe, 0, 1)))
        );
        assert_eq!(
            Token::get_token_from_string("   |2"),
            Ok(Some((TokenData::Pipe, 3, 4)))
        );
        assert_eq!(
            Token::get_token_from_string(" \"this\" "),
            Ok(Some((TokenData::String("\"this\""), 1, 7)))
        );
        assert_eq!(
            Token::get_token_from_string("\n\n0.3e10 "),
            Ok(Some((TokenData::Number("0.3e10"), 2, 8)))
        );
        assert_eq!(
            Token::get_token_from_string("\"open"),
            Err(TokeniserError::OpenString)
        )
    }

    #[test]
    fn tokenise_source_works() {
        use TokenData as T;
        assert_eq!(
            Token::tokenise_source("|x y|{x `* -2.0}", "")
                .map(|x| x.unwrap().data)
                .collect::<Vec<_>>(),
            vec![
                T::Pipe,
                T::Symbol("x"),
                T::Symbol("y"),
                T::Pipe,
                T::OpenCurlyBracket,
                T::Symbol("x"),
                T::Apostrophe,
                T::Symbol("*"),
                T::Number("-2.0"),
                T::CloseCurlyBracket
            ]
        );
        assert_eq!(
            Token::tokenise_source("let x 2, y cond {true ~ \"3\", else nil\n}", "")
                .map(|x| x.unwrap().data)
                .collect::<Vec<_>>(),
            vec![
                T::Let,
                T::Symbol("x"),
                T::Number("2"),
                T::Comma,
                T::Symbol("y"),
                T::Cond,
                T::OpenCurlyBracket,
                T::True,
                T::Tilde,
                T::String("\"3\""),
                T::Comma,
                T::Else,
                T::Nil,
                T::CloseCurlyBracket
            ]
        );
    }
}
