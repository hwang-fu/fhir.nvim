use crate::error::Error;
use rust_decimal::Decimal;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Ident(String),
    Int(i64),
    Dec(Decimal),
    Str(String),
    Date(String),
    DateTime(String),
    True,
    False,
    And,
    Or,
    In,
    Is,
    As,
    Xor,
    Implies,
    Div,
    Mod,
    Contains,
    DollarThis,
    Dot,
    Comma,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Pipe,
    Amp,
    Plus,
    Star,
    Slash,
    Tilde,
    NotTilde,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Minus,
}

fn keyword(word: &str) -> Option<Token> {
    match word {
        "true" => Some(Token::True),
        "false" => Some(Token::False),
        "and" => Some(Token::And),
        "or" => Some(Token::Or),
        "in" => Some(Token::In),
        "is" => Some(Token::Is),
        "as" => Some(Token::As),
        "xor" => Some(Token::Xor),
        "implies" => Some(Token::Implies),
        "div" => Some(Token::Div),
        "mod" => Some(Token::Mod),
        "contains" => Some(Token::Contains),
        _ => None,
    }
}

pub fn tokenize(input: &str) -> Result<Vec<Token>, Error> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&c) = chars.peek() {
        match c {
            c if c.is_whitespace() => {
                chars.next();
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let mut word = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_alphanumeric() || c == '_' {
                        word.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(keyword(&word).unwrap_or(Token::Ident(word)));
            }
            '`' => {
                chars.next();
                let mut word = String::new();
                loop {
                    match chars.next() {
                        Some('`') => break,
                        Some(c) => word.push(c),
                        None => return Err(Error::Lex("unterminated identifier".into())),
                    }
                }
                tokens.push(Token::Ident(word));
            }
            c if c.is_ascii_digit() => {
                let mut num = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_digit() {
                        num.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                // a '.' continues the number only when a digit follows it
                let mut ahead = chars.clone();
                ahead.next();
                if chars.peek() == Some(&'.') && ahead.peek().is_some_and(|c| c.is_ascii_digit()) {
                    num.push('.');
                    chars.next();
                    while let Some(&c) = chars.peek() {
                        if c.is_ascii_digit() {
                            num.push(c);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    let d = num
                        .parse::<Decimal>()
                        .map_err(|e| Error::Lex(format!("bad decimal {num}: {e}")))?;
                    tokens.push(Token::Dec(d));
                } else {
                    let n = num
                        .parse::<i64>()
                        .map_err(|e| Error::Lex(format!("bad integer {num}: {e}")))?;
                    tokens.push(Token::Int(n));
                }
            }
            '\'' => {
                chars.next();
                let mut s = String::new();
                loop {
                    match chars.next() {
                        None => return Err(Error::Lex("unterminated string".into())),
                        Some('\'') => {
                            // doubled quote is a literal quote; anything else closes
                            if chars.peek() == Some(&'\'') {
                                s.push('\'');
                                chars.next();
                            } else {
                                break;
                            }
                        }
                        Some('\\') => match chars.next() {
                            Some('\'') => s.push('\''),
                            Some('\\') => s.push('\\'),
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some(c) => return Err(Error::Lex(format!("bad escape: \\{c}"))),
                            None => return Err(Error::Lex("unterminated string".into())),
                        },
                        Some(c) => s.push(c),
                    }
                }
                tokens.push(Token::Str(s));
            }
            '@' => {
                chars.next();
                let mut t = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_digit() || matches!(c, 'T' | 'Z' | ':' | '+' | '.' | '-') {
                        t.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if t.contains('T') {
                    tokens.push(Token::DateTime(t));
                } else {
                    tokens.push(Token::Date(t));
                }
            }
            '$' => {
                chars.next();
                let mut word = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_alphabetic() {
                        word.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if word == "this" {
                    tokens.push(Token::DollarThis);
                } else {
                    return Err(Error::Lex(format!("unexpected token: ${word}")));
                }
            }
            '/' => {
                chars.next();
                match chars.peek() {
                    Some('/') => {
                        while let Some(&c) = chars.peek() {
                            if c == '\n' {
                                break;
                            }
                            chars.next();
                        }
                    }
                    Some('*') => {
                        chars.next();
                        let mut prev = ' ';
                        loop {
                            match chars.next() {
                                None => return Err(Error::Lex("unterminated comment".into())),
                                Some('/') if prev == '*' => break,
                                Some(c) => prev = c,
                            }
                        }
                    }
                    _ => tokens.push(Token::Slash),
                }
            }
            '!' => {
                chars.next();
                match chars.next() {
                    Some('=') => tokens.push(Token::Ne),
                    Some('~') => tokens.push(Token::NotTilde),
                    _ => return Err(Error::Lex("expected = or ~ after !".into())),
                }
            }
            '<' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::Le);
                } else {
                    tokens.push(Token::Lt);
                }
            }
            '>' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::Ge);
                } else {
                    tokens.push(Token::Gt);
                }
            }
            '=' => {
                chars.next();
                tokens.push(Token::Eq);
            }
            '.' => {
                chars.next();
                tokens.push(Token::Dot);
            }
            ',' => {
                chars.next();
                tokens.push(Token::Comma);
            }
            '(' => {
                chars.next();
                tokens.push(Token::LParen);
            }
            ')' => {
                chars.next();
                tokens.push(Token::RParen);
            }
            '[' => {
                chars.next();
                tokens.push(Token::LBracket);
            }
            ']' => {
                chars.next();
                tokens.push(Token::RBracket);
            }
            '|' => {
                chars.next();
                tokens.push(Token::Pipe);
            }
            '&' => {
                chars.next();
                tokens.push(Token::Amp);
            }
            '-' => {
                chars.next();
                tokens.push(Token::Minus);
            }
            '+' => {
                chars.next();
                tokens.push(Token::Plus);
            }
            '*' => {
                chars.next();
                tokens.push(Token::Star);
            }
            '~' => {
                chars.next();
                tokens.push(Token::Tilde);
            }
            other => return Err(Error::Lex(format!("unexpected character: {other}"))),
        }
    }
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(s: &str) -> Vec<Token> {
        tokenize(s).unwrap()
    }

    #[test]
    fn identifiers_and_paths() {
        assert_eq!(
            lex("Patient.name"),
            vec![
                Token::Ident("Patient".into()),
                Token::Dot,
                Token::Ident("name".into())
            ]
        );
        assert_eq!(lex("`div`"), vec![Token::Ident("div".into())]);
    }

    #[test]
    fn literals() {
        assert_eq!(lex("42"), vec![Token::Int(42)]);
        assert_eq!(lex("3.14"), vec![Token::Dec("3.14".parse().unwrap())]);
        assert_eq!(lex("'it''s'"), vec![Token::Str("it's".into())]);
        assert_eq!(lex("'a\\'b'"), vec![Token::Str("a'b".into())]);
        assert_eq!(lex("true false"), vec![Token::True, Token::False]);
        assert_eq!(lex("@1974-12-25"), vec![Token::Date("1974-12-25".into())]);
        assert_eq!(
            lex("@2015-02-04T14:34:28Z"),
            vec![Token::DateTime("2015-02-04T14:34:28Z".into())]
        );
    }

    #[test]
    fn operators_and_punctuation() {
        assert_eq!(
            lex("= != < <= > >= | & - , ( ) [ ]"),
            vec![
                Token::Eq,
                Token::Ne,
                Token::Lt,
                Token::Le,
                Token::Gt,
                Token::Ge,
                Token::Pipe,
                Token::Amp,
                Token::Minus,
                Token::Comma,
                Token::LParen,
                Token::RParen,
                Token::LBracket,
                Token::RBracket
            ]
        );
        assert_eq!(
            lex("and or in is as $this"),
            vec![
                Token::And,
                Token::Or,
                Token::In,
                Token::Is,
                Token::As,
                Token::DollarThis
            ]
        );
    }

    #[test]
    fn comments_and_whitespace() {
        assert_eq!(lex("a // line\n.b"), lex("a.b"));
        assert_eq!(lex("a /* block */ .b"), lex("a.b"));
    }

    #[test]
    fn arithmetic_and_equivalence_tokens() {
        assert_eq!(
            lex("+ * / ~"),
            vec![Token::Plus, Token::Star, Token::Slash, Token::Tilde]
        );
        assert_eq!(lex("!~"), vec![Token::NotTilde]);
        assert_eq!(
            lex("xor implies div mod contains"),
            vec![
                Token::Xor,
                Token::Implies,
                Token::Div,
                Token::Mod,
                Token::Contains
            ]
        );
        // bare / still lexes; comments still skip
        assert_eq!(lex("4 / 2").len(), 3);
        assert_eq!(lex("a // c\n.b"), lex("a.b"));
    }

    #[test]
    fn lex_errors() {
        assert!(matches!(tokenize("'unterminated"), Err(Error::Lex(_))));
        assert!(matches!(tokenize("#"), Err(Error::Lex(_))));
    }
}
