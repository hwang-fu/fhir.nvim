#![allow(dead_code)] // not yet wired into the public API

use crate::ast::{Expr, Literal};
use crate::error::Error;
use crate::lexer::{Token, tokenize};

pub fn parse(input: &str) -> Result<Expr, Error> {
    let tokens = tokenize(input)?;
    if tokens.is_empty() {
        return Err(Error::Parse("empty expression".into()));
    }
    let mut parser = Parser { tokens, pos: 0 };
    let expr = parser.parse_expr(0)?;
    if let Some(t) = parser.peek() {
        return Err(Error::Parse(format!(
            "unexpected token after expression: {t:?}"
        )));
    }
    Ok(expr)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn expect(&mut self, want: Token) -> Result<(), Error> {
        match self.next() {
            Some(t) if t == want => Ok(()),
            other => Err(Error::Parse(format!("expected {want:?}, got {other:?}"))),
        }
    }

    fn parse_expr(&mut self, _min_bp: u8) -> Result<Expr, Error> {
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, Error> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.peek() {
                Some(Token::Dot) => {
                    self.next();
                    let name = match self.next() {
                        Some(t) => member_name(t)?,
                        None => return Err(Error::Parse("expected member name after '.'".into())),
                    };
                    if self.peek() == Some(&Token::LParen) {
                        self.next();
                        let args = self.parse_args()?;
                        expr = Expr::Call {
                            base: Some(Box::new(expr)),
                            name,
                            args,
                        };
                    } else {
                        expr = Expr::Member {
                            base: Box::new(expr),
                            name,
                        };
                    }
                }
                Some(Token::LBracket) => {
                    self.next();
                    let index = self.parse_expr(0)?;
                    self.expect(Token::RBracket)?;
                    expr = Expr::Index {
                        base: Box::new(expr),
                        index: Box::new(index),
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, Error> {
        match self.next() {
            Some(Token::True) => Ok(Expr::Literal(Literal::Boolean(true))),
            Some(Token::False) => Ok(Expr::Literal(Literal::Boolean(false))),
            Some(Token::Int(n)) => Ok(Expr::Literal(Literal::Integer(n))),
            Some(Token::Dec(d)) => Ok(Expr::Literal(Literal::Decimal(d))),
            Some(Token::Str(s)) => Ok(Expr::Literal(Literal::Str(s))),
            Some(Token::Date(s)) => Ok(Expr::Literal(Literal::Date(s))),
            Some(Token::DateTime(s)) => Ok(Expr::Literal(Literal::DateTime(s))),
            Some(Token::DollarThis) => Ok(Expr::This),
            Some(Token::Minus) => match self.next() {
                Some(Token::Int(n)) => Ok(Expr::Literal(Literal::Integer(-n))),
                Some(Token::Dec(d)) => Ok(Expr::Literal(Literal::Decimal(-d))),
                other => Err(Error::Parse(format!(
                    "expected number after '-', got {other:?}"
                ))),
            },
            Some(Token::Ident(name)) => {
                if self.peek() == Some(&Token::LParen) {
                    self.next();
                    let args = self.parse_args()?;
                    Ok(Expr::Call {
                        base: None,
                        name,
                        args,
                    })
                } else {
                    Ok(Expr::Identifier(name))
                }
            }
            Some(Token::LParen) => {
                let expr = self.parse_expr(0)?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            other => Err(Error::Parse(format!("unexpected token: {other:?}"))),
        }
    }

    // the opening '(' is already consumed
    fn parse_args(&mut self) -> Result<Vec<Expr>, Error> {
        let mut args = Vec::new();
        if self.peek() == Some(&Token::RParen) {
            self.next();
            return Ok(args);
        }
        loop {
            args.push(self.parse_expr(0)?);
            match self.next() {
                Some(Token::Comma) => continue,
                Some(Token::RParen) => break,
                other => return Err(Error::Parse(format!("expected ',' or ')', got {other:?}"))),
            }
        }
        Ok(args)
    }
}

// keywords double as member names after a dot (e.g. Patient.text.div, x.is)
fn member_name(t: Token) -> Result<String, Error> {
    match t {
        Token::Ident(s) => Ok(s),
        Token::And => Ok("and".into()),
        Token::Or => Ok("or".into()),
        Token::In => Ok("in".into()),
        Token::Is => Ok("is".into()),
        Token::As => Ok("as".into()),
        Token::True => Ok("true".into()),
        Token::False => Ok("false".into()),
        other => Err(Error::Parse(format!("expected member name, got {other:?}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;

    fn id(s: &str) -> Expr {
        Expr::Identifier(s.into())
    }

    #[test]
    fn paths() {
        assert_eq!(parse("name").unwrap(), id("name"));
        assert_eq!(
            parse("Patient.name.given").unwrap(),
            Expr::Member {
                base: Box::new(Expr::Member {
                    base: Box::new(id("Patient")),
                    name: "name".into()
                }),
                name: "given".into()
            }
        );
        // keywords are valid member names after a dot
        assert_eq!(
            parse("x.is").unwrap(),
            Expr::Member {
                base: Box::new(id("x")),
                name: "is".into()
            }
        );
    }

    #[test]
    fn literals() {
        assert_eq!(
            parse("true").unwrap(),
            Expr::Literal(Literal::Boolean(true))
        );
        assert_eq!(parse("42").unwrap(), Expr::Literal(Literal::Integer(42)));
        assert_eq!(parse("-3").unwrap(), Expr::Literal(Literal::Integer(-3)));
        assert_eq!(
            parse("'hi'").unwrap(),
            Expr::Literal(Literal::Str("hi".into()))
        );
        assert_eq!(parse("$this").unwrap(), Expr::This);
    }

    #[test]
    fn indexer_and_calls() {
        assert_eq!(
            parse("name[0]").unwrap(),
            Expr::Index {
                base: Box::new(id("name")),
                index: Box::new(Expr::Literal(Literal::Integer(0)))
            }
        );
        assert_eq!(
            parse("name.exists()").unwrap(),
            Expr::Call {
                base: Some(Box::new(id("name"))),
                name: "exists".into(),
                args: vec![]
            }
        );
        assert_eq!(
            parse("count()").unwrap(),
            Expr::Call {
                base: None,
                name: "count".into(),
                args: vec![]
            }
        );
        assert_eq!(
            parse("skip(1)").unwrap(),
            Expr::Call {
                base: None,
                name: "skip".into(),
                args: vec![Expr::Literal(Literal::Integer(1))]
            }
        );
    }

    #[test]
    fn parens() {
        assert_eq!(parse("(name)").unwrap(), id("name"));
    }

    #[test]
    fn parse_errors() {
        assert!(matches!(parse(""), Err(crate::Error::Parse(_))));
        assert!(matches!(parse("name."), Err(crate::Error::Parse(_))));
        assert!(matches!(parse("name)"), Err(crate::Error::Parse(_))));
        assert!(matches!(parse("(name"), Err(crate::Error::Parse(_))));
    }
}
