//! Streaming UTF-8 tokenization with borrowed names and numeric spellings,
//! and scalar operator precedence.

#![expect(
    unsafe_code,
    reason = "token text is sliced between offsets that the scan leaves on character boundaries"
)]

use crate::errors::{ParseError, Span};

/// A cursor whose token boundaries are UTF-8 character boundaries.
pub struct Lexer<'source> {
    source: &'source str,
    bytes: &'source [u8],
    offset: usize,
}

/// A binary scalar operation in the source grammar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Power,
}

/// A source token retaining its UTF-8 byte range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Token<'source> {
    /// A numeric literal: ASCII digits, `_` separators, at most one decimal
    /// point, and an exponent `e` or `E` with an optional sign.
    Number(&'source str, Span),
    Name(&'source str, Span),
    Operator(BinaryOp, Span),
    Open(Span),
    Close(Span),
    Comma(Span),
}

impl<'source> Lexer<'source> {
    /// Start scanning a source string without allocating a token buffer.
    pub const fn new(source: &'source str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            offset: 0,
        }
    }

    /// Whether an opening parenthesis follows, after any whitespace, which
    /// this consumes.
    pub fn at_open(&mut self) -> bool {
        self.skip_whitespace().is_some_and(|(_, byte)| byte == b'(')
    }

    /// Consume the opening parenthesis that [`Self::at_open`] found.
    pub const fn open(&mut self) -> Span {
        self.boundary(self.offset, 1)
    }
}

impl<'source> Iterator for Lexer<'source> {
    type Item = Result<Token<'source>, ParseError>;

    // The parser's token loop is the only caller; inlining it there lets the
    // token's construction and dispatch share one branch.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let (start, byte) = self.skip_whitespace()?;
        let token = match byte {
            b'0'..=b'9' => self.number(start, false),
            b'.' if self
                .bytes
                .get(start.saturating_add(1))
                .is_some_and(u8::is_ascii_digit) =>
            {
                self.number(start, true)
            }
            b'_' | b'a'..=b'z' | b'A'..=b'Z' => self.name(start),
            b'+' => Token::Operator(BinaryOp::Add, self.boundary(start, 1)),
            b'-' => Token::Operator(BinaryOp::Subtract, self.boundary(start, 1)),
            b'*' => {
                if self.bytes.get(start.saturating_add(1)) == Some(&b'*') {
                    Token::Operator(BinaryOp::Power, self.boundary(start, 2))
                } else {
                    Token::Operator(BinaryOp::Multiply, self.boundary(start, 1))
                }
            }
            b'/' => Token::Operator(BinaryOp::Divide, self.boundary(start, 1)),
            b'^' => Token::Operator(BinaryOp::Power, self.boundary(start, 1)),
            b'(' => Token::Open(self.boundary(start, 1)),
            b')' => Token::Close(self.boundary(start, 1)),
            b',' => Token::Comma(self.boundary(start, 1)),
            0x80.. => return self.unicode_token(start),
            _ => {
                return Some(Err(ParseError::InvalidCharacter {
                    character: char::from(byte),
                    span: self.boundary(start, 1),
                }));
            }
        };
        Some(Ok(token))
    }
}

impl<'source> Lexer<'source> {
    /// Scan a literal from `start`, which holds a digit or, when `point`, the
    /// decimal point.
    fn number(&mut self, start: usize, point: bool) -> Token<'source> {
        let bytes = self.bytes;
        let digits = |mut offset: usize| {
            while bytes
                .get(offset)
                .is_some_and(|next| next.is_ascii_digit() || *next == b'_')
            {
                offset = offset.saturating_add(1);
            }
            offset
        };
        // A literal that starts with its point continues after the point.
        let mut offset = digits(if point {
            start.saturating_add(1)
        } else {
            start
        });
        if !point && bytes.get(offset) == Some(&b'.') {
            offset = digits(offset.saturating_add(1));
        }
        let sign = offset.saturating_add(1);
        if matches!(bytes.get(offset), Some(b'e' | b'E'))
            && bytes
                .get(sign)
                .is_some_and(|next| next.is_ascii_digit() || matches!(next, b'+' | b'-'))
        {
            offset = digits(if matches!(bytes.get(sign), Some(b'+' | b'-')) {
                sign.saturating_add(1)
            } else {
                sign
            });
        }
        self.offset = offset;
        // SAFETY: the literal spans ASCII bytes from `start` up to `offset`,
        // which is the end of the source or the start of another character.
        let text = unsafe { self.source.get_unchecked(start..offset) };
        Token::Number(text, Span::new(start, offset))
    }

    fn name(&mut self, start: usize) -> Token<'source> {
        let mut offset = start;
        while self
            .bytes
            .get(offset)
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        {
            offset = offset.saturating_add(1);
        }
        if self.bytes.get(offset).is_some_and(|byte| *byte >= 0x80) {
            offset = self.unicode_name_end(offset);
        }
        self.offset = offset;
        // SAFETY: `start` begins an ASCII byte or a decoded character, and the
        // scan stops after a whole ASCII byte or decoded character.
        let text = unsafe { self.source.get_unchecked(start..offset) };
        Token::Name(text, Span::new(start, offset))
    }

    // Unicode identifiers are uncommon; keep the decoder out of the ASCII path.
    #[cold]
    #[inline(never)]
    fn unicode_name_end(&self, mut offset: usize) -> usize {
        while let Some(&byte) = self.bytes.get(offset) {
            if byte < 0x80 {
                if !byte.is_ascii_alphanumeric() && byte != b'_' {
                    break;
                }
                offset = offset.saturating_add(1);
                continue;
            }
            let Some(character) = self
                .source
                .get(offset..)
                .and_then(|rest| rest.chars().next())
            else {
                break;
            };
            if !character.is_alphanumeric() {
                break;
            }
            offset = offset.saturating_add(character.len_utf8());
        }
        offset
    }

    /// The offset and byte of the next token, past ASCII and Unicode
    /// whitespace; `None` at the end of the source.
    fn skip_whitespace(&mut self) -> Option<(usize, u8)> {
        let mut offset = self.offset;
        while let Some(&byte) = self.bytes.get(offset) {
            if byte.is_ascii_whitespace() || byte == b'\x0b' {
                offset = offset.saturating_add(1);
                continue;
            }
            if byte >= 0x80 {
                let end = self.unicode_whitespace_end(offset);
                if end != offset {
                    offset = end;
                    continue;
                }
            }
            self.offset = offset;
            return Some((offset, byte));
        }
        self.offset = offset;
        None
    }

    // Unicode whitespace is uncommon; keep the decoder out of the ASCII path.
    #[cold]
    #[inline(never)]
    fn unicode_whitespace_end(&self, mut offset: usize) -> usize {
        while let Some(character) = self
            .source
            .get(offset..)
            .and_then(|rest| rest.chars().next())
        {
            if !character.is_whitespace() {
                break;
            }
            offset = offset.saturating_add(character.len_utf8());
        }
        offset
    }

    // Non-ASCII tokens are uncommon; keep the decoder out of the ASCII path.
    #[cold]
    #[inline(never)]
    fn unicode_token(&mut self, start: usize) -> Option<Result<Token<'source>, ParseError>> {
        let character = self.source.get(start..)?.chars().next()?;
        let length = character.len_utf8();
        let operation = match character {
            '\u{2212}' => BinaryOp::Subtract,
            '\u{00d7}' | '\u{00b7}' => BinaryOp::Multiply,
            '\u{00f7}' => BinaryOp::Divide,
            letter if letter.is_alphabetic() => return Some(Ok(self.name(start))),
            _ => {
                return Some(Err(ParseError::InvalidCharacter {
                    character,
                    span: self.boundary(start, length),
                }));
            }
        };
        Some(Ok(Token::Operator(operation, self.boundary(start, length))))
    }

    const fn boundary(&mut self, start: usize, length: usize) -> Span {
        let end = start.saturating_add(length);
        self.offset = end;
        Span::new(start, end)
    }
}

impl Token<'_> {
    /// Return the token's half-open byte range.
    pub const fn span(self) -> Span {
        match self {
            Self::Number(_, span)
            | Self::Name(_, span)
            | Self::Operator(_, span)
            | Self::Open(span)
            | Self::Close(span)
            | Self::Comma(span) => span,
        }
    }
}

impl BinaryOp {
    /// Rank addition below multiplication and right-associative exponentiation.
    pub const fn precedence(self) -> u8 {
        match self {
            Self::Add | Self::Subtract => 1,
            Self::Multiply | Self::Divide => 2,
            Self::Power => 4,
        }
    }
}
