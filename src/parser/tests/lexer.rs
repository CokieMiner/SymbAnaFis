//! UTF-8 source ranges and streaming token coverage.

use proptest::{prop_assert, prop_assert_eq, proptest};

use super::super::{Lexer, Token};

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
                Token::Number(text, _, _) | Token::Name(text, _) => {
                    prop_assert_eq!(source.get(span.start()..span.end()), Some(text));
                }
                Token::Operator(..) | Token::Open(_) | Token::Close(_) | Token::Comma(_) => (),
            }
            end = span.end();
        }
    }
}
