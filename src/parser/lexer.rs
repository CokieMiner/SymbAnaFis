//! Streaming UTF-8 tokenization with borrowed names and numeric spellings.

use core::{iter::Peekable, str::CharIndices};

use super::{BinaryOp, NumberKind, ParseError, Span, Token};

/// A cursor whose token boundaries are UTF-8 character boundaries.
pub struct Lexer<'source> {
    source: &'source str,
    characters: Peekable<CharIndices<'source>>,
}

impl<'source> Lexer<'source> {
    /// Start scanning a source string without allocating a token buffer.
    pub fn new(source: &'source str) -> Self {
        Self {
            source,
            characters: source.char_indices().peekable(),
        }
    }
}

impl<'source> Iterator for Lexer<'source> {
    type Item = Result<Token<'source>, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        let (start, character) = self
            .characters
            .find(|(_, character)| !character.is_whitespace())?;
        let token = match character {
            '0'..='9' => self.number(start, character),
            '.' if self
                .characters
                .peek()
                .is_some_and(|(_, next)| next.is_ascii_digit()) =>
            {
                self.number(start, character)
            }
            '_' => self.name(start),
            letter if letter.is_alphabetic() => self.name(start),
            '+' => Token::Operator(BinaryOp::Add, self.span(start)),
            '-' | '\u{2212}' => Token::Operator(BinaryOp::Subtract, self.span(start)),
            '*' => {
                let operation = if self.characters.next_if(|(_, next)| *next == '*').is_some() {
                    BinaryOp::Power
                } else {
                    BinaryOp::Multiply
                };
                Token::Operator(operation, self.span(start))
            }
            '\u{00d7}' | '\u{00b7}' => Token::Operator(BinaryOp::Multiply, self.span(start)),
            '/' | '\u{00f7}' => Token::Operator(BinaryOp::Divide, self.span(start)),
            '^' => Token::Operator(BinaryOp::Power, self.span(start)),
            '(' => Token::Open(self.span(start)),
            ')' => Token::Close(self.span(start)),
            ',' => Token::Comma(self.span(start)),
            _ => {
                return Some(Err(ParseError::InvalidCharacter {
                    character,
                    span: self.span(start),
                }));
            }
        };
        Some(Ok(token))
    }
}

impl<'source> Lexer<'source> {
    fn number(&mut self, start: usize, first: char) -> Token<'source> {
        let mut kind = if first == '.' {
            NumberKind::Float
        } else {
            NumberKind::Integer
        };
        while self
            .characters
            .next_if(|(_, next)| next.is_ascii_digit() || *next == '_')
            .is_some()
        {}
        if first != '.' && self.characters.next_if(|(_, next)| *next == '.').is_some() {
            kind = NumberKind::Float;
            while self
                .characters
                .next_if(|(_, next)| next.is_ascii_digit() || *next == '_')
                .is_some()
            {}
        }
        let mut lookahead = self.characters.clone();
        if lookahead
            .next_if(|(_, next)| matches!(next, 'e' | 'E'))
            .is_some()
            && lookahead
                .peek()
                .is_some_and(|(_, next)| next.is_ascii_digit() || matches!(next, '+' | '-'))
        {
            kind = NumberKind::Float;
            self.characters.next();
            self.characters
                .next_if(|(_, next)| matches!(next, '+' | '-'));
            while self
                .characters
                .next_if(|(_, next)| next.is_ascii_digit() || *next == '_')
                .is_some()
            {}
        }
        let span = self.span(start);
        #[expect(
            clippy::string_slice,
            reason = "both endpoints come from CharIndices or the source length and delimit a consumed token"
        )]
        Token::Number(&self.source[start..span.end()], span, kind)
    }

    fn name(&mut self, start: usize) -> Token<'source> {
        while self
            .characters
            .next_if(|(_, next)| next.is_alphanumeric() || *next == '_')
            .is_some()
        {}
        let span = self.span(start);
        #[expect(
            clippy::string_slice,
            reason = "both endpoints come from CharIndices or the source length and delimit a consumed token"
        )]
        Token::Name(&self.source[start..span.end()], span)
    }

    fn span(&mut self, start: usize) -> Span {
        Span::new(
            start,
            self.characters
                .peek()
                .map_or(self.source.len(), |(offset, _)| *offset),
        )
    }
}
