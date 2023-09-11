use std::error::Error;

#[derive(PartialEq, Debug)]
pub enum TokeniserError {
    OpenString,
}

type Result<Success> = std::result::Result<Success, TokeniserError>;

pub struct Location<'a> {
    pub file: &'a str,
    pub source: &'a str,
    start_pos: usize,
    end_pos: usize,
}

#[derive(Debug, PartialEq)]
pub enum TokenData<'a> {
    Pipe,
    Apostrophe,
    Comma,
    OpenSquareBracket,
    CloseSquareBracket,
    OpenAngleBracket,
    CloseAngleBracket,
    OpenCurlyBracket,
    CloseCurlyBracket,
    OpenParen,
    CloseParen,
    Dollar,
    Colon,
    Let,
    LetRec,
    Extract,
    Number(&'a str),
    String(&'a str),
    Symbol(&'a str),
}

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
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
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
            _ => None,
        }
    }

    /// Matches a whole token string again the keywords
    fn matches_keyword(token: &'a str) -> Option<TokenData<'a>> {
        match token {
            "let" => Some(TokenData::Let),
            "letrec" => Some(TokenData::LetRec),
            "extract" => Some(TokenData::Extract),
            _ => None,
        }
    }

    fn match_string(source: &'a str) -> Result<Option<(TokenData, usize)>> {
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
                Ok(Some((TokenData::String(&source[..offset + 2]), offset + 2)))
            }
            _ => Ok(None),
        }
    }

    fn match_symbol(source: &'a str) -> (&str, usize) {
        let mut offset = 0;
        loop {
            let source = &source[offset..];
            if Self::match_single(source).is_some()
                || Self::is_whitespace(source)
                || source.get(0..1).map(|c| c == "\"").unwrap_or(true)
            {
                break;
            } else {
                offset += 1;
            }
        }
        (&source[..offset], offset)
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
                string_token,
                start_offset,
                start_offset + string_offset,
            )))
        } else {
            Ok(None)
        }
    }

    pub fn tokenise_source(source: &'a str, file: &'a str) -> impl Iterator<Item = Token<'a>> {
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
        assert_eq!(
            Token::match_string("\"this\""),
            Ok(Some((TokenData::String("\"this\""), 6)))
        );
        assert_eq!(
            Token::match_string("\"open"),
            Err(TokeniserError::OpenString)
        );
        assert_eq!(
            Token::match_string("\"\\\"hi\\\"\""),
            Ok(Some((TokenData::String("\"\\\"hi\\\"\""), 8)))
        );
        assert_eq!(Token::match_string("none"), Ok(None));
    }

    #[test]
    fn match_symbol_works() {
        assert_eq!(Token::match_symbol("sym"), ("sym", 3));
        assert_eq!(Token::match_symbol("sym`"), ("sym", 3));
        assert_eq!(Token::match_symbol("sym  "), ("sym", 3));
        assert_eq!(Token::match_symbol("sym\""), ("sym", 3));
        assert_eq!(Token::match_symbol(""), ("", 0));
    }
}
