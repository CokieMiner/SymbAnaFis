//! UTF-8 source ranges and streaming token coverage.

use alloc::{vec, vec::Vec};

use proptest::{prop_assert, prop_assert_eq, proptest};

use crate::errors::{ParseError, Span};

use super::super::{BinaryOp, Lexer, Token};

proptest! {
    #[test]
    fn arbitrary_unicode_keeps_monotonic_character_boundaries(source in ".{0,512}") {
        let mut end = 0;
        for item in Lexer::new(&source) {
            let Ok(token) = item else { break; };
            let span = token.span();
            prop_assert!(span.start() >= end);
            prop_assert!(span.start() < span.end());
            prop_assert!(span.end() <= source.len());
            prop_assert!(source.is_char_boundary(span.start()));
            prop_assert!(source.is_char_boundary(span.end()));
            match token {
                Token::Number(text, _) | Token::Name(text, _) => {
                    prop_assert_eq!(source.get(span.start()..span.end()), Some(text));
                }
                Token::Operator(..) | Token::Open(_) | Token::Close(_) | Token::Comma(_) => (),
            }
            end = span.end();
        }
    }
}

#[test]
fn ascii_and_unicode_separators_keep_byte_spans() {
    let source = "\u{0b}\u{0c}\u{2003}\u{03c0}\u{00d7}x\u{2212}1\u{00f7}2";
    let tokens: Vec<_> = Lexer::new(source)
        .collect::<Result<_, _>>()
        .expect("valid tokens");
    assert_eq!(
        tokens,
        vec![
            Token::Name("\u{03c0}", Span::new(5, 7)),
            Token::Operator(BinaryOp::Multiply, Span::new(7, 9)),
            Token::Name("x", Span::new(9, 10)),
            Token::Operator(BinaryOp::Subtract, Span::new(10, 13)),
            Token::Number("1", Span::new(13, 14)),
            Token::Operator(BinaryOp::Divide, Span::new(14, 16)),
            Token::Number("2", Span::new(16, 17)),
        ]
    );
}

#[test]
fn scientific_lookahead_requires_a_digit_or_sign() {
    let source = "2e 2e+ 2e-3 2E4 2e_x";
    let tokens: Vec<_> = Lexer::new(source)
        .collect::<Result<_, _>>()
        .expect("valid tokens");
    assert_eq!(
        tokens,
        vec![
            Token::Number("2", Span::new(0, 1)),
            Token::Name("e", Span::new(1, 2)),
            Token::Number("2e+", Span::new(3, 6)),
            Token::Number("2e-3", Span::new(7, 11)),
            Token::Number("2E4", Span::new(12, 15)),
            Token::Number("2", Span::new(16, 17)),
            Token::Name("e_x", Span::new(17, 20)),
        ]
    );
}

#[test]
fn names_continue_through_unicode_alphanumerics_and_reject_other_symbols() {
    let source = "\u{03b1}\u{0661}_\u{03b2}\u{2603}";
    let mut lexer = Lexer::new(source);
    assert_eq!(
        lexer.next(),
        Some(Ok(Token::Name(
            "\u{03b1}\u{0661}_\u{03b2}",
            Span::new(0, 7)
        )))
    );
    assert!(matches!(
        lexer.next(),
        Some(Err(ParseError::InvalidCharacter { span, .. })) if span == Span::new(7, 10)
    ));
}
