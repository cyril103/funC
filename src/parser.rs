use crate::ast::{
    BinaryOp, Block, EnumDecl, Expr, ExprKind, Function, Parameter, Program, StructDecl,
    StructField, Type,
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
        let mut structs = Vec::new();
        let mut enums = Vec::new();
        let mut imports = Vec::new();
        while !self.check(TokenKind::Eof) {
            match self.current_kind() {
                Some(TokenKind::Import) => {
                    imports.push(self.parse_import()?);
                }
                Some(TokenKind::Struct) => {
                    structs.push(self.parse_struct_decl()?);
                }
                Some(TokenKind::Enum) => {
                    enums.push(self.parse_enum_decl()?);
                }
                _ => {
                    functions.push(self.parse_function()?);
                }
            }
        }
        Ok(Program {
            functions,
            structs,
            enums,
            imports,
        })
    }

    fn parse_struct_decl(&mut self) -> Result<StructDecl, ParseError> {
        self.expect(TokenKind::Struct)?;
        let name = self.consume_identifier("nom de structure")?;
        self.expect(TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while !self.check(TokenKind::RBrace) {
            let field_name = self.consume_identifier("nom de champ")?;
            self.expect(TokenKind::Colon)?;
            let ty = self.parse_type()?;
            self.expect(TokenKind::Semi)?;
            fields.push(StructField {
                name: field_name,
                ty,
            });
        }
        self.expect(TokenKind::RBrace)?;
        Ok(StructDecl { name, fields })
    }

    fn parse_enum_decl(&mut self) -> Result<EnumDecl, ParseError> {
        self.expect(TokenKind::Enum)?;
        let name = self.consume_identifier("nom d'énumération")?;
        self.expect(TokenKind::LBrace)?;
        let mut variants = Vec::new();
        if self.check(TokenKind::RBrace) {
            self.bump();
        } else {
            loop {
                variants.push(self.consume_identifier("nom de variante")?);
                if self.check(TokenKind::Comma) {
                    self.bump();
                } else {
                    break;
                }
            }
            self.expect(TokenKind::RBrace)?;
        }
        Ok(EnumDecl { name, variants })
    }

    fn parse_import(&mut self) -> Result<String, ParseError> {
        let line = self.current_line();
        let column = self.current_column();
        self.expect(TokenKind::Import)?;
        let raw = match self.bump() {
            Some(Token {
                kind: TokenKind::StringLiteral(value),
                ..
            }) => value,
            Some(tok) => {
                return Err(ParseError {
                    message: format!("chemin d'import attendu, trouvé {:?}", tok.kind),
                    line: tok.line,
                    column: tok.column,
                });
            }
            None => {
                return Err(ParseError {
                    message: "chemin d'import attendu mais fin de fichier atteinte".to_string(),
                    line,
                    column,
                });
            }
        };
        self.expect(TokenKind::Semi)?;
        Ok(raw)
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

        let mut base = if self.check(TokenKind::LBracket) {
            self.bump();
            let inner = self.parse_type()?;
            self.expect(TokenKind::Semi)?;
            let len_tok = self.bump().ok_or_else(|| ParseError {
                message: "longueur de tableau attendue".to_string(),
                line: self.current_line(),
                column: self.current_column(),
            })?;
            let len = match len_tok {
                Token {
                    kind: TokenKind::IntLiteral(len), ..
                } => usize::try_from(len).map_err(|_| ParseError {
                    message: "longueur de tableau invalide".to_string(),
                    line: len_tok.line,
                    column: len_tok.column,
                })?,
                token => {
                    return Err(ParseError {
                        message: format!("longueur de tableau attendue, trouvé {:?}", token.kind),
                        line: token.line,
                        column: token.column,
                    });
                }
            };
            self.expect(TokenKind::RBracket)?;
            Type::Array(Box::new(inner), len)
        } else {
            match self.bump() {
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
                Some(Token {
                    kind: TokenKind::Identifier(name), ..
                }) => Type::Struct(name),
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
            Some(TokenKind::Return) => {
                let stmt = self.parse_return()?;
                stmt
            }
            Some(TokenKind::For) => {
                let stmt = self.parse_for()?;
                stmt
            }
            Some(TokenKind::While) => {
                let stmt = self.parse_while()?;
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
        let (start_line, start_column) = self.current_position();
        self.expect(TokenKind::Let)?;
        let mutable = if self.check(TokenKind::Mut) {
            self.bump();
            true
        } else {
            false
        };
        let name = self.consume_identifier("nom de variable")?;
        let ty = if self.check(TokenKind::Colon) {
            self.bump();
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(TokenKind::Eq)?;
        let value = self.parse_expression(0)?;
        Ok(self.expr_at(start_line, start_column, ExprKind::Let {
            name,
            ty,
            value: Box::new(value),
            mutable,
        }))
    }

    fn parse_store(&mut self) -> Result<Expr, ParseError> {
        let (start_line, start_column) = self.current_position();
        self.expect(TokenKind::Store)?;
        self.expect(TokenKind::LParen)?;
        let value = self.parse_expression(0)?;
        self.expect(TokenKind::Comma)?;
        let ptr = self.parse_expression(0)?;
        self.expect(TokenKind::RParen)?;
        Ok(self.expr_at(start_line, start_column, ExprKind::Store(Box::new(value), Box::new(ptr))))
    }

    fn parse_expression(&mut self, min_prec: u8) -> Result<Expr, ParseError> {
        let (start_line, start_column) = self.current_position();
        let mut lhs = self.parse_unary_expression()?;
        if let ExprKind::Identifier(name) = &lhs.kind {
            if self.check(TokenKind::Eq) {
                self.bump();
                let rhs = self.parse_expression(0)?;
                lhs = self.expr_at(
                    start_line,
                    start_column,
                    ExprKind::Assign {
                        name: name.clone(),
                        value: Box::new(rhs),
                    },
                );
            }
        }
        while let Some((op, prec, left_assoc)) = Self::binary_meta(self.current_kind()) {
            if prec < min_prec {
                break;
            }
            self.bump();
            let next_min = if left_assoc { prec + 1 } else { prec };
            let rhs = self.parse_expression(next_min)?;
            lhs = self.expr(ExprKind::Binary(op, Box::new(lhs), Box::new(rhs)));
        }
        lhs = self.retag_expr(lhs, start_line, start_column);
        Ok(lhs)
    }

    fn parse_unary_expression(&mut self) -> Result<Expr, ParseError> {
        match self.current_kind() {
            Some(TokenKind::Not) => self.parse_not(),
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
        let (start_line, start_column) = self.current_position();
        self.expect(TokenKind::If)?;
        let condition = if self.check(TokenKind::LParen) {
            self.bump();
            let condition = self.parse_expression(0)?;
            self.expect(TokenKind::RParen)?;
            condition
        } else {
            self.parse_expression(0)?
        };
        let then_block = self.parse_block()?;
        self.expect(TokenKind::Else)?;
        let else_block = if self.check(TokenKind::If) {
            let else_if = self.parse_if_else()?;
            Block {
                expressions: vec![else_if],
            }
        } else {
            self.parse_block()?
        };
        Ok(self.expr_at(start_line, start_column, ExprKind::IfElse {
            condition: Box::new(condition),
            then_block,
            else_block,
        }))
    }

    fn parse_while(&mut self) -> Result<Expr, ParseError> {
        let (start_line, start_column) = self.current_position();
        self.expect(TokenKind::While)?;
        let condition = if self.check(TokenKind::LParen) {
            self.bump();
            let condition = self.parse_expression(0)?;
            self.expect(TokenKind::RParen)?;
            condition
        } else {
            self.parse_expression(0)?
        };
        let body = self.parse_block()?;
        Ok(self.expr_at(
            start_line,
            start_column,
            ExprKind::While {
                condition: Box::new(condition),
                body,
            },
        ))
    }

    fn parse_for(&mut self) -> Result<Expr, ParseError> {
        let (start_line, start_column) = self.current_position();
        self.expect(TokenKind::For)?;
        self.expect(TokenKind::LParen)?;

        let init = if self.check(TokenKind::Semi) {
            None
        } else {
            Some(Box::new(self.parse_expression(0)?))
        };
        self.expect(TokenKind::Semi)?;

        let condition = if self.check(TokenKind::Semi) {
            None
        } else {
            Some(Box::new(self.parse_expression(0)?))
        };
        self.expect(TokenKind::Semi)?;

        let post = if self.check(TokenKind::RParen) {
            None
        } else {
            Some(Box::new(self.parse_expression(0)?))
        };
        self.expect(TokenKind::RParen)?;

        let body = self.parse_block()?;
        Ok(self.expr_at(
            start_line,
            start_column,
            ExprKind::For {
                init,
                condition,
                post,
                body,
            },
        ))
    }

    fn parse_return(&mut self) -> Result<Expr, ParseError> {
        let (start_line, start_column) = self.current_position();
        self.expect(TokenKind::Return)?;
        let value = if self.check(TokenKind::Semi) {
            None
        } else {
            Some(Box::new(self.parse_expression(0)?))
        };
        self.expect(TokenKind::Semi)?;
        Ok(self.expr_at(start_line, start_column, ExprKind::Return(value)))
    }

    fn parse_not(&mut self) -> Result<Expr, ParseError> {
        let (start_line, start_column) = self.current_position();
        self.expect(TokenKind::Not)?;
        let expr = self.parse_unary_expression()?;
        Ok(self.expr_at(
            start_line,
            start_column,
            ExprKind::Not(Box::new(expr)),
        ))
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let (start_line, start_column) = self.current_position();
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
                let name = if self.check(TokenKind::Colon) {
                    self.bump();
                    self.expect(TokenKind::Colon)?;
                    let member = self.consume_identifier("nom du membre du namespace")?;
                    format!("{name}::{member}")
                } else {
                    name
                };

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
        }.and_then(|mut expr| {
            while self.check(TokenKind::LBracket) {
                self.bump();
                let index = self.parse_expression(0)?;
                self.expect(TokenKind::RBracket)?;
                expr = self.expr_at(
                    start_line,
                    start_column,
                    ExprKind::Index {
                        array: Box::new(expr),
                        index: Box::new(index),
                    },
                );
            }
            Ok(expr)
        })
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
        self.rebuild_expr(kind, self.current_position())
    }

    fn expr_at(&mut self, line: usize, column: usize, kind: ExprKind) -> Expr {
        self.rebuild_expr(kind, (line, column))
    }

    fn rebuild_expr(&mut self, kind: ExprKind, (line, column): (usize, usize)) -> Expr {
        let id = self.next_expr_id;
        self.next_expr_id += 1;
        Expr {
            id,
            line,
            column,
            kind,
        }
    }

    fn retag_expr(&self, mut expr: Expr, line: usize, column: usize) -> Expr {
        expr.line = line;
        expr.column = column;
        expr
    }

    fn current_position(&self) -> (usize, usize) {
        (self.current_line(), self.current_column())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    #[test]
    fn parse_operator_precedence_with_and_without_parentheses() {
        let source = "fn main() -> i64 { let a = 1 + 2 * 3; let b = (1 + 2) * 3; }";
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();

        let first_let = &program.functions[0].body.expressions[0];
        let second_let = &program.functions[0].body.expressions[1];

        let (left_a, right_a) = match &first_let.kind {
            ExprKind::Let {
                name,
                value,
                ..
            } => {
                assert_eq!(name, "a");
                match &value.kind {
                    ExprKind::Binary(op, left, right) => (op, (left, right)),
                    _ => panic!("expression inattendue pour let a"),
                }
            }
            _ => panic!("première expression pas un let"),
        };
        assert_eq!(*left_a, BinaryOp::Add);
        assert!(matches!(right_a.0.kind, ExprKind::Binary(BinaryOp::Mul, _, _)));

        let (left_b, right_b, op_b) = match &second_let.kind {
            ExprKind::Let {
                name,
                value,
                ..
            } => {
                assert_eq!(name, "b");
                match &value.kind {
                    ExprKind::Binary(op, left, right) => (left, right, op),
                    _ => panic!("expression inattendue pour let b"),
                }
            }
            _ => panic!("deuxième expression pas un let"),
        };
        assert_eq!(*op_b, BinaryOp::Mul);
        assert!(matches!(left_b.kind, ExprKind::Binary(BinaryOp::Add, _, _)));
    }

    #[test]
    fn parse_nested_if_else_blocks() {
        let source =
            "fn main() -> i64 { if 1 < 2 { 1; } else { if 3 < 4 { 2; } else { 3; } } }";
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();

        let top_if = &program.functions[0].body.expressions[0];
        match &top_if.kind {
            ExprKind::IfElse {
                condition,
                then_block,
                else_block,
            } => {
                assert!(matches!(
                    condition.kind,
                    ExprKind::Binary(BinaryOp::Lt, _, _)
                ));
                assert!(matches!(
                    then_block.expressions[0].kind,
                    ExprKind::IntLiteral(1)
                ));
                assert!(matches!(
                    else_block.expressions[0].kind,
                    ExprKind::IfElse { .. }
                ));
            }
            _ => panic!("expression racine pas un if-else"),
        }
    }

    #[test]
    fn parse_else_if_chain() {
        let source =
            "fn main() -> i64 { if 1 < 2 { 1; } else if 2 < 3 { 2; } else { 3; } }";
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();

        let top_if = &program.functions[0].body.expressions[0];
        match &top_if.kind {
            ExprKind::IfElse {
                then_block,
                else_block,
                ..
            } => {
                assert!(matches!(
                    then_block.expressions[0].kind,
                    ExprKind::IntLiteral(1)
                ));
                assert_eq!(else_block.expressions.len(), 1);
                assert!(matches!(
                    else_block.expressions[0].kind,
                    ExprKind::IfElse { .. }
                ));
            }
            _ => panic!("expression racine pas un if-else"),
        }
    }

    #[test]
    fn parse_if_with_parenthesized_condition() {
        let source =
            "fn main() -> i64 { if (1 < 2) { 1; } else { 2; } }";
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();

        let top_if = &program.functions[0].body.expressions[0];
        match &top_if.kind {
            ExprKind::IfElse { condition, .. } => {
                assert!(matches!(condition.kind, ExprKind::Binary(BinaryOp::Lt, _, _)));
            }
            _ => panic!("expression racine pas un if-else"),
        }
    }

    #[test]
    fn parse_not_operator() {
        let source = "fn main() -> bool { !true && false; }";
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();

        let expr = &program.functions[0].body.expressions[0];
        match &expr.kind {
            ExprKind::Binary(BinaryOp::And, lhs, rhs) => {
                assert!(matches!(lhs.kind, ExprKind::Not(_)));
                assert!(matches!(rhs.kind, ExprKind::BoolLiteral(false)));
            }
            _ => panic!("l'expression racine doit être un &&"),
        }
    }

    #[test]
    fn parse_logical_precedence_and_left_associativity() {
        let source = "fn main() -> bool { true && false && true || false; }";
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();

        let expr = &program.functions[0].body.expressions[0];
        let top = match &expr.kind {
            ExprKind::Binary(op, left, right) => (op, left, right),
            _ => panic!("l'expression racine doit être binaire"),
        };
        assert_eq!(*top.0, BinaryOp::Or);
        assert!(matches!(top.1.kind, ExprKind::Binary(BinaryOp::And, _, _)));
        assert!(matches!(top.2.kind, ExprKind::BoolLiteral(_)));

        if let ExprKind::Binary(BinaryOp::And, and_left, and_right) = &top.1.kind {
            assert!(matches!(and_left.kind, ExprKind::Binary(BinaryOp::And, _, _)));
            assert!(matches!(and_right.kind, ExprKind::BoolLiteral(false)));
        } else {
            panic!("attendu and gauche imbriqué");
        }
    }

    #[test]
    fn parse_while_loop() {
        let source = "fn main() -> i64 { while x < 3 { let x = 0; x; } }";
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();

        let expr = &program.functions[0].body.expressions[0];
        match &expr.kind {
            ExprKind::While { condition, body } => {
                assert!(matches!(condition.kind, ExprKind::Binary(_, _, _)));
                assert_eq!(body.expressions.len(), 2);
            }
            _ => panic!("expression racine pas un while"),
        }
    }

    #[test]
    fn parse_for_loop() {
        let source = "fn main() -> i64 { let x = 0; for (; x < 3; x) { x; } }";
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();

        let expr = &program.functions[0].body.expressions[1];
        match &expr.kind {
            ExprKind::For {
                init,
                condition,
                post,
                body,
            } => {
                assert!(init.is_none());
                assert!(condition.is_some());
                assert!(post.is_some());
                assert_eq!(body.expressions.len(), 1);
            }
            _ => panic!("expression racine pas un for"),
        }
    }

    #[test]
    fn parse_return_statement() {
        let source = "fn main() -> i64 { let x = 1; return x; }";
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();

        let expr = &program.functions[0].body.expressions[1];
        match &expr.kind {
            ExprKind::Return(value) => {
                assert!(value.is_some());
            }
            _ => panic!("expression racine pas un return"),
        }
    }

    #[test]
    fn parse_mutable_let_and_assignment() {
        let source = "fn main() -> i64 { let mut x: i64 = 1; x = 2; x; }";
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();

        let decl = &program.functions[0].body.expressions[0];
        match &decl.kind {
            ExprKind::Let { name, mutable, .. } => {
                assert_eq!(name, "x");
                assert!(*mutable);
            }
            _ => panic!("déclaration attendue comme let mut"),
        }

        match &program.functions[0].body.expressions[1].kind {
            ExprKind::Assign { name, .. } => assert_eq!(name, "x"),
            _ => panic!("assignment attendue"),
        }
    }

    #[test]
    fn parse_import_statement_collects_path() {
        let source = "import \"math\"; fn main() -> i64 { 0 }";
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();
        assert_eq!(program.imports, vec!["math".to_string()]);
        assert_eq!(program.functions.len(), 1);
        assert_eq!(program.functions[0].name, "main");
    }

    #[test]
    fn parse_std_namespace_call() {
        let source = "fn main() -> i64 { let p = func::alloc(8); return 0; }";
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();

        let first = &program.functions[0].body.expressions[0].kind;
        match first {
            ExprKind::Let { value, .. } => {
                let call = &value.kind;
                match call {
                    ExprKind::Call { name, args } => {
                        assert_eq!(name, "func::alloc");
                        assert_eq!(args.len(), 1);
                    }
                    _ => panic!("assignation attendue comme appel func::alloc"),
                }
            }
            _ => panic!("expression première attendue comme let"),
        }
    }

    #[test]
    fn parse_array_type_in_parameters() {
        let source = "fn main(values: [i64; 4]) -> void { 0 }";
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();

        let func = &program.functions[0];
        match &func.params[0].ty {
            Type::Array(inner, len) => {
                assert_eq!(*len, 4);
                assert_eq!(inner.as_ref(), &Type::I64);
            }
            _ => panic!("type de paramètre attendu en tableau"),
        }
    }

    #[test]
    fn parse_array_index() {
        let source = "fn main(values: [i64; 4]) -> i64 { return values[2]; }";
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();

        let return_expr = &program.functions[0].body.expressions[0];
        match &return_expr.kind {
            ExprKind::Return(expr) => match expr {
                Some(expr) => match &expr.kind {
                    ExprKind::Index { array, index } => {
                        assert!(matches!(array.kind, ExprKind::Identifier(_)));
                        assert!(matches!(index.kind, ExprKind::IntLiteral(2)));
                    }
                    _ => panic!("attendu un indexage dans le return"),
                },
                None => panic!("return sans valeur"),
            },
            _ => panic!("première expression doit être return"),
        }
    }

    #[test]
    fn parse_struct_declaration() {
        let source = "struct Point { x: i32; y: *i8; } fn main() -> void { 0 }";
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();

        assert_eq!(program.structs.len(), 1);
        let struct_decl = &program.structs[0];
        assert_eq!(struct_decl.name, "Point");
        assert_eq!(struct_decl.fields.len(), 2);
        assert_eq!(struct_decl.fields[0].name, "x");
        assert_eq!(struct_decl.fields[0].ty, Type::I32);
        assert_eq!(
            struct_decl.fields[1].ty,
            Type::Pointer(Box::new(Type::I8))
        );
    }

    #[test]
    fn parse_enum_declaration() {
        let source = "enum Color { Red, Green, Blue } fn main() -> void { 0 }";
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();

        assert_eq!(program.enums.len(), 1);
        let enum_decl = &program.enums[0];
        assert_eq!(enum_decl.name, "Color");
        assert_eq!(enum_decl.variants, vec!["Red", "Green", "Blue"]);
    }
}
