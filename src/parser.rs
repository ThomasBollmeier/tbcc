use crate::ast::{Expression, FunctionDefinition, Program, Statement};
use anyhow::{anyhow, Result};
use crate::ast::Statement::Return;
use crate::token::{Token, TokenStream, TokenType, TokenValue};

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

        let body = self.statement(stream)?;

        self.expect(stream, TokenType::RightBrace)?;

        Ok(FunctionDefinition::new(name, body))
    }

    fn statement(&self, stream: &mut TokenStream) -> Result<Statement> {
        self.expect(stream, TokenType::Return)?;
        let expr = self.expression(stream)?;
        self.expect(stream, TokenType::Semicolon)?;
        Ok(Return(expr))
    }

    fn expression(&self, stream: &mut TokenStream) -> Result<Expression> {
        let token = self.expect(stream, TokenType::IntegerConstant)?;
        if let Some(TokenValue::Integer(value)) = token.value {
            Ok(Expression::IntegerConstant(value))
        } else {
            Err(anyhow!("Expected integer constant value"))
        }
    }

    fn expect(&self, stream: &mut TokenStream, expected_type: TokenType) -> Result<Token> {
        match stream.advance() {
            Some(token) => {
                if token.token_type == expected_type {
                    Ok(token.clone())
                } else {
                    Err(anyhow!("Expected token {:?}, got {:?}", expected_type, token.token_type))
                }
            },
            None => Err(anyhow!("Expected token of type {:?} but found end of input", expected_type)),
        }
    }

}

#[cfg(test)]
mod tests {
    use crate::lexer::Lexer;
    use super::*;

    #[test]
    fn parses_with_success() {
        let parser = Parser::new();
        let lexer = Lexer::new();

        let code = "int main(void) { return 42; }";

        let tokens = lexer.scan_tokens(code).expect("Failed to scan tokens");
        let program = parser.parse(tokens).expect("Failed to parse program");

        dbg!(&program);
    }
}