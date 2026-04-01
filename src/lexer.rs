use std::collections::HashMap;
use crate::token::{Token, TokenType, TokenValue};
use anyhow::Result;
use regex::Regex;

pub type ValueFn = fn(lexeme: &str) -> TokenValue;

#[derive(Debug, Clone)]
struct TokenTypeData {
    pub token_type: TokenType,
    pub regex: Regex,
    pub skip: bool,
    pub value_fn_opt: Option<ValueFn>,
}

pub struct Lexer {
    token_types: Vec<TokenTypeData>,
    keywords: HashMap<String, TokenType>,
}

impl Lexer {
    pub fn new() -> Lexer {
        let mut lexer = Lexer {
            token_types: Vec::new(),
            keywords: HashMap::new(),
        };

        lexer.add_token_type_full(TokenType::Whitespace, r"^\s+", true, None);

        lexer.add_token_type_full(
            TokenType::Identifier,
            r"^[a-zA-Z_][a-zA-Z0-9_]*\b",
            false,
            None,
        );

        lexer.add_token_type_full(
            TokenType::IntegerConstant,
            r"^\d+\b",
            false,
            Some(|lexeme| {
                let value = lexeme.parse::<i32>().unwrap();
                TokenValue::Integer(value)
            }),
        );

        lexer.add_token_type(TokenType::LeftParen, r"^\(");
        lexer.add_token_type(TokenType::RightParen, r"^\)");
        lexer.add_token_type(TokenType::LeftBrace, r"^\{");
        lexer.add_token_type(TokenType::RightBrace, r"^\}");
        lexer.add_token_type(TokenType::Comma, r"^,");
        lexer.add_token_type(TokenType::Semicolon, r"^;");
        lexer.add_token_type(TokenType::Minus, r"^\-");
        lexer.add_token_type(TokenType::MinusMinus, r"^\-\-");
        lexer.add_token_type(TokenType::Tilde, r"^~");

        lexer.keywords.insert("int".to_string(), TokenType::Int);
        lexer.keywords.insert("void".to_string(), TokenType::Void);
        lexer.keywords.insert("return".to_string(), TokenType::Return);

        lexer
    }

    pub fn scan_tokens(&self, code: &str) -> Result<Vec<Token>> {
        let mut tokens: Vec<Token> = Vec::new();
        let mut line: usize = 1;
        let mut column: usize = 1;
        let mut remaining = code.to_string();

        while !remaining.is_empty() {
            match self.find_max_match(&remaining) {
                Some((token_type, lexeme, skip, value_opt)) => {
                    let curr_line = line;
                    let curr_column = column;

                    (line, column) = Self::advance_position(&lexeme, line, column);
                    // remove the matched lexeme from the remaining code
                    remaining = remaining[lexeme.len()..].to_string();

                    if !skip {
                        let token_type = if token_type == TokenType::Identifier {
                            self.keywords.get(&lexeme).cloned().unwrap_or(TokenType::Identifier)
                        } else {
                            token_type
                        };

                        let token =
                            Token::new(token_type, value_opt, lexeme, curr_line, curr_column);
                        tokens.push(token);
                    }
                }
                None => {
                    return Err(anyhow::anyhow!(
                        "Unexpected token at line {}, column {}: '{}'",
                        line,
                        column,
                        remaining.chars().next().unwrap()
                    ));
                }
            }
        }

        Ok(tokens)
    }

    fn add_token_type(&mut self, token_type: TokenType, pattern: &str) {
        self.add_token_type_full(token_type, pattern, false, None);
    }

    fn add_token_type_full(
        &mut self,
        token_type: TokenType,
        pattern: &str,
        skip: bool,
        value_fn_opt: Option<ValueFn>,
    ) {
        let token_type_data = TokenTypeData {
            token_type,
            regex: Regex::new(pattern).unwrap(),
            skip,
            value_fn_opt,
        };
        self.token_types.push(token_type_data);
    }

    fn advance_position(lexeme: &str, line: usize, column: usize) -> (usize, usize) {
        let mut new_line = line;
        let mut new_column = column;

        for ch in lexeme.chars() {
            if ch == '\n' {
                new_line += 1;
                new_column = 1;
            } else {
                new_column += 1;
            }
        }

        (new_line, new_column)
    }

    fn find_max_match(&self, code: &str) -> Option<(TokenType, String, bool, Option<TokenValue>)> {
        let mut max_match: Option<(TokenType, String, bool, Option<TokenValue>)> = None;

        for token_type_data in &self.token_types {
            if let Some(mat) = token_type_data.regex.find(code) {
                if mat.start() == 0 {
                    let matched_str = mat.as_str().to_string();
                    if max_match.is_none()
                        || matched_str.len() > max_match.as_ref().unwrap().1.len()
                    {
                        let value_opt = token_type_data
                            .value_fn_opt
                            .map(|value_fn| value_fn(&matched_str));
                        max_match = Some((
                            token_type_data.token_type.clone(),
                            matched_str,
                            token_type_data.skip,
                            value_opt,
                        ));
                    }
                }
            }
        }

        max_match
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn scan_tokens() {
        let lexer = Lexer::new();
        let code = r#"
            answer
            42
        "#;

        let tokens = lexer.scan_tokens(code).unwrap();

        assert_eq!(tokens.len(), 2);

        assert_eq!(tokens[0].token_type, TokenType::Identifier);
        assert_eq!(tokens[0].lexeme, "answer");
        assert_eq!(tokens[0].line, 2);
        assert_eq!(tokens[0].column, 13);

        assert_eq!(tokens[1].token_type, TokenType::IntegerConstant);
        assert_eq!(tokens[1].value, Some(TokenValue::Integer(42)));
        assert_eq!(tokens[1].lexeme, "42");
        assert_eq!(tokens[1].line, 3);
        assert_eq!(tokens[1].column, 13);
    }

    #[test]
    fn scan_main_function() {
        let lexer = Lexer::new();
        let code = r#"
int main(void) {
    return 42;
}"#;

        let tokens = lexer.scan_tokens(code).unwrap();
        assert_eq!(tokens.len(), 10);

        //dbg!(tokens);
    }

    #[test]
    fn scan_tilde_decrement() {
        let lexer = Lexer::new();
        let code = r#"
        ~a--;
        "#;

        let tokens = lexer.scan_tokens(code).unwrap();
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].token_type, TokenType::Tilde);
        assert_eq!(tokens[0].lexeme, "~");
        assert_eq!(tokens[0].line, 2);
        assert_eq!(tokens[0].column, 9);

        assert_eq!(tokens[1].token_type, TokenType::Identifier);
        assert_eq!(tokens[1].lexeme, "a");
        assert_eq!(tokens[1].line, 2);
        assert_eq!(tokens[1].column, 10);

        assert_eq!(tokens[2].token_type, TokenType::MinusMinus);
        assert_eq!(tokens[2].lexeme, "--");
        assert_eq!(tokens[2].line, 2);
        assert_eq!(tokens[2].column, 11);

        assert_eq!(tokens[3].token_type, TokenType::Semicolon);
        assert_eq!(tokens[3].lexeme, ";");
        assert_eq!(tokens[3].line, 2);
        assert_eq!(tokens[3].column, 13);
    }
}
