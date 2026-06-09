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
