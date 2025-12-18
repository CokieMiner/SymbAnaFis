//! Pratt parser for building abstract syntax trees from tokens
//!
//! Implements a top-down operator precedence parser with support for
//! infix operators, prefix operators (unary minus), and function calls.

use crate::parser::tokens::{Operator, Token};
use crate::{DiffError, Expr};

/// Parse tokens into an AST using Pratt parsing algorithm
pub(crate) fn parse_expression(tokens: &[Token]) -> Result<Expr, DiffError> {
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
                        span: None,
                    });
                }
            }
        }

        Ok(args)
    }

    fn parse_prefix(&mut self) -> Result<Expr, DiffError> {
        // Direct access enables borrowing token while mutating self.pos (via advance)
        // because we borrow from the underlying slice 'a, not from self
        let token = self
            .tokens
            .get(self.pos)
            .ok_or(DiffError::UnexpectedEndOfInput)?;

        match token {
            Token::Number(n) => {
                self.advance();
                Ok(Expr::number(*n))
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
                            span: None,
                        });
                    }

                    Ok(Expr::func_multi(name.clone(), args))
                } else {
                    Ok(Expr::symbol(name.clone()))
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
                            span: None,
                        });
                    }

                    // Use the canonical name from Operator::to_name()
                    let func_name = op.to_name();

                    Ok(Expr::func_multi(func_name, args))
                } else {
                    Err(DiffError::UnexpectedToken {
                        expected: "(".to_string(),
                        got: format!("{:?}", self.current()),
                        span: None,
                    })
                }
            }

            // Unary minus: precedence between Mul (20) and Pow (30)
            // This ensures -x^2 parses as -(x^2), not (-x)^2
            Token::Operator(Operator::Sub) => {
                self.advance();
                let expr = self.parse_expr(25)?; // Lower than Pow (30), higher than Mul (20)
                Ok(Expr::mul_expr(Expr::number(-1.0), expr))
            }

            // Unary plus: same precedence as unary minus, just returns the expression
            Token::Operator(Operator::Add) => {
                self.advance();
                self.parse_expr(25) // Same precedence as unary minus
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
                        span: None,
                    })
                }
            }

            Token::Derivative {
                order,
                func,
                args,
                var,
            } => {
                self.advance();

                let arg_exprs = if args.is_empty() {
                    // Implicit dependency on the differentiation variable
                    vec![Expr::symbol(var.clone())]
                } else {
                    // Parse the tokenized arguments
                    // We create a temporary sub-parser for the argument tokens
                    let mut sub_parser = Parser {
                        tokens: args,
                        pos: 0,
                    };
                    let mut exprs = Vec::new();

                    loop {
                        if sub_parser.current().is_none() {
                            break;
                        }
                        let expr = sub_parser.parse_expr(0)?;
                        exprs.push(expr);

                        if let Some(Token::Comma) = sub_parser.current() {
                            sub_parser.advance();
                        } else {
                            // If not comma, we expect end of input (sub-parser exhausted)
                            if sub_parser.current().is_some() {
                                return Err(DiffError::UnexpectedToken {
                                    expected: "comma or end of arguments".to_string(),
                                    got: format!("{:?}", sub_parser.current()), // sub_parser.current() is Option<&Token>
                                    span: None,
                                });
                            }
                            break;
                        }
                    }
                    exprs
                };

                let inner_expr = Expr::func_multi(func.clone(), arg_exprs);

                Ok(Expr::derivative(inner_expr, var.clone(), *order))
            }

            _ => Err(DiffError::invalid_token(token.to_user_string())),
        }
    }

    fn parse_infix(&mut self, left: Expr, precedence: u8) -> Result<Expr, DiffError> {
        let token = self
            .tokens
            .get(self.pos)
            .ok_or(DiffError::UnexpectedEndOfInput)?;

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
                    Operator::Add => Expr::add_expr(left, right),
                    Operator::Sub => Expr::sub_expr(left, right),
                    Operator::Mul => Expr::mul_expr(left, right),
                    Operator::Div => Expr::div_expr(left, right),
                    Operator::Pow => Expr::pow(left, right),
                    _ => {
                        return Err(DiffError::invalid_token(format!(
                            "operator '{}'",
                            op.to_name()
                        )));
                    }
                };

                Ok(result)
            }

            _ => Err(DiffError::invalid_token(token.to_user_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Expr, ExprKind};

    #[test]
    fn test_parse_number() {
        let tokens = vec![Token::Number(314.0 / 100.0)];
        let ast = parse_expression(&tokens).unwrap();
        assert_eq!(ast, Expr::number(314.0 / 100.0));
    }

    #[test]
    fn test_parse_symbol() {
        let tokens = vec![Token::Identifier("x".to_string())];
        let ast = parse_expression(&tokens).unwrap();
        assert_eq!(ast, Expr::symbol("x"));
    }

    #[test]
    fn test_parse_addition() {
        let tokens = vec![
            Token::Number(1.0),
            Token::Operator(Operator::Add),
            Token::Number(2.0),
        ];
        let ast = parse_expression(&tokens).unwrap();
        assert!(matches!(ast.kind, ExprKind::Sum(_)));
    }

    #[test]
    fn test_parse_multiplication() {
        let tokens = vec![
            Token::Identifier("x".to_string()),
            Token::Operator(Operator::Mul),
            Token::Number(2.0),
        ];
        let ast = parse_expression(&tokens).unwrap();
        assert!(matches!(ast.kind, ExprKind::Product(_)));
    }

    #[test]
    fn test_parse_power() {
        let tokens = vec![
            Token::Identifier("x".to_string()),
            Token::Operator(Operator::Pow),
            Token::Number(2.0),
        ];
        let ast = parse_expression(&tokens).unwrap();
        assert!(matches!(ast.kind, ExprKind::Pow(_, _)));
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
        assert!(matches!(ast.kind, ExprKind::FunctionCall { .. }));
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

        // With n-ary Sum, this becomes Sum([x, Product([2, 3])])
        match &ast.kind {
            ExprKind::Sum(terms) => {
                assert_eq!(terms.len(), 2);
                // One should be a symbol, one should be a product
                let has_symbol = terms.iter().any(|t| matches!(t.kind, ExprKind::Symbol(_)));
                let has_product = terms.iter().any(|t| matches!(t.kind, ExprKind::Product(_)));
                assert!(has_symbol, "Expected a symbol in sum");
                assert!(has_product, "Expected a product in sum");
            }
            _ => panic!("Expected Sum at top level, got {:?}", ast.kind),
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

        // With n-ary Product: Product([Sum([x, 1]), 2])
        match &ast.kind {
            ExprKind::Product(factors) => {
                assert_eq!(factors.len(), 2);
                let has_sum = factors.iter().any(|f| matches!(f.kind, ExprKind::Sum(_)));
                let has_num = factors
                    .iter()
                    .any(|f| matches!(f.kind, ExprKind::Number(n) if (n - 2.0).abs() < 1e-10));
                assert!(has_sum, "Expected a sum in product");
                assert!(has_num, "Expected number 2 in product");
            }
            _ => panic!("Expected Product at top level, got {:?}", ast.kind),
        }
    }

    #[test]
    fn test_empty_parentheses() {
        // () should be an error, NOT 1.0 or anything else
        let tokens = vec![Token::LeftParen, Token::RightParen];
        let result = parse_expression(&tokens);
        assert!(
            result.is_err(),
            "Empty parentheses should fail to parse, but got: {:?}",
            result
        );
    }
}
