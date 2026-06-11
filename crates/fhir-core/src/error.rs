use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    Lex(String),
    Parse(String),
    Eval(String),
    Validate(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Lex(m) => write!(f, "lex error: {m}"),
            Error::Parse(m) => write!(f, "parse error: {m}"),
            Error::Eval(m) => write!(f, "eval error: {m}"),
            Error::Validate(m) => write!(f, "validate error: {m}"),
        }
    }
}

impl std::error::Error for Error {}
