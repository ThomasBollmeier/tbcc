#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    Whitespace,
    Identifier,
    IntegerConstant,
    Int,
    Void,
    Return,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Semicolon,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenValue {
    Integer(i64),
}

#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub value: Option<TokenValue>,
    pub lexeme: String,
    pub line: usize,
    pub column: usize,
}

impl Token {
    pub fn new(
        token_type: TokenType,
        value: Option<TokenValue>,
        lexeme: String,
        line: usize,
        column: usize,
    ) -> Token {
        Token {
            token_type,
            value,
            lexeme,
            line,
            column,
        }
    }
}
