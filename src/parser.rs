use crate::ast::{
    BinaryOp, Block, Expr, ExprKind, Function, Parameter, Program, Type,
};
use crate::lexer::ParseError;
use crate::token::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    next_expr_id: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            next_expr_id: 0,
        }
    }

    pub fn parse_program(mut self) -> Result<Program, ParseError> {
        let mut functions = Vec::new();
        while !self.check(&TokenKind::Eof) {
            functions.push(self.parse_function()?);
        }
        Ok(Program { functions })
    }

    fn parse_function(&mut self) -> Result<Function, ParseError> {
        self.expect(TokenKind::Fn)?;
        let name = self.consume_identifier("nom de fonction")?;
        self.expect(TokenKind::LParen)?;

        let mut params = Vec::new();
        if !self.check(TokenKind::RParen) {
            loop {
                let param_name = self.consume_identifier("nom de paramètre")?;
                self.expect(TokenKind::Colon)?;
                let ty = self.parse_type()?;
                params.push(Parameter {
                    name: param_name,
                    ty,
                });
                if self.check(TokenKind::Comma) {
                    self.bump();
                } else {
                    break;
                }
            }
        }
        self.expect(TokenKind::RParen)?;
        self.expect(TokenKind::Arrow)?;
        let return_type = self.parse_type()?;
        let body = self.parse_block()?;
        Ok(Function {
            name,
            params,
            return_type,
            body,
        })
    }

    fn parse_type(&mut self) -> Result<Type, ParseError> {
        let mut ptr_depth = 0;
        while self.check(TokenKind::Star) {
            self.bump();
            ptr_depth += 1;
        }

        let mut base = match self.bump() {
            Some(Token {
                kind: TokenKind::I8, ..
            }) => Type::I8,
            Some(Token {
                kind: TokenKind::I16, ..
            }) => Type::I16,
            Some(Token {
                kind: TokenKind::I32, ..
            }) => Type::I32,
            Some(Token {
                kind: TokenKind::I64, ..
            }) => Type::I64,
            Some(Token {
                kind: TokenKind::U8, ..
            }) => Type::U8,
            Some(Token {
                kind: TokenKind::U16, ..
            }) => Type::U16,
            Some(Token {
                kind: TokenKind::U32, ..
            }) => Type::U32,
            Some(Token {
                kind: TokenKind::U64, ..
            }) => Type::U64,
            Some(Token {
                kind: TokenKind::F32, ..
            }) => Type::F32,
            Some(Token {
                kind: TokenKind::F64, ..
            }) => Type::F64,
            Some(Token {
                kind: TokenKind::Bool, ..
            }) => Type::Bool,
            Some(Token {
                kind: TokenKind::Void, ..
            }) => Type::Void,
            Some(tok) => {
                return Err(ParseError {
                    message: format!("type inattendu {:?}", tok.kind),
                    line: tok.line,
                    column: tok.column,
                });
            }
            None => {
                return Err(ParseError {
                    message: "fin inattendue lors de la lecture d'un type".to_string(),
                    line: 0,
                    column: 0,
                });
            }
        };

        for _ in 0..ptr_depth {
            base = Type::Pointer(Box::new(base));
        }
        Ok(base)
    }

    fn parse_block(&mut self) -> Result<Block, ParseError> {
        self.expect(TokenKind::LBrace)?;
        let mut expressions = Vec::new();
        while !self.check(TokenKind::RBrace) {
            expressions.push(self.parse_statement()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Block { expressions })
    }

    fn parse_statement(&mut self) -> Result<Expr, ParseError> {
        let expr = match self.current_kind() {
            Some(TokenKind::Let) => {
                let stmt = self.parse_let()?;
                self.expect(TokenKind::Semi)?;
                stmt
            }
            Some(TokenKind::Store) => {
                let stmt = self.parse_store()?;
                self.expect(TokenKind::Semi)?;
                stmt
            }
            _ => {
                let expr = self.parse_expression(0)?;
                if self.check(TokenKind::Semi) {
                    self.bump();
                }
                expr
            }
        };
        Ok(expr)
    }

    fn parse_let(&mut self) -> Result<Expr, ParseError> {
        self.expect(TokenKind::Let)?;
        let name = self.consume_identifier("nom de variable")?;
        let ty = if self.check(TokenKind::Colon) {
            self.bump();
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(TokenKind::Eq)?;
        let value = self.parse_expression(0)?;
        Ok(self.expr(ExprKind::Let {
            name,
            ty,
            value: Box::new(value),
        }))
    }

    fn parse_store(&mut self) -> Result<Expr, ParseError> {
        self.expect(TokenKind::Store)?;
        self.expect(TokenKind::LParen)?;
        let value = self.parse_expression(0)?;
        self.expect(TokenKind::Comma)?;
        let ptr = self.parse_expression(0)?;
        self.expect(TokenKind::RParen)?;
        Ok(self.expr(ExprKind::Store(Box::new(value), Box::new(ptr))))
    }

    fn parse_expression(&mut self, min_prec: u8) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_unary_expression()?;
        while let Some((op, prec, left_assoc)) = Self::binary_meta(self.current_kind()) {
            if prec < min_prec {
                break;
            }
            self.bump();
            let next_min = if left_assoc { prec + 1 } else { prec };
            let rhs = self.parse_expression(next_min)?;
            lhs = self.expr(ExprKind::Binary(op, Box::new(lhs), Box::new(rhs)));
        }
        Ok(lhs)
    }

    fn parse_unary_expression(&mut self) -> Result<Expr, ParseError> {
        match self.current_kind() {
            Some(TokenKind::If) => self.parse_if_else(),
            Some(TokenKind::LBrace) => {
                let block = self.parse_block()?;
                Ok(self.expr(ExprKind::Block(block)))
            }
            Some(TokenKind::IntLiteral(_))
            | Some(TokenKind::FloatLiteral(_))
            | Some(TokenKind::True)
            | Some(TokenKind::False)
            | Some(TokenKind::Identifier(_))
            | Some(TokenKind::Load)
            | Some(TokenKind::Alloc)
            | Some(TokenKind::Free)
            | Some(TokenKind::SizeOf)
            | Some(TokenKind::LParen) => self.parse_postfix(),
            other => Err(ParseError {
                message: format!(
                    "élément inattendu en début d'expression: {:?}",
                    other
                ),
                line: self.current_line(),
                column: self.current_column(),
            }),
        }
    }

    fn parse_if_else(&mut self) -> Result<Expr, ParseError> {
        self.expect(TokenKind::If)?;
        let condition = self.parse_expression(0)?;
        let then_block = self.parse_block()?;
        self.expect(TokenKind::Else)?;
        let else_block = self.parse_block()?;
        Ok(self.expr(ExprKind::IfElse {
            condition: Box::new(condition),
            then_block,
            else_block,
        }))
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        match self.current_kind() {
            Some(TokenKind::IntLiteral(_)) => {
                let tok = self.bump().unwrap();
                if let TokenKind::IntLiteral(v) = tok.kind {
                    Ok(self.expr(ExprKind::IntLiteral(v)))
                } else {
                    Err(self.expected("litérale entière"))
                }
            }
            Some(TokenKind::FloatLiteral(_)) => {
                let tok = self.bump().unwrap();
                if let TokenKind::FloatLiteral(v) = tok.kind {
                    Ok(self.expr(ExprKind::FloatLiteral(v)))
                } else {
                    Err(self.expected("litérale flottante"))
                }
            }
            Some(TokenKind::True) => {
                self.bump();
                Ok(self.expr(ExprKind::BoolLiteral(true)))
            }
            Some(TokenKind::False) => {
                self.bump();
                Ok(self.expr(ExprKind::BoolLiteral(false)))
            }
            Some(TokenKind::Identifier(_)) => {
                let name = self.consume_identifier("nom")?;
                if self.check(TokenKind::LParen) {
                    self.bump();
                    let mut args = Vec::new();
                    if !self.check(TokenKind::RParen) {
                        loop {
                            args.push(self.parse_expression(0)?);
                            if self.check(TokenKind::Comma) {
                                self.bump();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(TokenKind::RParen)?;
                    Ok(self.expr(ExprKind::Call { name, args }))
                } else {
                    Ok(self.expr(ExprKind::Identifier(name)))
                }
            }
            Some(TokenKind::Load) => {
                self.expect(TokenKind::Load)?;
                self.expect(TokenKind::LParen)?;
                let expr = self.parse_expression(0)?;
                self.expect(TokenKind::RParen)?;
                Ok(self.expr(ExprKind::Load(Box::new(expr))))
            }
            Some(TokenKind::Free) => {
                self.expect(TokenKind::Free)?;
                self.expect(TokenKind::LParen)?;
                let expr = self.parse_expression(0)?;
                self.expect(TokenKind::RParen)?;
                Ok(self.expr(ExprKind::Free(Box::new(expr))))
            }
            Some(TokenKind::Alloc) => {
                self.expect(TokenKind::Alloc)?;
                self.expect(TokenKind::LParen)?;
                let expr = self.parse_expression(0)?;
                self.expect(TokenKind::RParen)?;
                Ok(self.expr(ExprKind::Alloc(Box::new(expr))))
            }
            Some(TokenKind::SizeOf) => {
                self.expect(TokenKind::SizeOf)?;
                self.expect(TokenKind::LParen)?;
                let ty = self.parse_type()?;
                self.expect(TokenKind::RParen)?;
                Ok(self.expr(ExprKind::SizeOf(ty)))
            }
            Some(TokenKind::LParen) => {
                self.expect(TokenKind::LParen)?;
                let expr = self.parse_expression(0)?;
                self.expect(TokenKind::RParen)?;
                Ok(expr)
            }
            other => Err(ParseError {
                message: format!("expression primaire attendue, trouvé {:?}", other),
                line: self.current_line(),
                column: self.current_column(),
            }),
        }
    }

    fn binary_meta(kind: Option<&TokenKind>) -> Option<(BinaryOp, u8, bool)> {
        match kind {
            Some(TokenKind::OrOr) => Some((BinaryOp::Or, 1, true)),
            Some(TokenKind::AndAnd) => Some((BinaryOp::And, 2, true)),
            Some(TokenKind::EqEq) => Some((BinaryOp::Eq, 3, true)),
            Some(TokenKind::NotEq) => Some((BinaryOp::NotEq, 3, true)),
            Some(TokenKind::Lt) => Some((BinaryOp::Lt, 4, true)),
            Some(TokenKind::LtEq) => Some((BinaryOp::LtEq, 4, true)),
            Some(TokenKind::Gt) => Some((BinaryOp::Gt, 4, true)),
            Some(TokenKind::GtEq) => Some((BinaryOp::GtEq, 4, true)),
            Some(TokenKind::Plus) => Some((BinaryOp::Add, 5, true)),
            Some(TokenKind::Minus) => Some((BinaryOp::Sub, 5, true)),
            Some(TokenKind::Star) => Some((BinaryOp::Mul, 6, true)),
            Some(TokenKind::Slash) => Some((BinaryOp::Div, 6, true)),
            Some(TokenKind::Percent) => Some((BinaryOp::Mod, 6, true)),
            _ => None,
        }
    }

    fn expr(&mut self, kind: ExprKind) -> Expr {
        let id = self.next_expr_id;
        self.next_expr_id += 1;
        Expr { id, kind }
    }

    fn consume_identifier(&mut self, label: &str) -> Result<String, ParseError> {
        match self.bump() {
            Some(Token {
                kind: TokenKind::Identifier(name),
                ..
            }) => Ok(name),
            Some(tok) => Err(ParseError {
                message: format!("{} attendu, trouvé {:?}", label, tok.kind),
                line: tok.line,
                column: tok.column,
            }),
            None => Err(ParseError {
                message: format!("{} attendu mais fin de fichier atteinte", label),
                line: 0,
                column: 0,
            }),
        }
    }

    fn expect(&mut self, expected: TokenKind) -> Result<(), ParseError> {
        let line = self.current_line();
        let column = self.current_column();
        match self.bump() {
            Some(tok) if tok.kind == expected => Ok(()),
            Some(tok) => Err(ParseError {
                message: format!("attendu {:?}, trouvé {:?}", expected, tok.kind),
                line,
                column,
            }),
            None => Err(ParseError {
                message: format!("attendu {:?}, fin de fichier atteinte", expected),
                line,
                column,
            }),
        }
    }

    fn check(&self, expected: TokenKind) -> bool {
        matches!(self.current(), Some(tok) if tok.kind == expected)
    }

    fn current_kind(&self) -> Option<&TokenKind> {
        self.current().map(|t| &t.kind)
    }

    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn bump(&mut self) -> Option<Token> {
        if self.pos >= self.tokens.len() {
            return None;
        }
        let tok = self.tokens[self.pos].clone();
        self.pos += 1;
        Some(tok)
    }

    fn current_line(&self) -> usize {
        self.current().map_or(0, |t| t.line)
    }

    fn current_column(&self) -> usize {
        self.current().map_or(0, |t| t.column)
    }

    fn expected(&self, msg: &str) -> ParseError {
        ParseError {
            message: format!("{} attendu", msg),
            line: self.current_line(),
            column: self.current_column(),
        }
    }
}
