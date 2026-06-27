use crate::token::{TokenStream, TokenType};
use anyhow::{Result, anyhow};
use std::collections::{HashMap, HashSet};

pub fn make_specifier_parser() -> SpecifierParser {
    create_specifier_parser(true)
}

pub fn make_type_specifier_parser() -> SpecifierParser {
    create_specifier_parser(false)
}

fn create_specifier_parser(include_storage_types: bool) -> SpecifierParser {
    let type_token_types: HashSet<TokenType> = HashSet::from_iter([
        TokenType::Int,
        TokenType::Long,
        TokenType::Double,
        TokenType::Unsigned,
        TokenType::Signed,
    ]);
    let mut allowed_token_types = Vec::from_iter(type_token_types.clone());
    if include_storage_types {
        allowed_token_types.push(TokenType::Extern);
        allowed_token_types.push(TokenType::Static);
    }

    let mut ret = SpecifierParser::new(allowed_token_types);

    ret.add_token_type_constraint(
        TokenType::Double,
        Box::new(ExcludeConstraint::new(
            "double cannot be combined with int, long, signed or unsigned".to_string(),
            vec![
                TokenType::Int,
                TokenType::Long,
                TokenType::Signed,
                TokenType::Unsigned,
            ]
            .into_iter()
            .collect(),
        )),
    );

    ret.add_token_type_constraint(
        TokenType::Signed,
        Box::new(ExcludeConstraint::new(
            "signed cannot be combined with unsigned".to_string(),
            vec![TokenType::Unsigned].into_iter().collect(),
        )),
    );

    if include_storage_types {
        ret.add_token_type_constraint(
            TokenType::Static,
            Box::new(ExcludeConstraint::new(
                "static cannot be combined with extern".to_string(),
                vec![TokenType::Extern].into_iter().collect(),
            )),
        );
    }

    ret.add_constraint(Box::new(RequiredConstraint::new(
        "at least one type specifier is required".to_string(),
        type_token_types,
    )));

    ret
}

pub struct SpecifierParser {
    allowed_token_types: HashSet<TokenType>,
    token_type_constraints: HashMap<TokenType, Vec<Box<dyn SpecifierConstraint>>>,
    constraints: Vec<Box<dyn SpecifierConstraint>>,
}

impl SpecifierParser {
    pub fn new(allowed_token_types: Vec<TokenType>) -> Self {
        Self {
            allowed_token_types: allowed_token_types.into_iter().collect(),
            token_type_constraints: HashMap::new(),
            constraints: Vec::new(),
        }
    }

    fn add_constraint(&mut self, constraint: Box<dyn SpecifierConstraint>) {
        self.constraints.push(constraint);
    }

    fn add_token_type_constraint(
        &mut self,
        token_type: TokenType,
        constraint: Box<dyn SpecifierConstraint>,
    ) {
        let entry = self
            .token_type_constraints
            .entry(token_type)
            .or_insert(vec![]);
        entry.push(constraint);
    }

    pub fn parse(&self, stream: &mut TokenStream) -> Result<HashSet<TokenType>> {
        let mut specifiers = HashSet::new();

        while let Some(token) = stream.peek() {
            let token_type = &token.token_type;
            if !self.allowed_token_types.contains(token_type) {
                break;
            }
            if specifiers.contains(token_type) {
                return Err(anyhow!("duplicate specifier {}", token.lexeme));
            }

            specifiers.insert(token.token_type.clone());
            stream.advance();
        }

        for token_type in &specifiers {
            if let Some(constraints) = self.token_type_constraints.get(token_type) {
                for constraint in constraints {
                    if !constraint.is_valid(&specifiers) {
                        return Err(anyhow!("{}", constraint.get_error_message()));
                    }
                }
            }
        }

        if !specifiers.is_empty() {
            for constraint in &self.constraints {
                if !constraint.is_valid(&specifiers) {
                    return Err(anyhow!("{}", constraint.get_error_message()));
                }
            }
        }

        Ok(specifiers)
    }
}

trait SpecifierConstraint {
    fn is_valid(&self, token_types: &HashSet<TokenType>) -> bool;
    fn get_error_message(&self) -> String;
}

struct ExcludeConstraint {
    message: String,
    excluded: HashSet<TokenType>,
}

impl ExcludeConstraint {
    fn new(message: String, excluded: HashSet<TokenType>) -> Self {
        Self { message, excluded }
    }
}

impl SpecifierConstraint for ExcludeConstraint {
    fn is_valid(&self, token_types: &HashSet<TokenType>) -> bool {
        self.excluded.is_disjoint(token_types)
    }

    fn get_error_message(&self) -> String {
        self.message.clone()
    }
}

struct RequiredConstraint {
    message: String,
    required: HashSet<TokenType>,
}

impl RequiredConstraint {
    fn new(message: String, required: HashSet<TokenType>) -> Self {
        Self { message, required }
    }
}

impl SpecifierConstraint for RequiredConstraint {
    fn is_valid(&self, token_types: &HashSet<TokenType>) -> bool {
        !self.required.is_disjoint(token_types)
    }

    fn get_error_message(&self) -> String {
        self.message.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::token::{TokenStream, TokenType};

    fn make_token_stream(code: &str) -> TokenStream {
        let lexer = Lexer::new();
        let tokens = lexer.scan_tokens(code).unwrap();
        TokenStream::new(tokens)
    }

    #[test]
    fn test_specifier_parser_unsigned_int() {
        let code = r"static unsigned int";
        let mut stream = make_token_stream(code);

        let parser = make_specifier_parser();

        let specifiers = parser.parse(&mut stream).unwrap();

        assert!(specifiers.contains(&TokenType::Unsigned));
        assert!(specifiers.contains(&TokenType::Int));
    }

    #[test]
    fn test_specifier_parser_static_double() {
        let code = r"static double";
        let mut stream = make_token_stream(code);

        let parser = make_specifier_parser();

        let specifiers = parser.parse(&mut stream).unwrap();

        assert!(specifiers.contains(&TokenType::Static));
        assert!(specifiers.contains(&TokenType::Double));
    }

    #[test]
    fn test_specifier_parser_double_int_error() {
        let code = r"double int";
        let mut stream = make_token_stream(code);

        let parser = make_specifier_parser();

        parser
            .parse(&mut stream)
            .expect_err("Expected specifier error");
    }

    #[test]
    fn test_specifier_parser_extern_static_error() {
        let code = r"extern static";
        let mut stream = make_token_stream(code);

        let parser = make_specifier_parser();

        parser
            .parse(&mut stream)
            .expect_err("Expected specifier error");
    }

    #[test]
    fn no_types_error() {
        let code = r"static";
        let mut stream = make_token_stream(code);

        let parser = make_specifier_parser();

        parser
            .parse(&mut stream)
            .expect_err("Expected specifier error");
    }
}
