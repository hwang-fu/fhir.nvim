mod ast;
mod error;
mod eval;
mod functions;
mod lexer;
mod parser;
mod value;

pub use error::Error;

pub fn ping() -> &'static str {
    "hello from fhir-core"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_returns_greeting() {
        assert_eq!(ping(), "hello from fhir-core");
    }
}
