use crate::ast::Statement::{For, IfStatement, Return, SwitchStatement};
use crate::ast::{
    Associativity, BinaryOp, Block, BlockItem, Declaration, Expression, ForInit,
    FunctionDeclaration, Label, Program, Statement, StorageClass, UnaryOp, VarDeclaration,
};
use crate::token::{Token, TokenStream, TokenType, TokenValue};
use anyhow::{Result, anyhow};
use std::collections::HashSet;

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
        let mut program = Program::new();
        while let Some(_) = stream.peek() {
            program.decls.push(self.declaration(stream)?);
        }
        Ok(program)
    }

    fn declaration(&self, stream: &mut TokenStream) -> Result<Declaration> {
        let specifiers = self.specifiers(stream)?;
        let storage_class = if specifiers.contains(&TokenType::Static) {
            Some(StorageClass::Static)
        } else if specifiers.contains(&TokenType::Extern) {
            Some(StorageClass::Extern)
        } else {
            None
        };

        let name = self.expect(stream, TokenType::Identifier)?.lexeme;

        let next_token = stream
            .peek()
            .ok_or_else(|| anyhow!("Unexpected end of input after identifier"))?;

        match next_token.token_type {
            TokenType::LeftParen => {
                let func_decl = self.function_declaration(stream, name, storage_class)?;
                Ok(Declaration::FunctionDecl(func_decl))
            }
            _ => {
                let var_decl = self.variable_declaration(stream, name, storage_class)?;
                Ok(Declaration::VarDecl(var_decl))
            }
        }
    }

    fn specifiers(&self, stream: &mut TokenStream) -> Result<HashSet<TokenType>> {
        let allowed_token_types: HashSet<TokenType> =
            HashSet::from_iter([TokenType::Int, TokenType::Extern, TokenType::Static]);
        let mut specifiers = HashSet::new();

        while let Some(token) = stream.peek() {
            if !allowed_token_types.contains(&token.token_type) {
                break;
            }
            if specifiers.contains(&token.token_type) {
                return Err(anyhow!("duplicate specifier {}", token.lexeme));
            }
            specifiers.insert(token.token_type.clone());
            stream.advance();
        }

        let num_specifiers = specifiers.len();

        match num_specifiers {
            1 => {
                if specifiers.contains(&TokenType::Static)
                    || specifiers.contains(&TokenType::Extern)
                {
                    return Err(anyhow!(
                        "static or extern specifier cannot be specified without type"
                    ));
                }
            }
            2 => {
                if specifiers.contains(&TokenType::Static)
                    && specifiers.contains(&TokenType::Extern)
                {
                    return Err(anyhow!(
                        "static specifier cannot be specified together with extern"
                    ));
                }
            }
            _ => return Err(anyhow!("expected 1 or 2 specifiers")),
        }

        Ok(specifiers)
    }

    fn function_declaration(
        &self,
        stream: &mut TokenStream,
        name: String,
        storage_class: Option<StorageClass>,
    ) -> Result<FunctionDeclaration> {
        self.expect(stream, TokenType::LeftParen)?;
        let mut parameters = vec![];
        loop {
            let token = stream
                .peek()
                .ok_or_else(|| anyhow!("Unexpected end of input in parameter list"))?;

            match token.token_type {
                TokenType::Void => {
                    if parameters.is_empty() {
                        stream.advance();
                        break;
                    } else {
                        return Err(anyhow!("Unexpected 'void' in parameter list"));
                    }
                }
                TokenType::Int => {
                    if !parameters.is_empty() {
                        return Err(anyhow!("Unexpected 'int' in parameter list"));
                    }
                    stream.advance();
                    let param_name_token = self.expect(stream, TokenType::Identifier)?;
                    parameters.push(param_name_token.lexeme);
                }
                TokenType::Comma => {
                    if parameters.is_empty() {
                        return Err(anyhow!("Unexpected ',' in parameter list"));
                    }
                    stream.advance();
                    self.expect(stream, TokenType::Int)?;
                    let param_name_token = self.expect(stream, TokenType::Identifier)?;
                    parameters.push(param_name_token.lexeme);
                }
                TokenType::RightParen => {
                    break;
                }
                _ => return Err(anyhow!("Unexpected token {:?} in parameter list", token)),
            }
        }
        self.expect(stream, TokenType::RightParen)?;

        let next_token = stream
            .peek()
            .ok_or_else(|| anyhow!("Unexpected end of input after function declaration"))?;

        let body = match next_token.token_type {
            TokenType::Semicolon => {
                stream.advance();
                None
            }
            _ => Some(self.block(stream)?),
        };

        Ok(FunctionDeclaration::new(
            name,
            parameters,
            body,
            storage_class,
        ))
    }

    fn block(&self, stream: &mut TokenStream) -> Result<Block> {
        let mut items = Vec::new();

        self.expect(stream, TokenType::LeftBrace)?;

        while let Some(token) = stream.peek() {
            match token.token_type {
                TokenType::RightBrace => break,
                TokenType::Int | TokenType::Static | TokenType::Extern => {
                    let declaration = self.declaration(stream)?;
                    match declaration {
                        Declaration::FunctionDecl(func_decl) => {
                            items.push(BlockItem::FunctionDeclaration(func_decl));
                        }
                        Declaration::VarDecl(var_decl) => {
                            items.push(BlockItem::VarDeclaration(var_decl));
                        }
                    }
                }
                _ => {
                    items.push(BlockItem::Statement(self.statement(stream)?));
                }
            }
        }

        self.expect(stream, TokenType::RightBrace)?;

        Ok(Block::new(items))
    }

    fn variable_declaration(
        &self,
        stream: &mut TokenStream,
        name: String,
        storage_class: Option<StorageClass>,
    ) -> Result<VarDeclaration> {
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

        Ok(VarDeclaration::new(name, init_expr, storage_class))
    }

    fn statement(&self, stream: &mut TokenStream) -> Result<Statement> {
        if let Some(label) = self.get_label(stream)? {
            let statement = self.statement(stream)?;
            return Ok(Statement::LabeledStatement {
                label,
                statement: Box::new(statement),
            });
        }

        let next_token = stream.peek();
        match next_token {
            Some(token) => match token.token_type {
                TokenType::Return => self.return_statement(stream),
                TokenType::Break => self.break_statement(stream),
                TokenType::Continue => self.continue_statement(stream),
                TokenType::Semicolon => self.null_statement(stream),
                TokenType::LeftBrace => self.compound_statement(stream),
                TokenType::If => self.if_statement(stream),
                TokenType::Switch => self.switch_statement(stream),
                TokenType::While => self.while_statement(stream),
                TokenType::Do => self.do_while_statement(stream),
                TokenType::For => self.for_statement(stream),
                TokenType::Goto => self.goto_statement(stream),
                _ => self.expression_statement(stream),
            },
            None => Err(anyhow!("Unexpected end of statement")),
        }
    }

    fn get_label(&self, stream: &mut TokenStream) -> Result<Option<Label>> {
        let next_token = match stream.peek() {
            Some(token) => token.clone(),
            None => return Ok(None),
        };

        match next_token.token_type {
            TokenType::Identifier => {
                let next_next_token = match stream.peek_with_offset(1) {
                    Some(token) => token.clone(),
                    None => return Ok(None),
                };
                if next_next_token.token_type == TokenType::Colon {
                    stream.advance(); // consume identifier
                    stream.advance(); // consume colon
                    Ok(Some(Label::Label(next_token.lexeme)))
                } else {
                    Ok(None)
                }
            }
            TokenType::Case => {
                stream.advance();
                let expr = self.expression(stream, 0)?;
                self.expect(stream, TokenType::Colon)?;
                Ok(Some(Label::Case {
                    case_id: "".to_string(),
                    value: expr,
                }))
            }
            TokenType::Default => {
                stream.advance();
                self.expect(stream, TokenType::Colon)?;
                Ok(Some(Label::Default {
                    default_id: "".to_string(),
                }))
            }
            _ => Ok(None),
        }
    }

    fn goto_statement(&self, stream: &mut TokenStream) -> Result<Statement> {
        self.expect(stream, TokenType::Goto)?;
        let target_token = self.expect(stream, TokenType::Identifier)?;
        let target = target_token.lexeme;
        self.expect(stream, TokenType::Semicolon)?;
        Ok(Statement::GotoStatement(target))
    }

    fn compound_statement(&self, stream: &mut TokenStream) -> Result<Statement> {
        self.block(stream).map(Statement::CompoundStatement)
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

    fn switch_statement(&self, stream: &mut TokenStream) -> Result<Statement> {
        self.expect(stream, TokenType::Switch)?;
        self.expect(stream, TokenType::LeftParen)?;
        let condition = self.expression(stream, 0)?;
        self.expect(stream, TokenType::RightParen)?;
        let body = Box::new(self.statement(stream)?);

        Ok(SwitchStatement {
            switch_id: "".to_string(),
            condition,
            body,
            arms: Vec::new(),
        })
    }

    fn while_statement(&self, stream: &mut TokenStream) -> Result<Statement> {
        self.expect(stream, TokenType::While)?;
        self.expect(stream, TokenType::LeftParen)?;
        let condition = self.expression(stream, 0)?;
        self.expect(stream, TokenType::RightParen)?;
        let body = self.statement(stream)?;

        Ok(Statement::While {
            loop_id: String::new(),
            condition,
            body: Box::new(body),
        })
    }

    fn do_while_statement(&self, stream: &mut TokenStream) -> Result<Statement> {
        self.expect(stream, TokenType::Do)?;
        let body = self.statement(stream)?;
        self.expect(stream, TokenType::While)?;
        self.expect(stream, TokenType::LeftParen)?;
        let condition = self.expression(stream, 0)?;
        self.expect(stream, TokenType::RightParen)?;
        self.expect(stream, TokenType::Semicolon)?;

        Ok(Statement::DoWhile {
            loop_id: String::new(),
            condition,
            body: Box::new(body),
        })
    }

    fn for_statement(&self, stream: &mut TokenStream) -> Result<Statement> {
        self.expect(stream, TokenType::For)?;
        self.expect(stream, TokenType::LeftParen)?;

        let init = self.for_init(stream)?;
        let condition = self.for_condition(stream)?;
        let post = self.for_post(stream)?;
        let body = Box::new(self.statement(stream)?);

        Ok(For {
            loop_id: String::new(),
            init,
            condition,
            post,
            body,
        })
    }

    fn for_post(&self, stream: &mut TokenStream) -> Result<Option<Expression>> {
        self.get_optional_expression(stream, TokenType::RightParen)
    }

    fn for_condition(&self, stream: &mut TokenStream) -> Result<Option<Expression>> {
        self.get_optional_expression(stream, TokenType::Semicolon)
    }

    fn get_optional_expression(
        &self,
        stream: &mut TokenStream,
        terminator_type: TokenType,
    ) -> Result<Option<Expression>> {
        let token = stream
            .peek()
            .ok_or_else(|| anyhow!("Unexpected end of input in for-loop initializer"))?;

        Ok(if token.token_type == terminator_type {
            stream.advance();
            None
        } else {
            let expr = self.expression(stream, 0)?;
            self.expect(stream, terminator_type)?;
            Some(expr)
        })
    }

    fn for_init(&self, stream: &mut TokenStream) -> Result<ForInit> {
        let token = stream
            .peek()
            .ok_or_else(|| anyhow!("Unexpected end of input in for-loop initializer"))?;

        let for_init = match token.token_type {
            TokenType::Int => self.for_init_declaration(stream)?,
            TokenType::Semicolon => {
                self.expect(stream, TokenType::Semicolon)?;
                ForInit::InitExpression(None)
            }
            _ => {
                let expr = self.expression(stream, 0)?;
                self.expect(stream, TokenType::Semicolon)?;
                ForInit::InitExpression(Some(expr))
            }
        };

        Ok(for_init)
    }

    fn for_init_declaration(&self, stream: &mut TokenStream) -> Result<ForInit> {
        self.expect(stream, TokenType::Int)?;
        let name = self.expect(stream, TokenType::Identifier)?.lexeme;
        let var_decl = self.variable_declaration(stream, name, None)?;

        Ok(ForInit::InitDeclaration(var_decl))
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

    fn break_statement(&self, stream: &mut TokenStream) -> Result<Statement> {
        self.expect(stream, TokenType::Break)?;
        self.expect(stream, TokenType::Semicolon)?;
        Ok(Statement::Break {
            loop_id: String::new(),
        })
    }

    fn continue_statement(&self, stream: &mut TokenStream) -> Result<Statement> {
        self.expect(stream, TokenType::Continue)?;
        self.expect(stream, TokenType::Semicolon)?;
        Ok(Statement::Continue {
            loop_id: String::new(),
        })
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
            TokenType::Identifier => {
                let name = token.lexeme.clone();
                let is_func_call = if let Some(next_token) = stream.peek() {
                    next_token.token_type == TokenType::LeftParen
                } else {
                    false
                };
                if is_func_call {
                    Expression::FuncCall {
                        name,
                        args: self.argument_list(stream)?,
                    }
                } else {
                    Expression::Var(name)
                }
            }
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

    fn argument_list(&self, stream: &mut TokenStream) -> Result<Vec<Expression>> {
        let mut args = Vec::new();
        self.expect(stream, TokenType::LeftParen)?;
        loop {
            let token = stream
                .peek()
                .ok_or_else(|| anyhow!("Unexpected end of input in argument list"))?;
            match token.token_type {
                TokenType::RightParen => break,
                TokenType::Comma => {
                    if args.is_empty() {
                        return Err(anyhow!("Unexpected comma in argument list"));
                    }
                    self.consume(stream)?; // consume comma
                    args.push(self.expression(stream, 0)?);
                }
                _ => args.push(self.expression(stream, 0)?),
            }
        }
        self.expect(stream, TokenType::RightParen)?;

        Ok(args)
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

    #[test]
    fn parse_goto() {
        let code = r#"
        int main(void) {
            goto everything;
            return 666;
        everything:
            return 42;
        }
        "#;
        parse_code(code, true);
    }

    #[test]
    fn parse_for_loop() {
        let code = r#"
        int main(void)
        {
            for (i = 0; i < 1; i = i + 1)
            {
                return 0;
            }
        }
        "#;
        parse_code(code, true);
    }

    #[test]
    fn parse_switch() {
        let code = r#"
        int main(void)
        {
            int answer = 42;

            switch (answer) {
            case 42:
                break;
            default:
                return 0;
            }

            return 1;
        }
        "#;
        parse_code(code, true);
    }

    #[test]
    fn parse_duffs_device() {
        let code = r#"
        // A fun use of fallthrough - see https://en.wikipedia.org/wiki/Duff%27s_device
        int main(void) {
            int count = 37;
            int iterations = (count + 4) / 5;
            switch (count % 5) {
                case 0:
                    do {
                        count = count - 1;
                        case 4:
                            count = count - 1;
                        case 3:
                            count = count - 1;
                        case 2:
                            count = count - 1;
                        case 1:
                            count = count - 1;
                    } while ((iterations = iterations - 1) > 0);
            }
            return (count == 0 && iterations == 0);
        }
        "#;
        parse_code(code, true);
    }

    #[test]
    fn parse_functions() {
        let code = r#"
        int do_something(int a);
        int do_two_things(int a, int b) {
            return
                do_something(a) +
                do_something(b);
        }

        int main(void)
        {
            for (i = 0; i < 1; i = i + 1)
            {
                return 0;
            }
        }
        "#;
        parse_code(code, true);
    }

    #[test]
    fn parse_functions_and_variables() {
        let code = r#"
        extern int answer = 42;
        int do_something(int a);
        int do_two_things(int a, int b) {
            static int counter = 0;
            counter++;
            return
                do_something(a) +
                do_something(b);
        }

        int main(void)
        {
            for (i = 0; i < 1; i = i + 1)
            {
                return 0;
            }
        }
        "#;
        parse_code(code, true);
    }

    #[test]
    fn parse_missing_type_specifier() {
        let code = r#"
        static answer = 42;

        int main(void)
        {
            return answer;
        }
        "#;
        parse_code(code, false);
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
