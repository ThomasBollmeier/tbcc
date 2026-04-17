use crate::ast::Statement::{IfStatement, Return};
use crate::ast::{
    Associativity, BinaryOp, BlockItem, Declaration, Expression, FunctionDefinition, Program,
    Statement, UnaryOp,
};
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
            Some(token) => match token.token_type {
                TokenType::Return => self.return_statement(stream),
                TokenType::Semicolon => self.null_statement(stream),
                TokenType::If => self.if_statement(stream),
                _ => self.expression_statement(stream),
            },
            None => Err(anyhow!("Unexpected end of statement")),
        }
    }

    fn if_statement(&self, stream: &mut TokenStream) -> Result<Statement> {
        self.expect(stream, TokenType::If)?;
        self.expect(stream, TokenType::LeftParen)?;
        let condition = self.expression(stream, 0)?;
        self.expect(stream, TokenType::RightParen)?;
        let then_branch = self.statement(stream)?;

        let else_opt = stream.peek();
        let else_branch = if let Some(token) = else_opt {
            if token.token_type == TokenType::Else {
                stream.advance();
                let else_branch = self.statement(stream)?;
                Some(else_branch)
            } else {
                None
            }
        } else {
            None
        };

        Ok(IfStatement {
            condition,
            then_branch: Box::new(then_branch),
            else_branch: else_branch.map(Box::new),
        })
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
        use BinaryOp::*;
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

            let then_expr_opt = if op == Conditional {
                let then_expr = self.expression(stream, 0)?;
                self.expect(stream, TokenType::Colon)?;
                Some(then_expr)
            } else {
                None
            };

            let assoc = Associativity::from(&op);
            let next_min_prec = match assoc {
                Associativity::Left => prec + 1,
                Associativity::Right => prec,
            };

            let right = self.expression(stream, next_min_prec)?;
            left = match op {
                Assign => Expression::Assignment {
                    left: Box::new(left),
                    right: Box::new(right),
                    is_postfix: false,
                },
                Conditional => {
                    let then_expr = then_expr_opt.ok_or_else(|| {
                        anyhow!("Expected then-expression for conditional operator")
                    })?;
                    Expression::ConditionalExpr {
                        condition: Box::new(left),
                        then_expr: Box::new(then_expr),
                        else_expr: Box::new(right),
                    }
                }
                AssignAdd | AssignSubtract | AssignMultiply | AssignDivide | AssignRemainder
                | AssignBitAnd | AssignBitOr | AssignBitXor | AssignShiftLeft
                | AssignShiftRight => Expression::Assignment {
                    left: Box::new(left.clone()),
                    right: Box::new(Expression::BinaryExpr(
                        self.get_binary_op_from_compound(&op)?,
                        Box::new(left),
                        Box::new(right),
                    )),
                    is_postfix: false,
                },
                _ => Expression::BinaryExpr(op, Box::new(left), Box::new(right)),
            };
        }

        Ok(left)
    }

    fn get_binary_op_from_compound(&self, compound_op: &BinaryOp) -> Result<BinaryOp> {
        use BinaryOp::*;
        match compound_op {
            AssignAdd => Ok(Add),
            AssignSubtract => Ok(Subtract),
            AssignMultiply => Ok(Multiply),
            AssignDivide => Ok(Divide),
            AssignRemainder => Ok(Remainder),
            AssignBitAnd => Ok(BitAnd),
            AssignBitOr => Ok(BitOr),
            AssignBitXor => Ok(BitXor),
            AssignShiftLeft => Ok(ShiftLeft),
            AssignShiftRight => Ok(ShiftRight),
            _ => Err(anyhow!(
                "Unexpected operator for compound-operator argument"
            )),
        }
    }

    fn get_unary_op(&self, token_type: &TokenType) -> Option<UnaryOp> {
        match token_type {
            TokenType::Minus => Some(UnaryOp::Negate),
            TokenType::Tilde => Some(UnaryOp::Complement),
            TokenType::LogicalNot => Some(UnaryOp::Not),
            _ => None,
        }
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
            TokenType::AssignAdd => Some(BinaryOp::AssignAdd),
            TokenType::AssignSub => Some(BinaryOp::AssignSubtract),
            TokenType::AssignMul => Some(BinaryOp::AssignMultiply),
            TokenType::AssignDiv => Some(BinaryOp::AssignDivide),
            TokenType::AssignRemainder => Some(BinaryOp::AssignRemainder),
            TokenType::AssignBitAnd => Some(BinaryOp::AssignBitAnd),
            TokenType::AssignBitOr => Some(BinaryOp::AssignBitOr),
            TokenType::AssignBitXor => Some(BinaryOp::AssignBitXor),
            TokenType::AssignShiftLeft => Some(BinaryOp::AssignShiftLeft),
            TokenType::AssignShiftRight => Some(BinaryOp::AssignShiftRight),
            TokenType::QuestionMark => Some(BinaryOp::Conditional),
            _ => None,
        }
    }

    fn get_precedence(&self, binary_op: &BinaryOp) -> i32 {
        use BinaryOp::*;
        match binary_op {
            Assign | AssignAdd | AssignSubtract | AssignMultiply | AssignDivide
            | AssignRemainder | AssignBitAnd | AssignBitOr | AssignBitXor | AssignShiftLeft
            | AssignShiftRight => 1,
            Conditional => 5,
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
        let mut expr = match token.token_type {
            TokenType::IntegerConstant => {
                if let Some(TokenValue::Integer(value)) = token.value {
                    Expression::IntegerConstant(value)
                } else {
                    return Err(anyhow!("Expected integer constant value"));
                }
            }
            TokenType::Identifier => Expression::Var(token.lexeme),
            TokenType::Minus | TokenType::Tilde | TokenType::LogicalNot => {
                let op = self.get_unary_op(&token.token_type).unwrap();
                Expression::UnaryExpr(op, Box::new(self.factor(stream)?))
            }
            TokenType::IncrementPrefix | TokenType::DecrementPrefix => {
                let op = if token.token_type == TokenType::IncrementPrefix {
                    BinaryOp::Add
                } else {
                    BinaryOp::Subtract
                };
                let expr = self.factor(stream)?;
                Expression::Assignment {
                    left: Box::new(expr.clone()),
                    right: Box::new(Expression::BinaryExpr(
                        op,
                        Box::new(expr),
                        Box::new(Expression::IntegerConstant(1)),
                    )),
                    is_postfix: false,
                }
            }
            TokenType::LeftParen => {
                let expr = self.expression(stream, 0)?;
                self.expect(stream, TokenType::RightParen)?;
                expr
            }
            _ => return Err(anyhow!("Unexpected token {:?}", token)),
        };

        let next_token = stream.peek();
        if let Some(token) = next_token {
            match token.token_type {
                TokenType::IncrementPostfix => {
                    stream.advance(); // consume '++'
                    expr = self.create_postfix_assignment(BinaryOp::Add, &expr);
                }
                TokenType::DecrementPostfix => {
                    stream.advance(); // consume '--'
                    expr = self.create_postfix_assignment(BinaryOp::Subtract, &expr);
                }
                _ => {}
            }
        }

        Ok(expr)
    }

    fn create_postfix_assignment(&self, op: BinaryOp, expr: &Expression) -> Expression {
        Expression::Assignment {
            left: Box::new(expr.clone()),
            right: Box::new(Expression::BinaryExpr(
                op,
                Box::new(expr.clone()),
                Box::new(Expression::IntegerConstant(1)),
            )),
            is_postfix: true,
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
    fn parse_bitwise() {
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

    #[test]
    fn parse_inc_dec() {
        let code = r#"
        int main(void) {
            int a = 1;
            int b = 2;
            int c = -++(a);
            int d = !(b)--;
            return (a == 2 && b == 1 && c == -2 && d == 0);
        }
        "#;
        parse_code(code, true);
    }

    #[test]
    fn parse_compound_assignments() {
        let code = r#"
        int main(void) {
            int a = 8;
            int b = 3;
            a += b;
            a -= b;
            a *= b;
            a /= b;
            a &= b;
            a |= b;
            a ^= b;
            a <<= 1;
            a >>= 1;
            return a;
        }
        "#;
        parse_code(code, true);
    }

    #[test]
    fn parse_compound_chained() {
        let code = r#"
        int main(void) {
            int a = 250;
            int b = 200;
            int c = 100;
            int d = 75;
            int e = -25;
            int f = 0;
            int x = 0;
            x = a += b -= c *= d /= e %= f = -7;
            return a == 2250 && b == 2000 && c == -1800 && d == -18 && e == -4 &&
                   f == -7 && x == 2250;
        }
        "#;
        parse_code(code, true);
    }

    #[test]
    fn parse_if() {
        let code = r#"
        int main(void) {
            int everything = 1;
            if (everything)
                return 42;
        }
        "#;
        parse_code(code, true);
    }

    #[test]
    fn parse_if_else() {
        let code = r#"
        int main(void) {
            int everything = 0;
            if (everything)
                return 42;
            else
                return 23;
        }
        "#;
        parse_code(code, true);
    }

    #[test]
    fn parse_conditional() {
        let code = r#"
        int main(void) {
            int everything = 1;
            int beast = 0;
            int answer = !everything ? beast ? 666 : 23 : 42;
            return answer;
        }
        "#;
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
