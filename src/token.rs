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
    Minus,
    MinusMinus,
    Tilde,
    Plus,
    Asterisk,
    Slash,
    Percent,
    BitAnd,
    BitOr,
    BitXor,
    ShiftLeft,
    ShiftRight,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenValue {
    Integer(i32),
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

pub struct TokenStream {
    tokens: Vec<Token>,
    position: usize,
}

impl TokenStream {
    pub fn new(tokens: Vec<Token>) -> TokenStream {
        TokenStream {
            tokens,
            position: 0,
        }
    }

    pub fn has_next(&self) -> bool {
        self.position < self.tokens.len()
    }

    pub fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    pub fn peek_with_offset(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.position + offset)
    }

    pub fn advance(&mut self) -> Option<&Token> {
        let result = self.tokens.get(self.position);
        self.position += 1;
        result
    }
}


