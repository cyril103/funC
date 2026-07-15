use crate::token::{LexError, Token, TokenKind};

#[derive(Debug)]
pub struct Lexer {
    chars: Vec<char>,
    idx: usize,
    line: usize,
    column: usize,
    next_id: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            chars: input.chars().collect(),
            idx: 0,
            line: 1,
            column: 1,
            next_id: 0,
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        while let Some(ch) = self.peek() {
            self.skip_ws_and_comments();
            if self.idx >= self.chars.len() {
                break;
            }
            let line = self.line;
            let column = self.column;
            let tok = match ch {
                'a'..='z' | 'A'..='Z' | '_' => self.lex_identifier()?,
                '0'..='9' => self.lex_number()?,
                '+' => {
                    self.bump();
                    TokenKind::Plus
                }
                '-' => {
                    if self.peek_n(1) == Some('>') {
                        self.bump_n(2);
                        TokenKind::Arrow
                    } else {
                        self.bump();
                        TokenKind::Minus
                    }
                }
                '*' => {
                    self.bump();
                    TokenKind::Star
                }
                '%' => {
                    self.bump();
                    TokenKind::Percent
                }
                '/' => {
                    self.bump();
                    TokenKind::Slash
                }
                '(' => {
                    self.bump();
                    TokenKind::LParen
                }
                ')' => {
                    self.bump();
                    TokenKind::RParen
                }
                '{' => {
                    self.bump();
                    TokenKind::LBrace
                }
                '}' => {
                    self.bump();
                    TokenKind::RBrace
                }
                ':' => {
                    self.bump();
                    TokenKind::Colon
                }
                ';' => {
                    self.bump();
                    TokenKind::Semi
                }
                ',' => {
                    self.bump();
                    TokenKind::Comma
                }
                '=' => {
                    self.bump();
                    if self.match_next('=') {
                        self.bump();
                        TokenKind::EqEq
                    } else {
                        TokenKind::Eq
                    }
                }
                '!' => {
                    self.bump();
                    if self.match_next('=') {
                        self.bump();
                        TokenKind::NotEq
                    } else {
                        return Err(LexError {
                            message: "caractère inattendu '!' (attendu '!=')".to_string(),
                            line,
                            column,
                        });
                    }
                }
                '<' => {
                    self.bump();
                    if self.match_next('=') {
                        self.bump();
                        TokenKind::LtEq
                    } else {
                        TokenKind::Lt
                    }
                }
                '>' => {
                    self.bump();
                    if self.match_next('=') {
                        self.bump();
                        TokenKind::GtEq
                    } else {
                        TokenKind::Gt
                    }
                }
                '&' => {
                    self.bump();
                    if self.match_next('&') {
                        self.bump();
                        TokenKind::AndAnd
                    } else {
                        return Err(LexError {
                            message: "caractère inattendu '&' (attendu '&&')".to_string(),
                            line,
                            column,
                        });
                    }
                }
                '|' => {
                    self.bump();
                    if self.match_next('|') {
                        self.bump();
                        TokenKind::OrOr
                    } else {
                        return Err(LexError {
                            message: "caractère inattendu '|' (attendu '||')".to_string(),
                            line,
                            column,
                        });
                    }
                }
                _ => {
                    return Err(LexError {
                        message: format!("caractère inattendu '{}" , ch),
                        line,
                        column,
                    });
                }
            };
            tokens.push(Token {
                kind: tok,
                line,
                column,
            });
        }
        tokens.push(Token {
            kind: TokenKind::Eof,
            line: self.line,
            column: self.column,
        });
        Ok(tokens)
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            match self.peek() {
                Some(ch) if ch.is_whitespace() => {
                    self.bump();
                }
                Some('/') if self.peek_n(1) == Some('/') => {
                    self.bump_n(2);
                    while let Some(ch) = self.peek() {
                        if ch == '\n' {
                            break;
                        }
                        self.bump();
                    }
                }
                Some('/') if self.peek_n(1) == Some('*') => {
                    self.bump_n(2);
                    while let Some(ch) = self.peek() {
                        if ch == '*' && self.peek_n(1) == Some('/') {
                            self.bump_n(2);
                            break;
                        }
                        self.bump();
                    }
                }
                _ => break,
            }
        }
    }

    fn lex_identifier(&mut self) -> Result<TokenKind, LexError> {
        let start = self.idx;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                self.bump();
            } else {
                break;
            }
        }
        let txt: String = self.chars[start..self.idx].iter().collect();
        match txt.as_str() {
            "fn" => Ok(TokenKind::Fn),
            "let" => Ok(TokenKind::Let),
            "if" => Ok(TokenKind::If),
            "else" => Ok(TokenKind::Else),
            "alloc" => Ok(TokenKind::Alloc),
            "free" => Ok(TokenKind::Free),
            "load" => Ok(TokenKind::Load),
            "store" => Ok(TokenKind::Store),
            "sizeof" => Ok(TokenKind::SizeOf),
            "i8" => Ok(TokenKind::I8),
            "i16" => Ok(TokenKind::I16),
            "i32" => Ok(TokenKind::I32),
            "i64" => Ok(TokenKind::I64),
            "u8" => Ok(TokenKind::U8),
            "u16" => Ok(TokenKind::U16),
            "u32" => Ok(TokenKind::U32),
            "u64" => Ok(TokenKind::U64),
            "f32" => Ok(TokenKind::F32),
            "f64" => Ok(TokenKind::F64),
            "bool" => Ok(TokenKind::Bool),
            "void" => Ok(TokenKind::Void),
            "true" => Ok(TokenKind::True),
            "false" => Ok(TokenKind::False),
            _ => Ok(TokenKind::Identifier(txt)),
        }
    }

    fn lex_number(&mut self) -> Result<TokenKind, LexError> {
        let start = self.idx;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                self.bump();
            } else {
                break;
            }
        }

        let mut is_float = false;
        if self.peek() == Some('.') && self.peek_n(1).map_or(false, |c| c.is_ascii_digit()) {
            self.bump();
            is_float = true;
            while let Some(ch) = self.peek() {
                if ch.is_ascii_digit() {
                    self.bump();
                } else {
                    break;
                }
            }
        }

        let raw: String = self.chars[start..self.idx].iter().collect();
        if is_float {
            let value = raw
                .parse::<f64>()
                .map_err(|e| LexError {
                    message: format!("nombre flottant invalide '{raw}' ({e})"),
                    line: self.line,
                    column: self.column,
                })?;
            Ok(TokenKind::FloatLiteral(value))
        } else {
            let value = raw
                .parse::<i64>()
                .map_err(|e| LexError {
                    message: format!("entier invalide '{raw}' ({e})"),
                    line: self.line,
                    column: self.column,
                })?;
            Ok(TokenKind::IntLiteral(value))
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.idx).copied()
    }

    fn peek_n(&self, n: usize) -> Option<char> {
        self.chars.get(self.idx + n).copied()
    }

    fn match_next(&self, expected: char) -> bool {
        self.peek_n(0) == Some(expected)
    }

    fn bump(&mut self) {
        if let Some(ch) = self.peek() {
            self.idx += 1;
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
    }

    fn bump_n(&mut self, n: usize) {
        for _ in 0..n {
            self.bump();
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}
