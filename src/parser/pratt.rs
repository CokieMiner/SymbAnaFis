// Pratt parser - converts tokens to AST
use crate::parser::{Operator, Token};
use crate::{DiffError, Expr};

/// Parse tokens into an AST using Pratt parsing algorithm
pub fn parse_expression(tokens: &[Token]) -> Result<Expr, DiffError> {
    if tokens.is_empty() {
        return Err(DiffError::UnexpectedEndOfInput);
    }

    let mut parser = Parser { tokens, pos: 0 };

    parser.parse_expr(0)
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn parse_expr(&mut self, min_precedence: u8) -> Result<Expr, DiffError> {
        // Parse left side (prefix)
        let mut left = self.parse_prefix()?;

        // Parse operators and right side (infix)
        while let Some(token) = self.current() {
            let precedence = match token {
                Token::Operator(op) if !op.is_function() => op.precedence(),
                _ => break,
            };

            if precedence < min_precedence {
                break;
            }

            left = self.parse_infix(left, precedence)?;
        }

        Ok(left)
    }

    fn parse_arguments(&mut self) -> Result<Vec<Expr>, DiffError> {
        let mut args = Vec::new();

        if let Some(Token::RightParen) = self.current() {
            return Ok(args); // Empty argument list
        }

        loop {
            args.push(self.parse_expr(0)?);

            match self.current() {
                Some(Token::Comma) => {
                    self.advance(); // consume ,
                }
                Some(Token::RightParen) => {
                    break;
                }
                _ => {
                    return Err(DiffError::UnexpectedToken {
                        expected: ", or )".to_string(),
                        got: format!("{:?}", self.current()),
                    });
                }
            }
        }

        Ok(args)
    }

    fn parse_prefix(&mut self) -> Result<Expr, DiffError> {
        let token = self
            .current()
            .ok_or(DiffError::UnexpectedEndOfInput)?
            .clone();

        match token {
            Token::Number(n) => {
                self.advance();
                Ok(Expr::Number(n))
            }

            Token::Identifier(name) => {
                self.advance();

                // Check if this is a function call
                if let Some(Token::LeftParen) = self.current() {
                    // This is a custom function call
                    self.advance(); // consume (
                    let args = self.parse_arguments()?;

                    if let Some(Token::RightParen) = self.current() {
                        self.advance(); // consume )
                    } else {
                        return Err(DiffError::UnexpectedToken {
                            expected: ")".to_string(),
                            got: format!("{:?}", self.current()),
                        });
                    }

                    Ok(Expr::FunctionCall { name, args })
                } else {
                    Ok(Expr::Symbol(name))
                }
            }

            Token::Operator(op) if op.is_function() => {
                self.advance();

                // Function must be followed by (
                if let Some(Token::LeftParen) = self.current() {
                    self.advance(); // consume (
                    let args = self.parse_arguments()?;

                    if let Some(Token::RightParen) = self.current() {
                        self.advance(); // consume )
                    } else {
                        return Err(DiffError::UnexpectedToken {
                            expected: ")".to_string(),
                            got: format!("{:?}", self.current()),
                        });
                    }

                    let func_name = match op {
                        Operator::Sin => "sin",
                        Operator::Cos => "cos",
                        Operator::Tan => "tan",
                        Operator::Cot => "cot",
                        Operator::Sec => "sec",
                        Operator::Csc => "csc",
                        Operator::Asin => "asin",
                        Operator::Acos => "acos",
                        Operator::Atan => "atan",
                        Operator::Acot => "acot",
                        Operator::Asec => "asec",
                        Operator::Acsc => "acsc",
                        Operator::Ln => "ln",
                        Operator::Exp => "exp",
                        Operator::Sinh => "sinh",
                        Operator::Cosh => "cosh",
                        Operator::Tanh => "tanh",
                        Operator::Coth => "coth",
                        Operator::Sech => "sech",
                        Operator::Csch => "csch",
                        Operator::Sqrt => "sqrt",
                        Operator::Cbrt => "cbrt",
                        Operator::Log => "log",
                        Operator::Log10 => "log10",
                        Operator::Log2 => "log2",
                        Operator::Sinc => "sinc",
                        Operator::ExpPolar => "exp_polar",
                        Operator::Erf => "erf",
                        Operator::Erfc => "erfc",
                        Operator::Gamma => "gamma",
                        Operator::Digamma => "digamma",
                        Operator::Trigamma => "trigamma",
                        Operator::Tetragamma => "tetragamma",
                        Operator::Polygamma => "polygamma",
                        Operator::Beta => "beta",
                        Operator::BesselJ => "besselj",
                        Operator::BesselY => "bessely",
                        Operator::BesselI => "besseli",
                        Operator::BesselK => "besselk",
                        Operator::LambertW => "LambertW",
                        Operator::Ynm => "Ynm",
                        Operator::AssocLegendre => "assoc_legendre",
                        Operator::Hermite => "hermite",
                        Operator::EllipticE => "elliptic_e",
                        Operator::EllipticK => "elliptic_k",
                        Operator::Asinh => "asinh",
                        Operator::Acosh => "acosh",
                        Operator::Atanh => "atanh",
                        Operator::Acoth => "acoth",
                        Operator::Asech => "asech",
                        Operator::Acsch => "acsch",
                        _ => unreachable!(),
                    };

                    Ok(Expr::FunctionCall {
                        name: func_name.to_string(),
                        args,
                    })
                } else {
                    Err(DiffError::UnexpectedToken {
                        expected: "(".to_string(),
                        got: format!("{:?}", self.current()),
                    })
                }
            }

            Token::LeftParen => {
                self.advance(); // consume (
                let expr = self.parse_expr(0)?;

                if let Some(Token::RightParen) = self.current() {
                    self.advance(); // consume )
                    Ok(expr)
                } else {
                    Err(DiffError::UnexpectedToken {
                        expected: ")".to_string(),
                        got: format!("{:?}", self.current()),
                    })
                }
            }

            _ => Err(DiffError::InvalidToken(format!("{:?}", token))),
        }
    }

    fn parse_infix(&mut self, left: Expr, precedence: u8) -> Result<Expr, DiffError> {
        let token = self
            .current()
            .ok_or(DiffError::UnexpectedEndOfInput)?
            .clone();

        match token {
            Token::Operator(op) => {
                self.advance();

                // Right associative for power, left for others
                let next_precedence = if matches!(op, Operator::Pow) {
                    precedence // Right associative
                } else {
                    precedence + 1 // Left associative
                };

                let right = self.parse_expr(next_precedence)?;

                let result = match op {
                    Operator::Add => Expr::Add(Box::new(left), Box::new(right)),
                    Operator::Sub => Expr::Sub(Box::new(left), Box::new(right)),
                    Operator::Mul => Expr::Mul(Box::new(left), Box::new(right)),
                    Operator::Div => Expr::Div(Box::new(left), Box::new(right)),
                    Operator::Pow => Expr::Pow(Box::new(left), Box::new(right)),
                    _ => return Err(DiffError::InvalidToken(format!("{:?}", op))),
                };

                Ok(result)
            }

            _ => Err(DiffError::InvalidToken(format!("{:?}", token))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_number() {
        let tokens = vec![Token::Number(3.14)];
        let ast = parse_expression(&tokens).unwrap();
        assert_eq!(ast, Expr::Number(3.14));
    }

    #[test]
    fn test_parse_symbol() {
        let tokens = vec![Token::Identifier("x".to_string())];
        let ast = parse_expression(&tokens).unwrap();
        assert_eq!(ast, Expr::Symbol("x".to_string()));
    }

    #[test]
    fn test_parse_addition() {
        let tokens = vec![
            Token::Number(1.0),
            Token::Operator(Operator::Add),
            Token::Number(2.0),
        ];
        let ast = parse_expression(&tokens).unwrap();
        assert!(matches!(ast, Expr::Add(_, _)));
    }

    #[test]
    fn test_parse_multiplication() {
        let tokens = vec![
            Token::Identifier("x".to_string()),
            Token::Operator(Operator::Mul),
            Token::Number(2.0),
        ];
        let ast = parse_expression(&tokens).unwrap();
        assert!(matches!(ast, Expr::Mul(_, _)));
    }

    #[test]
    fn test_parse_power() {
        let tokens = vec![
            Token::Identifier("x".to_string()),
            Token::Operator(Operator::Pow),
            Token::Number(2.0),
        ];
        let ast = parse_expression(&tokens).unwrap();
        assert!(matches!(ast, Expr::Pow(_, _)));
    }

    #[test]
    fn test_parse_function() {
        let tokens = vec![
            Token::Operator(Operator::Sin),
            Token::LeftParen,
            Token::Identifier("x".to_string()),
            Token::RightParen,
        ];
        let ast = parse_expression(&tokens).unwrap();
        assert!(matches!(ast, Expr::FunctionCall { .. }));
    }

    #[test]
    fn test_precedence() {
        // x + 2 * 3 should be x + (2 * 3)
        let tokens = vec![
            Token::Identifier("x".to_string()),
            Token::Operator(Operator::Add),
            Token::Number(2.0),
            Token::Operator(Operator::Mul),
            Token::Number(3.0),
        ];
        let ast = parse_expression(&tokens).unwrap();

        match ast {
            Expr::Add(left, right) => {
                assert!(matches!(*left, Expr::Symbol(_)));
                assert!(matches!(*right, Expr::Mul(_, _)));
            }
            _ => panic!("Expected Add at top level"),
        }
    }

    #[test]
    fn test_parentheses() {
        // (x + 1) * 2
        let tokens = vec![
            Token::LeftParen,
            Token::Identifier("x".to_string()),
            Token::Operator(Operator::Add),
            Token::Number(1.0),
            Token::RightParen,
            Token::Operator(Operator::Mul),
            Token::Number(2.0),
        ];
        let ast = parse_expression(&tokens).unwrap();

        match ast {
            Expr::Mul(left, right) => {
                assert!(matches!(*left, Expr::Add(_, _)));
                assert!(matches!(*right, Expr::Number(2.0)));
            }
            _ => panic!("Expected Mul at top level"),
        }
    }
}
