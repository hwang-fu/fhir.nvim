use crate::ast::{BinOp, Expr, Literal, TypeOp};
use crate::error::Error;
use crate::lexer::{Token, tokenize};

enum Op {
    Bin(BinOp),
    Type(TypeOp),
    // `a contains b` is `b in a`
    ContainsRev,
}

// binding powers, loosest first (FHIRPath precedence)
fn operator(t: &Token) -> Option<(Op, u8)> {
    match t {
        Token::Implies => Some((Op::Bin(BinOp::Implies), 0)),
        Token::Or => Some((Op::Bin(BinOp::Or), 1)),
        Token::Xor => Some((Op::Bin(BinOp::Xor), 1)),
        Token::And => Some((Op::Bin(BinOp::And), 2)),
        Token::In => Some((Op::Bin(BinOp::In), 3)),
        Token::Contains => Some((Op::ContainsRev, 3)),
        Token::Eq => Some((Op::Bin(BinOp::Eq), 4)),
        Token::Ne => Some((Op::Bin(BinOp::Ne), 4)),
        Token::Tilde => Some((Op::Bin(BinOp::Equiv), 4)),
        Token::NotTilde => Some((Op::Bin(BinOp::NotEquiv), 4)),
        Token::Lt => Some((Op::Bin(BinOp::Lt), 5)),
        Token::Le => Some((Op::Bin(BinOp::Le), 5)),
        Token::Gt => Some((Op::Bin(BinOp::Gt), 5)),
        Token::Ge => Some((Op::Bin(BinOp::Ge), 5)),
        Token::Pipe => Some((Op::Bin(BinOp::Union), 6)),
        Token::Is => Some((Op::Type(TypeOp::Is), 7)),
        Token::As => Some((Op::Type(TypeOp::As), 7)),
        Token::Plus => Some((Op::Bin(BinOp::Add), 8)),
        Token::Minus => Some((Op::Bin(BinOp::Sub), 8)),
        Token::Amp => Some((Op::Bin(BinOp::Concat), 8)),
        Token::Star => Some((Op::Bin(BinOp::Mul), 9)),
        Token::Slash => Some((Op::Bin(BinOp::Div), 9)),
        Token::Div => Some((Op::Bin(BinOp::IntDiv), 9)),
        Token::Mod => Some((Op::Bin(BinOp::Mod), 9)),
        _ => None,
    }
}

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

    fn parse_expr(&mut self, min_bp: u8) -> Result<Expr, Error> {
        let mut lhs = self.parse_postfix()?;
        while let Some((op, bp)) = self.peek().and_then(operator) {
            if bp < min_bp {
                break;
            }
            self.next();
            lhs = match op {
                // bp + 1 keeps equal-power operators left-associative
                Op::Bin(op) => Expr::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(self.parse_expr(bp + 1)?),
                },
                Op::Type(op) => Expr::TypeTest {
                    op,
                    expr: Box::new(lhs),
                    type_name: self.parse_type_name()?,
                },
                Op::ContainsRev => Expr::Binary {
                    op: BinOp::In,
                    lhs: Box::new(self.parse_expr(bp + 1)?),
                    rhs: Box::new(lhs),
                },
            };
        }
        Ok(lhs)
    }

    // a qualified identifier: Ident ('.' Ident)*, e.g. Quantity or System.String
    fn parse_type_name(&mut self) -> Result<String, Error> {
        let mut name = match self.next() {
            Some(Token::Ident(s)) => s,
            other => return Err(Error::Parse(format!("expected type name, got {other:?}"))),
        };
        while self.peek() == Some(&Token::Dot) {
            self.next();
            match self.next() {
                Some(Token::Ident(s)) => {
                    name.push('.');
                    name.push_str(&s);
                }
                other => return Err(Error::Parse(format!("expected type name, got {other:?}"))),
            }
        }
        Ok(name)
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
            Some(Token::Minus) => match self.peek() {
                Some(Token::Int(_)) | Some(Token::Dec(_)) => match self.next() {
                    Some(Token::Int(n)) => Ok(Expr::Literal(Literal::Integer(-n))),
                    Some(Token::Dec(d)) => Ok(Expr::Literal(Literal::Decimal(-d))),
                    _ => unreachable!(),
                },
                // general polarity desugars to 0 - expr (empty propagates)
                _ => Ok(Expr::Binary {
                    op: BinOp::Sub,
                    lhs: Box::new(Expr::Literal(Literal::Integer(0))),
                    rhs: Box::new(self.parse_expr(10)?),
                }),
            },
            Some(Token::Plus) => self.parse_expr(10),
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
        Token::Xor => Ok("xor".into()),
        Token::Implies => Ok("implies".into()),
        Token::Div => Ok("div".into()),
        Token::Mod => Ok("mod".into()),
        Token::Contains => Ok("contains".into()),
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

    fn bin(op: BinOp, l: Expr, r: Expr) -> Expr {
        Expr::Binary {
            op,
            lhs: Box::new(l),
            rhs: Box::new(r),
        }
    }

    #[test]
    fn precedence() {
        // and binds tighter than or
        assert_eq!(
            parse("a or b and c").unwrap(),
            bin(BinOp::Or, id("a"), bin(BinOp::And, id("b"), id("c")))
        );
        // equality binds tighter than and
        assert_eq!(
            parse("a = b and c = d").unwrap(),
            bin(
                BinOp::And,
                bin(BinOp::Eq, id("a"), id("b")),
                bin(BinOp::Eq, id("c"), id("d"))
            )
        );
        // comparison binds tighter than equality; union tighter than comparison
        assert_eq!(
            parse("a | b < c").unwrap(),
            bin(BinOp::Lt, bin(BinOp::Union, id("a"), id("b")), id("c"))
        );
        // concat binds tighter than comparison
        assert_eq!(
            parse("a & b = c").unwrap(),
            bin(BinOp::Eq, bin(BinOp::Concat, id("a"), id("b")), id("c"))
        );
        // membership sits between and and equality
        assert_eq!(
            parse("a in b and c").unwrap(),
            bin(BinOp::And, bin(BinOp::In, id("a"), id("b")), id("c"))
        );
        // left associativity
        assert_eq!(
            parse("a | b | c").unwrap(),
            bin(BinOp::Union, bin(BinOp::Union, id("a"), id("b")), id("c"))
        );
    }

    #[test]
    fn arithmetic_precedence() {
        // multiplicative binds tighter than additive
        assert_eq!(
            parse("a + b * c").unwrap(),
            bin(BinOp::Add, id("a"), bin(BinOp::Mul, id("b"), id("c")))
        );
        // additive binds tighter than comparison
        assert_eq!(
            parse("a + b < c").unwrap(),
            bin(BinOp::Lt, bin(BinOp::Add, id("a"), id("b")), id("c"))
        );
        // unary minus on an expression desugars to 0 - expr
        assert_eq!(
            parse("-a").unwrap(),
            bin(BinOp::Sub, Expr::Literal(Literal::Integer(0)), id("a"))
        );
        // the literal fast-path still folds
        assert_eq!(parse("-3").unwrap(), Expr::Literal(Literal::Integer(-3)));
        // contains desugars to in with swapped operands
        assert_eq!(
            parse("a contains b").unwrap(),
            bin(BinOp::In, id("b"), id("a"))
        );
        // div/mod/contains are still valid member names
        assert_eq!(
            parse("text.div").unwrap(),
            Expr::Member {
                base: Box::new(id("text")),
                name: "div".into()
            }
        );
        // the string function form still parses as a call
        assert!(matches!(
            parse("name.contains('x')").unwrap(),
            Expr::Call { .. }
        ));
        assert_eq!(parse("a xor b").unwrap(), bin(BinOp::Xor, id("a"), id("b")));
        assert_eq!(parse("a ~ b").unwrap(), bin(BinOp::Equiv, id("a"), id("b")));
        // implies is the loosest binder
        assert_eq!(
            parse("a implies b or c").unwrap(),
            bin(BinOp::Implies, id("a"), bin(BinOp::Or, id("b"), id("c")))
        );
    }

    #[test]
    fn type_operators() {
        assert_eq!(
            parse("value is Quantity").unwrap(),
            Expr::TypeTest {
                op: TypeOp::Is,
                expr: Box::new(id("value")),
                type_name: "Quantity".into()
            }
        );
        assert_eq!(
            parse("(value as Quantity).unit").unwrap(),
            Expr::Member {
                base: Box::new(Expr::TypeTest {
                    op: TypeOp::As,
                    expr: Box::new(id("value")),
                    type_name: "Quantity".into()
                }),
                name: "unit".into()
            }
        );
    }
}
