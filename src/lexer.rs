use crate::token::{Token, TokenType, TokenValue};
use anyhow::Result;
use fancy_regex::Regex;
use std::collections::HashMap;

trait TokenDerivation {
    fn derive_token_type(&self, lexeme: &str) -> Result<TokenType>;
    fn derive_token_value(&self, lexeme: &str) -> Result<Option<TokenValue>>;
}

struct ConstantTokenDerivation {
    token_type: TokenType,
}

impl ConstantTokenDerivation {
    fn new(token_type: TokenType) -> ConstantTokenDerivation {
        ConstantTokenDerivation { token_type }
    }
}

impl TokenDerivation for ConstantTokenDerivation {
    fn derive_token_type(&self, _: &str) -> Result<TokenType> {
        Ok(self.token_type.clone())
    }

    fn derive_token_value(&self, _: &str) -> Result<Option<TokenValue>> {
        Ok(None)
    }
}

impl Into<Box<dyn TokenDerivation>> for TokenType {
    fn into(self) -> Box<dyn TokenDerivation> {
        Box::new(ConstantTokenDerivation::new(self))
    }
}

struct NumberTokenDerivation {
    combined_pattern: String,
    regex_map: HashMap<TokenType, Regex>,
}

impl NumberTokenDerivation {
    fn new() -> Self {
        let patterns = HashMap::from([
            (TokenType::IntegerConstant, r"^\d+"),
            (TokenType::UnsignedIntegerConstant, r"\d+[uU]"),
            (TokenType::LongConstant, r"\d+[lL]"),
            (TokenType::UnsignedLongConstant, r"\d+([uU][lL]|[lL][uU])"),
            (
                TokenType::DoubleConstant,
                r"((\d*\.\d+|\d+\.?)[eE][+-]?\d+|\d*\.\d+|\d+\.)",
            ),
        ]);
        let regex_map: HashMap<TokenType, Regex> = patterns
            .iter()
            .map(|(token_type, pattern)| {
                let regex = Regex::new(pattern).unwrap();
                (token_type.clone(), regex)
            })
            .collect();

        let combined_pattern = format!(
            r"({})(?![\w\.])",
            patterns.values().cloned().collect::<Vec<&str>>().join("|")
        );

        Self {
            combined_pattern,
            regex_map,
        }
    }

    pub fn get_combined_pattern(&self) -> String {
        self.combined_pattern.to_string()
    }

    fn verify_number_token_type(lexeme: &str, token_type: &TokenType) -> Result<TokenType> {
        match token_type {
            TokenType::IntegerConstant => {
                if lexeme.parse::<i32>().is_ok() {
                    Ok(TokenType::IntegerConstant)
                } else {
                    Self::verify_unsigned(lexeme)
                }
            }
            TokenType::UnsignedIntegerConstant => {
                let lexeme = &lexeme[0..lexeme.len() - 1]; // Remove the trailing 'u' or 'U'
                Self::verify_unsigned(lexeme)
            }
            TokenType::LongConstant => {
                let lexeme = &lexeme[0..lexeme.len() - 1]; // Remove the trailing 'l' or 'L'
                Self::verify_long(lexeme)
            }
            TokenType::UnsignedLongConstant => {
                let lexeme = &lexeme[0..lexeme.len() - 2]; // Remove the trailing 'ul', 'uL', 'Ul', or 'UL'
                Self::verify_unsigned_long(lexeme)
            }
            _ => Ok(token_type.clone()),
        }
    }

    fn verify_unsigned(lexeme: &str) -> Result<TokenType> {
        if lexeme.parse::<u32>().is_ok() {
            Ok(TokenType::UnsignedIntegerConstant)
        } else {
            Self::verify_long(lexeme)
        }
    }

    fn verify_long(lexeme: &str) -> Result<TokenType> {
        if lexeme.parse::<i64>().is_ok() {
            Ok(TokenType::LongConstant)
        } else {
            Self::verify_unsigned_long(lexeme)
        }
    }

    fn verify_unsigned_long(lexeme: &str) -> Result<TokenType> {
        if lexeme.parse::<u64>().is_ok() {
            Ok(TokenType::UnsignedLongConstant)
        } else {
            Err(anyhow::anyhow!("{} is not a valid number literal", lexeme))
        }
    }
}

impl TokenDerivation for NumberTokenDerivation {
    fn derive_token_type(&self, lexeme: &str) -> Result<TokenType> {
        let mut max_match_length = 0;
        let mut max_match_type_opt: Option<TokenType> = None;

        for (token_type, regex) in &self.regex_map {
            if let Ok(Some(regex_match)) = regex.find(lexeme) {
                let matched_len = regex_match.as_str().len();
                if matched_len > max_match_length {
                    max_match_length = matched_len;
                    max_match_type_opt = Some(token_type.clone());
                }
            }
        }

        match max_match_type_opt {
            Some(token_type) => match Self::verify_number_token_type(lexeme, &token_type) {
                Ok(token_type) => Ok(token_type),
                Err(e) => Err(e),
            },
            None => Err(anyhow::anyhow!("{} is not a valid number literal", lexeme)),
        }
    }

    fn derive_token_value(&self, lexeme: &str) -> Result<Option<TokenValue>> {
        let token_type = self.derive_token_type(lexeme)?;
        let lexeme = lexeme.trim_end_matches(|c| c == 'u' || c == 'U' || c == 'l' || c == 'L');

        let value = match token_type {
            TokenType::IntegerConstant => Some(TokenValue::Integer(lexeme.parse::<i32>()?)),
            TokenType::UnsignedIntegerConstant => {
                Some(TokenValue::UnsignedInteger(lexeme.parse::<u32>()?))
            }

            TokenType::LongConstant => Some(TokenValue::Long(lexeme.parse::<i64>()?)),
            TokenType::UnsignedLongConstant => {
                Some(TokenValue::UnsignedLong(lexeme.parse::<u64>()?))
            }
            TokenType::DoubleConstant => Some(TokenValue::Double(lexeme.parse::<f64>()?)),
            _ => return Err(anyhow::anyhow!("{} is not a valid number literal", lexeme)),
        };

        Ok(value)
    }
}

impl Into<Box<dyn TokenDerivation>> for NumberTokenDerivation {
    fn into(self) -> Box<dyn TokenDerivation> {
        Box::new(self)
    }
}

struct TokenTypeData {
    pub token_derivation: Box<dyn TokenDerivation>,
    pub regex: Regex,
    pub skip: bool,
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

        lexer.add_token_rule_full(TokenType::Whitespace, r"\s+", true, false);
        lexer.add_token_rule_full(TokenType::LineComment, r"//.*", true, false);

        let number_token_derivation = NumberTokenDerivation::new();
        let pattern = number_token_derivation.get_combined_pattern();
        lexer.add_token_rule(number_token_derivation, &pattern);

        lexer.add_token_rule(TokenType::Identifier, r"[a-zA-Z_][a-zA-Z0-9_]*\b");
        lexer.add_token_rule(TokenType::LeftParen, r"\(");
        lexer.add_token_rule(TokenType::RightParen, r"\)");
        lexer.add_token_rule(TokenType::LeftBrace, r"\{");
        lexer.add_token_rule(TokenType::RightBrace, r"\}");
        lexer.add_token_rule(TokenType::Comma, r",");
        lexer.add_token_rule(TokenType::Semicolon, r";");
        lexer.add_token_rule(TokenType::Minus, r"\-");
        lexer.add_token_rule(TokenType::Tilde, r"~");
        lexer.add_token_rule(TokenType::Plus, r"\+");
        lexer.add_token_rule(TokenType::Asterisk, r"\*");
        lexer.add_token_rule(TokenType::Slash, r"/");
        lexer.add_token_rule(TokenType::Percent, r"%");
        lexer.add_token_rule(TokenType::BitAnd, r"&");
        lexer.add_token_rule(TokenType::BitOr, r"\|");
        lexer.add_token_rule(TokenType::BitXor, r"\^");
        lexer.add_token_rule(TokenType::ShiftLeft, r"<<");
        lexer.add_token_rule(TokenType::ShiftRight, r">>");
        lexer.add_token_rule(TokenType::LogicalAnd, r"&&");
        lexer.add_token_rule(TokenType::LogicalOr, r"\|\|");
        lexer.add_token_rule(TokenType::LogicalNot, r"!");
        lexer.add_token_rule(TokenType::Equal, r"==");
        lexer.add_token_rule(TokenType::NotEqual, r"!=");
        lexer.add_token_rule(TokenType::GreaterEqual, r">=");
        lexer.add_token_rule(TokenType::Greater, r">");
        lexer.add_token_rule(TokenType::Less, r"<");
        lexer.add_token_rule(TokenType::LessEqual, r"<=");
        lexer.add_token_rule(TokenType::Assign, r"=");
        lexer.add_token_rule(TokenType::AssignAdd, r"\+=");
        lexer.add_token_rule(TokenType::AssignSub, r"\-=");
        lexer.add_token_rule(TokenType::AssignMul, r"\*=");
        lexer.add_token_rule(TokenType::AssignDiv, r"/=");
        lexer.add_token_rule(TokenType::AssignRemainder, r"%=");
        lexer.add_token_rule(TokenType::AssignBitAnd, r"&=");
        lexer.add_token_rule(TokenType::AssignBitOr, r"\|=");
        lexer.add_token_rule(TokenType::AssignBitXor, r"\^=");
        lexer.add_token_rule(TokenType::AssignShiftLeft, r"<<=");
        lexer.add_token_rule(TokenType::AssignShiftRight, r">>=");
        lexer.add_token_rule(TokenType::IncrementPrefix, r"\+\+(?=[\w\(])");
        lexer.add_token_rule(TokenType::IncrementPostfix, r"\+\+(?![\w\(])");
        lexer.add_token_rule(TokenType::DecrementPrefix, r"\-\-(?=[\w\(])");
        lexer.add_token_rule(TokenType::DecrementPostfix, r"\-\-(?![\w\(])");
        lexer.add_token_rule(TokenType::QuestionMark, r"\?");
        lexer.add_token_rule(TokenType::Colon, r":");

        lexer.add_keyword("int", TokenType::Int);
        lexer.add_keyword("long", TokenType::Long);
        lexer.add_keyword("signed", TokenType::Signed);
        lexer.add_keyword("unsigned", TokenType::Unsigned);
        lexer.add_keyword("double", TokenType::Double);
        lexer.add_keyword("void", TokenType::Void);
        lexer.add_keyword("return", TokenType::Return);
        lexer.add_keyword("if", TokenType::If);
        lexer.add_keyword("else", TokenType::Else);
        lexer.add_keyword("goto", TokenType::Goto);
        lexer.add_keyword("do", TokenType::Do);
        lexer.add_keyword("while", TokenType::While);
        lexer.add_keyword("for", TokenType::For);
        lexer.add_keyword("break", TokenType::Break);
        lexer.add_keyword("continue", TokenType::Continue);
        lexer.add_keyword("switch", TokenType::Switch);
        lexer.add_keyword("case", TokenType::Case);
        lexer.add_keyword("default", TokenType::Default);
        lexer.add_keyword("static", TokenType::Static);
        lexer.add_keyword("extern", TokenType::Extern);

        lexer
    }

    pub fn scan_tokens(&self, code: &str) -> Result<Vec<Token>> {
        let mut tokens: Vec<Token> = Vec::new();
        let mut line: usize = 1;
        let mut column: usize = 1;
        let mut remaining = code.to_string();

        while !remaining.is_empty() {
            (line, column) = self.remove_block_comment(line, column, &mut remaining);
            if remaining.is_empty() {
                break;
            }

            match self.find_max_match(&remaining) {
                Some((token_type, lexeme, skip, value_opt)) => {
                    let curr_line = line;
                    let curr_column = column;

                    (line, column) = Self::advance_position(&lexeme, line, column);
                    // remove the matched lexeme from the remaining code
                    remaining = remaining[lexeme.len()..].to_string();

                    if !skip {
                        let token_type = if token_type == TokenType::Identifier {
                            self.keywords
                                .get(&lexeme)
                                .cloned()
                                .unwrap_or(TokenType::Identifier)
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

    fn remove_block_comment(
        &self,
        line: usize,
        column: usize,
        remaining: &mut String,
    ) -> (usize, usize) {
        if !remaining.starts_with("/*") {
            return (line, column);
        }

        // Find "*/":
        let mut block_comment: Option<String> = None;

        remaining.find("*/").map(|end| {
            // Remove the block comment
            block_comment = Some(remaining[..end + 2].to_string());
            *remaining = remaining[end + 2..].to_string();
        });

        if block_comment.is_none() {
            block_comment = Some(remaining.clone());
            *remaining = "".to_string();
        }

        let block_comment = block_comment.unwrap();
        Self::advance_position(&block_comment, line, column)
    }

    fn add_token_rule<T: Into<Box<dyn TokenDerivation>>>(
        &mut self,
        token_derivation: T,
        pattern: &str,
    ) {
        self.add_token_rule_full(token_derivation, pattern, false, false);
    }

    fn add_token_rule_full<T: Into<Box<dyn TokenDerivation>>>(
        &mut self,
        token_derivation: T,
        pattern: &str,
        skip: bool,
        multi_line: bool,
    ) {
        let mut pattern = if pattern.starts_with("^") {
            pattern.to_string()
        } else {
            format!("^{}", pattern)
        };

        if multi_line {
            pattern = "(?ms)".to_string() + &pattern;
        }

        let regex = Regex::new(&pattern).unwrap();

        let token_type_data = TokenTypeData {
            token_derivation: token_derivation.into(),
            regex,
            skip,
        };
        self.token_types.push(token_type_data);
    }

    fn add_keyword(&mut self, keyword: &str, token_type: TokenType) {
        self.keywords.insert(keyword.to_string(), token_type);
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
            if let Ok(Some(mat)) = token_type_data.regex.find(code) {
                if mat.start() == 0 {
                    let matched_str = mat.as_str().to_string();
                    if max_match.is_none()
                        || matched_str.len() > max_match.as_ref().unwrap().1.len()
                    {
                        let token_type = match token_type_data
                            .token_derivation
                            .derive_token_type(&matched_str)
                        {
                            Ok(token_type) => token_type,
                            Err(_) => continue,
                        };
                        let value_opt = match token_type_data
                            .token_derivation
                            .derive_token_value(&matched_str)
                        {
                            Ok(value_opt) => value_opt,
                            Err(_) => continue,
                        };

                        max_match =
                            Some((token_type, matched_str, token_type_data.skip, value_opt));
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
    fn scan_number_constants() {
        let lexer = Lexer::new();
        let code = r#"
           42
           42l
           42L
           42u
           42U
           42ul
           42uL
           42Ul
           42UL
           42lu
           42lU
           42Lu
           42LU
           4.2E1
        "#;

        let tokens = lexer.scan_tokens(code).unwrap();
        assert_eq!(tokens.len(), 14);

        assert_eq!(tokens[0].token_type, TokenType::IntegerConstant);
        assert_eq!(tokens[0].value, Some(TokenValue::Integer(42)));

        for i in 1..=2 {
            assert_eq!(tokens[i].token_type, TokenType::LongConstant);
            assert_eq!(tokens[i].value, Some(TokenValue::Long(42)));
        }

        for i in 3..=4 {
            assert_eq!(tokens[i].token_type, TokenType::UnsignedIntegerConstant);
            assert_eq!(tokens[i].value, Some(TokenValue::UnsignedInteger(42)));
        }

        for i in 5..=12 {
            assert_eq!(tokens[i].token_type, TokenType::UnsignedLongConstant);
            assert_eq!(tokens[i].value, Some(TokenValue::UnsignedLong(42)));
        }

        assert_eq!(tokens[13].token_type, TokenType::DoubleConstant);
        assert_eq!(tokens[13].value, Some(TokenValue::Double(42.0)));
    }

    #[test]
    fn scan_main_function() {
        let lexer = Lexer::new();
        let code = r#"
        int main(void) {
            return 42;
        }
        "#;

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

        assert_eq!(tokens[2].token_type, TokenType::DecrementPostfix);
        assert_eq!(tokens[2].lexeme, "--");
        assert_eq!(tokens[2].line, 2);
        assert_eq!(tokens[2].column, 11);

        assert_eq!(tokens[3].token_type, TokenType::Semicolon);
        assert_eq!(tokens[3].lexeme, ";");
        assert_eq!(tokens[3].line, 2);
        assert_eq!(tokens[3].column, 13);
    }

    #[test]
    fn scan_binary_operators() {
        let lexer = Lexer::new();
        let code = "a + b - c*d /e % f";

        let tokens = lexer.scan_tokens(code).unwrap();

        assert_eq!(tokens.len(), 11);
        assert_eq!(tokens[0].token_type, TokenType::Identifier);
        assert_eq!(tokens[0].lexeme, "a");
        assert_eq!(tokens[1].token_type, TokenType::Plus);
        assert_eq!(tokens[1].lexeme, "+");
        assert_eq!(tokens[2].token_type, TokenType::Identifier);
        assert_eq!(tokens[2].lexeme, "b");
        assert_eq!(tokens[3].token_type, TokenType::Minus);
        assert_eq!(tokens[3].lexeme, "-");
        assert_eq!(tokens[4].token_type, TokenType::Identifier);
        assert_eq!(tokens[4].lexeme, "c");
        assert_eq!(tokens[5].token_type, TokenType::Asterisk);
        assert_eq!(tokens[5].lexeme, "*");
        assert_eq!(tokens[6].token_type, TokenType::Identifier);
        assert_eq!(tokens[6].lexeme, "d");
        assert_eq!(tokens[7].token_type, TokenType::Slash);
        assert_eq!(tokens[7].lexeme, "/");
        assert_eq!(tokens[8].token_type, TokenType::Identifier);
        assert_eq!(tokens[8].lexeme, "e");
        assert_eq!(tokens[9].token_type, TokenType::Percent);
        assert_eq!(tokens[9].lexeme, "%");
        assert_eq!(tokens[10].token_type, TokenType::Identifier);
        assert_eq!(tokens[10].lexeme, "f");
    }

    #[test]
    fn scan_bitwise_operators() {
        let lexer = Lexer::new();
        let code = "a & b | c ^ d << e >> f";

        let tokens = lexer.scan_tokens(code).unwrap();

        assert_eq!(tokens.len(), 11);
        assert_eq!(tokens[0].token_type, TokenType::Identifier);
        assert_eq!(tokens[0].lexeme, "a");
        assert_eq!(tokens[1].token_type, TokenType::BitAnd);
        assert_eq!(tokens[1].lexeme, "&");
        assert_eq!(tokens[2].token_type, TokenType::Identifier);
        assert_eq!(tokens[2].lexeme, "b");
        assert_eq!(tokens[3].token_type, TokenType::BitOr);
        assert_eq!(tokens[3].lexeme, "|");
        assert_eq!(tokens[4].token_type, TokenType::Identifier);
        assert_eq!(tokens[4].lexeme, "c");
        assert_eq!(tokens[5].token_type, TokenType::BitXor);
        assert_eq!(tokens[5].lexeme, "^");
        assert_eq!(tokens[6].token_type, TokenType::Identifier);
        assert_eq!(tokens[6].lexeme, "d");
        assert_eq!(tokens[7].token_type, TokenType::ShiftLeft);
        assert_eq!(tokens[7].lexeme, "<<");
        assert_eq!(tokens[8].token_type, TokenType::Identifier);
        assert_eq!(tokens[8].lexeme, "e");
        assert_eq!(tokens[9].token_type, TokenType::ShiftRight);
        assert_eq!(tokens[9].lexeme, ">>");
        assert_eq!(tokens[10].token_type, TokenType::Identifier);
        assert_eq!(tokens[10].lexeme, "f");
    }

    #[test]
    fn scan_logical_operators() {
        let lexer = Lexer::new();
        let code = "a && b || !c";

        let tokens = lexer.scan_tokens(code).unwrap();

        assert_eq!(tokens.len(), 6);
        assert_eq!(tokens[0].token_type, TokenType::Identifier);
        assert_eq!(tokens[0].lexeme, "a");
        assert_eq!(tokens[1].token_type, TokenType::LogicalAnd);
        assert_eq!(tokens[1].lexeme, "&&");
        assert_eq!(tokens[2].token_type, TokenType::Identifier);
        assert_eq!(tokens[2].lexeme, "b");
        assert_eq!(tokens[3].token_type, TokenType::LogicalOr);
        assert_eq!(tokens[3].lexeme, "||");
        assert_eq!(tokens[4].token_type, TokenType::LogicalNot);
        assert_eq!(tokens[4].lexeme, "!");
        assert_eq!(tokens[5].token_type, TokenType::Identifier);
        assert_eq!(tokens[5].lexeme, "c");
    }

    #[test]
    fn scan_assignment() {
        let lexer = Lexer::new();
        let code = "a = b";

        let tokens = lexer.scan_tokens(code).unwrap();
        assert_eq!(tokens.len(), 3);

        assert_eq!(tokens[0].token_type, TokenType::Identifier);
        assert_eq!(tokens[0].lexeme, "a");
        assert_eq!(tokens[1].token_type, TokenType::Assign);
        assert_eq!(tokens[1].lexeme, "=");
        assert_eq!(tokens[2].token_type, TokenType::Identifier);
        assert_eq!(tokens[2].lexeme, "b");
    }

    #[test]
    fn scan_increment_decrement() {
        let lexer = Lexer::new();
        let code = "++a; a++; --b; b--;";

        let tokens = lexer.scan_tokens(code).unwrap();
        assert_eq!(tokens.len(), 12);

        assert_eq!(tokens[0].token_type, TokenType::IncrementPrefix);
        assert_eq!(tokens[0].lexeme, "++");
        assert_eq!(tokens[1].token_type, TokenType::Identifier);
        assert_eq!(tokens[1].lexeme, "a");
        assert_eq!(tokens[2].token_type, TokenType::Semicolon);
        assert_eq!(tokens[2].lexeme, ";");

        assert_eq!(tokens[3].token_type, TokenType::Identifier);
        assert_eq!(tokens[3].lexeme, "a");
        assert_eq!(tokens[4].token_type, TokenType::IncrementPostfix);
        assert_eq!(tokens[4].lexeme, "++");
        assert_eq!(tokens[5].token_type, TokenType::Semicolon);
        assert_eq!(tokens[5].lexeme, ";");

        assert_eq!(tokens[6].token_type, TokenType::DecrementPrefix);
        assert_eq!(tokens[6].lexeme, "--");
        assert_eq!(tokens[7].token_type, TokenType::Identifier);
        assert_eq!(tokens[7].lexeme, "b");
        assert_eq!(tokens[8].token_type, TokenType::Semicolon);
        assert_eq!(tokens[8].lexeme, ";");

        assert_eq!(tokens[9].token_type, TokenType::Identifier);
        assert_eq!(tokens[9].lexeme, "b");
        assert_eq!(tokens[10].token_type, TokenType::DecrementPostfix);
        assert_eq!(tokens[10].lexeme, "--");
        assert_eq!(tokens[11].token_type, TokenType::Semicolon);
        assert_eq!(tokens[11].lexeme, ";");
    }

    #[test]
    fn scan_compound_assignments() {
        let lexer = Lexer::new();
        let code =
            "a += b; c -= d; e *= f; g /= h; i %= j; k &= l; m |= n; o ^= p; q <<= r; s >>= t;";

        let tokens = lexer.scan_tokens(code).unwrap();

        // Count assignment operators: 10 assignments, each with 3 tokens (identifier, operator, identifier) plus semicolons = 30 tokens
        assert_eq!(tokens.len(), 40);

        // Test += (plus equals)
        assert_eq!(tokens[1].token_type, TokenType::AssignAdd);
        assert_eq!(tokens[1].lexeme, "+=");

        // Test -= (minus equals)
        assert_eq!(tokens[5].token_type, TokenType::AssignSub);
        assert_eq!(tokens[5].lexeme, "-=");

        // Test *= (multiply equals)
        assert_eq!(tokens[9].token_type, TokenType::AssignMul);
        assert_eq!(tokens[9].lexeme, "*=");

        // Test /= (divide equals)
        assert_eq!(tokens[13].token_type, TokenType::AssignDiv);
        assert_eq!(tokens[13].lexeme, "/=");

        // Test %= (remainder equals)
        assert_eq!(tokens[17].token_type, TokenType::AssignRemainder);
        assert_eq!(tokens[17].lexeme, "%=");

        // Test &= (bitwise and equals)
        assert_eq!(tokens[21].token_type, TokenType::AssignBitAnd);
        assert_eq!(tokens[21].lexeme, "&=");

        // Test |= (bitwise or equals)
        assert_eq!(tokens[25].token_type, TokenType::AssignBitOr);
        assert_eq!(tokens[25].lexeme, "|=");

        // Test ^= (bitwise xor equals)
        assert_eq!(tokens[29].token_type, TokenType::AssignBitXor);
        assert_eq!(tokens[29].lexeme, "^=");

        // Test <<= (shift left equals)
        assert_eq!(tokens[33].token_type, TokenType::AssignShiftLeft);
        assert_eq!(tokens[33].lexeme, "<<=");

        // Test >>= (shift right equals)
        assert_eq!(tokens[37].token_type, TokenType::AssignShiftRight);
        assert_eq!(tokens[37].lexeme, ">>=");
    }

    #[test]
    fn scan_if_else_question_mark_colon() {
        let lexer = Lexer::new();
        let code = "if else ?:";

        let tokens = lexer.scan_tokens(code).unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].token_type, TokenType::If);
        assert_eq!(tokens[1].token_type, TokenType::Else);
        assert_eq!(tokens[2].token_type, TokenType::QuestionMark);
        assert_eq!(tokens[3].token_type, TokenType::Colon);
    }

    #[test]
    fn scan_goto_and_label() {
        let lexer = Lexer::new();
        let code = "goto end; end: return 42;";

        let tokens = lexer.scan_tokens(code).unwrap();
        assert_eq!(tokens.len(), 8);
        assert_eq!(tokens[0].token_type, TokenType::Goto);
        assert_eq!(tokens[0].lexeme, "goto");
        assert_eq!(tokens[1].token_type, TokenType::Identifier);
        assert_eq!(tokens[1].lexeme, "end");
        assert_eq!(tokens[2].token_type, TokenType::Semicolon);
        assert_eq!(tokens[3].token_type, TokenType::Identifier);
        assert_eq!(tokens[3].lexeme, "end");
        assert_eq!(tokens[4].token_type, TokenType::Colon);
        assert_eq!(tokens[5].token_type, TokenType::Return);
        assert_eq!(tokens[5].lexeme, "return");
        assert_eq!(tokens[6].token_type, TokenType::IntegerConstant);
        assert_eq!(tokens[6].value, Some(TokenValue::Integer(42)));
        assert_eq!(tokens[6].lexeme, "42");
        assert_eq!(tokens[7].token_type, TokenType::Semicolon);
    }

    #[test]
    fn scan_switch_statement() {
        let lexer = Lexer::new();
        let code = "switch (x) { case 1: break; default: return 0; }";

        let tokens = lexer.scan_tokens(code).unwrap();

        assert_eq!(tokens.len(), 16);

        assert_eq!(tokens[0].token_type, TokenType::Switch);
        assert_eq!(tokens[0].lexeme, "switch");
        assert_eq!(tokens[1].token_type, TokenType::LeftParen);
        assert_eq!(tokens[2].token_type, TokenType::Identifier);
        assert_eq!(tokens[2].lexeme, "x");
        assert_eq!(tokens[3].token_type, TokenType::RightParen);
        assert_eq!(tokens[4].token_type, TokenType::LeftBrace);
        assert_eq!(tokens[5].token_type, TokenType::Case);
        assert_eq!(tokens[6].token_type, TokenType::IntegerConstant);
        assert_eq!(tokens[6].value, Some(TokenValue::Integer(1)));
        assert_eq!(tokens[7].token_type, TokenType::Colon);
        assert_eq!(tokens[8].token_type, TokenType::Break);
        assert_eq!(tokens[9].token_type, TokenType::Semicolon);
        assert_eq!(tokens[10].token_type, TokenType::Default);
        assert_eq!(tokens[11].token_type, TokenType::Colon);
        assert_eq!(tokens[12].token_type, TokenType::Return);
        assert_eq!(tokens[13].token_type, TokenType::IntegerConstant);
        assert_eq!(tokens[13].value, Some(TokenValue::Integer(0)));
        assert_eq!(tokens[14].token_type, TokenType::Semicolon);
        assert_eq!(tokens[15].token_type, TokenType::RightBrace);
    }

    #[test]
    fn scan_long() {
        let lexer = Lexer::new();
        let code = "long answer = 42L;";

        let tokens = lexer.scan_tokens(code).unwrap();
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].token_type, TokenType::Long);
        assert_eq!(tokens[1].token_type, TokenType::Identifier);
        assert_eq!(tokens[1].lexeme, "answer");
        assert_eq!(tokens[2].token_type, TokenType::Assign);
        assert_eq!(tokens[3].token_type, TokenType::LongConstant);
        assert_eq!(tokens[3].value, Some(TokenValue::Long(42)));
        assert_eq!(tokens[3].lexeme, "42L");
        assert_eq!(tokens[4].token_type, TokenType::Semicolon);
    }

    #[test]
    fn scan_promote_constants() {
        let lexer = Lexer::new();

        let code = r#"
        long negative_one = 1l; // can't use negative static initializers; negate this in main

        int main(void) {

            negative_one = -negative_one;
            if (68719476736u >= negative_one) {
                return 1;
            }

            return 0;
        }
        "#;

        let tokens = lexer.scan_tokens(code).unwrap();
        assert_eq!(tokens.len(), 31);
    }

    #[test]
    fn scan_double_to_unsigned_integer() {
        let lexer = Lexer::new();

        let code = r#"
        int main(void) {

            if (double_to_uint(2147483750.5) != 2147483750) {
                return 2;
            }

            return 0;
        }
        "#;

        let tokens = lexer.scan_tokens(code).unwrap();
        assert!(tokens.len() > 0);
    }
}
