use crate::ast::Statement::Return;
use crate::ast::{Associativity, BinaryOp, BlockItem, Declaration, Expression, FunctionDefinition, Program, Statement, UnaryOp};
use crate::token::{Token, TokenStream, TokenType, TokenValue};
use anyhow::{Result, anyhow};

pub struct Parser {}

impl Parser {
    pub fn new() -> Self {
        Parser {}
    }

    pub fn parse(&self, tokens: Vec<Token>) -> Result<Program> {
        let mut stream = TokenStream::new(tokens);
        let program = self.program(&mut stream)?;

        if !stream.has_next() {
            Ok(program)
        } else {
            Err(anyhow!("Unexpected tokens after end of program"))
        }
    }

    fn program(&self, stream: &mut TokenStream) -> Result<Program> {
        let func_def = self.function_definition(stream)?;
        Ok(Program::new(func_def))
    }

    fn function_definition(&self, stream: &mut TokenStream) -> Result<FunctionDefinition> {
        self.expect(stream, TokenType::Int)?;

        let name_token = self.expect(stream, TokenType::Identifier)?;
        let name = name_token.lexeme;

        self.expect(stream, TokenType::LeftParen)?;
        self.expect(stream, TokenType::Void)?;
        self.expect(stream, TokenType::RightParen)?;
        self.expect(stream, TokenType::LeftBrace)?;

        let body = self.body(stream)?;

        self.expect(stream, TokenType::RightBrace)?;

        Ok(FunctionDefinition::new(name, body))
    }

    fn body(&self, stream: &mut TokenStream) -> Result<Vec<BlockItem>> {
        let mut items = Vec::new();

        while let Some(token) = stream.peek() {
            match token.token_type {
                TokenType::RightBrace => break,
                TokenType::Int => {
                    items.push(BlockItem::Declaration(self.declaration(stream)?));
                }
                _ => {
                    items.push(BlockItem::Statement(self.statement(stream)?));
                }
            }
        }

        Ok(items)
    }

    fn declaration(&self, stream: &mut TokenStream) -> Result<Declaration> {
        self.expect(stream, TokenType::Int)?;
        let name_token = self.expect(stream, TokenType::Identifier)?;
        let name = name_token.lexeme;

        let init_expr = if let Some(token) = stream.peek() {
            if token.token_type == TokenType::Assign {
                stream.advance(); // consume '='
                Some(self.expression(stream, 0)?)
            } else {
                None
            }
        } else {
            None
        };

        self.expect(stream, TokenType::Semicolon)?;

        Ok(Declaration::new(name, init_expr))
    }

    fn statement(&self, stream: &mut TokenStream) -> Result<Statement> {
        let next_token = stream.peek();
        match next_token {
            Some(token) => {
                match token.token_type {
                    TokenType::Return => self.return_statement(stream),
                    TokenType::Semicolon => self.null_statement(stream),
                    _ => self.expression_statement(stream),
                }
            }
            None => Err(anyhow!("Unexpected end of statement")),
        }
    }

    fn expression_statement(&self, stream: &mut TokenStream) -> Result<Statement> {
        let expr = self.expression(stream, 0)?;
        self.expect(stream, TokenType::Semicolon)?;
        Ok(Statement::Expression(expr))
    }

    fn null_statement(&self, stream: &mut TokenStream) -> Result<Statement> {
        self.consume(stream)?; // consume the semicolon
        Ok(Statement::Null)
    }

    fn return_statement(&self, stream: &mut TokenStream) -> Result<Statement> {
        self.expect(stream, TokenType::Return)?;
        let expr = self.expression(stream, 0)?;
        self.expect(stream, TokenType::Semicolon)?;
        Ok(Return(expr))
    }

    fn expression(&self, stream: &mut TokenStream, min_precedence: i32) -> Result<Expression> {
        let mut left = self.factor(stream)?;

        while let Some(token) = stream.peek() {
            let op = match self.get_binary_op(&token.token_type) {
                Some(op) => op,
                None => break,
            };
            let prec = self.get_precedence(&op);
            if prec < min_precedence {
                break;
            }
            stream.advance(); // consume the operator

            let assoc = Associativity::from(&op);
            let next_min_prec = match assoc {
                Associativity::Left => prec + 1,
                Associativity::Right => prec
            };

            let right = self.expression(stream, next_min_prec)?;
            left = match op {
                BinaryOp::Assign => Expression::Assignment(Box::new(left), Box::new(right)),
                _ => Expression::BinaryExpr(op, Box::new(left), Box::new(right)),
            };

        }

        Ok(left)
    }

    fn get_binary_op(&self, token_type: &TokenType) -> Option<BinaryOp> {
        match token_type {
            TokenType::Plus => Some(BinaryOp::Add),
            TokenType::Minus => Some(BinaryOp::Subtract),
            TokenType::Asterisk => Some(BinaryOp::Multiply),
            TokenType::Slash => Some(BinaryOp::Divide),
            TokenType::Percent => Some(BinaryOp::Remainder),
            TokenType::BitAnd => Some(BinaryOp::BitAnd),
            TokenType::BitOr => Some(BinaryOp::BitOr),
            TokenType::BitXor => Some(BinaryOp::BitXor),
            TokenType::ShiftLeft => Some(BinaryOp::ShiftLeft),
            TokenType::ShiftRight => Some(BinaryOp::ShiftRight),
            TokenType::LogicalAnd => Some(BinaryOp::LogicalAnd),
            TokenType::LogicalOr => Some(BinaryOp::LogicalOr),
            TokenType::Equal => Some(BinaryOp::Equal),
            TokenType::NotEqual => Some(BinaryOp::NotEqual),
            TokenType::Greater => Some(BinaryOp::Greater),
            TokenType::Less => Some(BinaryOp::Less),
            TokenType::GreaterEqual => Some(BinaryOp::GreaterEqual),
            TokenType::LessEqual => Some(BinaryOp::LessEqual),
            TokenType::Assign => Some(BinaryOp::Assign),
            _ => None,
        }
    }

    fn get_precedence(&self, binary_op: &BinaryOp) -> i32 {
        use BinaryOp::*;
        match binary_op {
            Assign => 1,
            LogicalOr => 15,
            LogicalAnd => 20,
            BitOr => 25,
            BitXor => 30,
            BitAnd => 35,
            Equal | NotEqual => 36,
            Greater | Less | GreaterEqual | LessEqual => 37,
            ShiftLeft | ShiftRight => 40,
            Add | Subtract => 45,
            Multiply | Divide | Remainder => 50,
        }
    }

    fn factor(&self, stream: &mut TokenStream) -> Result<Expression> {
        let token = self.consume(stream)?;
        match token.token_type {
            TokenType::IntegerConstant => {
                if let Some(TokenValue::Integer(value)) = token.value {
                    Ok(Expression::IntegerConstant(value))
                } else {
                    Err(anyhow!("Expected integer constant value"))
                }
            }
            TokenType::Identifier => {
                Ok(Expression::Var(token.lexeme))
            }
            TokenType::Minus | TokenType::Tilde | TokenType::LogicalNot => {
                let op = match token.token_type {
                    TokenType::Minus => UnaryOp::Negate,
                    TokenType::Tilde => UnaryOp::Complement,
                    TokenType::LogicalNot => UnaryOp::Not,
                    _ => unreachable!(),
                };
                Ok(Expression::UnaryExpr(op, Box::new(self.factor(stream)?)))
            }
            TokenType::LeftParen => {
                let expr = self.expression(stream, 0)?;
                self.expect(stream, TokenType::RightParen)?;
                Ok(expr)
            }
            _ => Err(anyhow!("Unexpected token {:?}", token)),
        }
    }

    fn expect(&self, stream: &mut TokenStream, expected_type: TokenType) -> Result<Token> {
        match stream.advance() {
            Some(token) => {
                if token.token_type == expected_type {
                    Ok(token.clone())
                } else {
                    Err(anyhow!(
                        "Expected token {:?}, got {:?}",
                        expected_type,
                        token.token_type
                    ))
                }
            }
            None => Err(anyhow!(
                "Expected token of type {:?} but found end of input",
                expected_type
            )),
        }
    }

    fn consume(&self, stream: &mut TokenStream) -> Result<Token> {
        Ok(stream
            .advance()
            .ok_or_else(|| anyhow!("Unexpected end of input"))?
            .clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    #[test]
    fn parses_with_success() {
        let code = "int main(void) { return 42; }";
        parse_code(code, true);
    }

    #[test]
    fn parse_with_negation_and_complement() {
        let code = "int main(void) { return -(-(~42)); }";
        parse_code(code, true);
    }

    #[test]
    fn parse_complement() {
        let code = r#"
        int main(void) {
            return ~12;
        }
        "#;
        parse_code(code, true);
    }

    #[test]
    fn parse_binary() {
        let code = r#"
        int main(void) {
            return 1 + 2 * 3;
        }"#;
        parse_code(code, true);
    }

    #[test]
    fn parse_bitwise()  {
        let code = r#"
        int main(void) {
            return 1 | 2 & 3 ^ 4 << 1 >> 2;
        }"#;
        parse_code(code, true);
    }

    #[test]
    fn parse_variable() {
        let code = r#"
        int main(void) {
            int x = 1;
            int y = 2;
            int z;
            z = 3;
            42;;
            return x + y + z;
        }"#;
        parse_code(code, true);
    }

    fn parse_code(code: &str, expect_success: bool) {
        let parser = Parser::new();
        let lexer = Lexer::new();

        let tokens = lexer.scan_tokens(code).expect("Failed to scan tokens");
        let result = parser.parse(tokens);

        match result {
            Ok(program) => {
                if expect_success {
                    dbg!(&program);
                } else {
                    panic!("Expected parsing to fail but it succeeded");
                }
            }
            Err(err) => {
                if expect_success {
                    panic!("{}", err);
                }
            }
        }
    }
}
