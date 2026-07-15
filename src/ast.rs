use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub functions: Vec<Function>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Type,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub expressions: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub id: usize,
    pub kind: ExprKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Let {
        name: String,
        ty: Option<Type>,
        value: Box<Expr>,
    },
    Store(Box<Expr>, Box<Expr>),
    IfElse {
        condition: Box<Expr>,
        then_block: Block,
        else_block: Block,
    },
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
    Identifier(String),
    IntLiteral(i64),
    FloatLiteral(f64),
    BoolLiteral(bool),
    Call {
        name: String,
        args: Vec<Expr>,
    },
    Alloc(Box<Expr>),
    Free(Box<Expr>),
    Load(Box<Expr>),
    SizeOf(Type),
    Block(Block),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Or,
    And,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Void,
    Bool,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Pointer(Box<Type>),
}

impl Type {
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::U8 | Type::U16 | Type::U32 | Type::U64,
        )
    }

    pub fn is_signed_integer(&self) -> bool {
        matches!(self, Type::I8 | Type::I16 | Type::I32 | Type::I64)
    }

    pub fn is_float(&self) -> bool {
        matches!(self, Type::F32 | Type::F64)
    }

    pub fn is_numeric(&self) -> bool {
        self.is_integer() || self.is_float()
    }

    pub fn llvm_name(&self) -> String {
        match self {
            Type::Void => "void".to_string(),
            Type::Bool => "i1".to_string(),
            Type::I8 => "i8".to_string(),
            Type::I16 => "i16".to_string(),
            Type::I32 => "i32".to_string(),
            Type::I64 => "i64".to_string(),
            Type::U8 => "i8".to_string(),
            Type::U16 => "i16".to_string(),
            Type::U32 => "i32".to_string(),
            Type::U64 => "i64".to_string(),
            Type::F32 => "float".to_string(),
            Type::F64 => "double".to_string(),
            Type::Pointer(inner) => format!("{}*", inner.llvm_name()),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Void => write!(f, "void"),
            Type::Bool => write!(f, "bool"),
            Type::I8 => write!(f, "i8"),
            Type::I16 => write!(f, "i16"),
            Type::I32 => write!(f, "i32"),
            Type::I64 => write!(f, "i64"),
            Type::U8 => write!(f, "u8"),
            Type::U16 => write!(f, "u16"),
            Type::U32 => write!(f, "u32"),
            Type::U64 => write!(f, "u64"),
            Type::F32 => write!(f, "f32"),
            Type::F64 => write!(f, "f64"),
            Type::Pointer(inner) => write!(f, "*{}", inner),
        }
    }
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            BinaryOp::Or => "||",
            BinaryOp::And => "&&",
            BinaryOp::Eq => "==",
            BinaryOp::NotEq => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::LtEq => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::GtEq => ">=",
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Mod => "%",
        };
        write!(f, "{}", text)
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.format(f, 0)
    }
}

impl Expr {
    pub fn format(&self, f: &mut fmt::Formatter<'_>, indent: usize) -> fmt::Result {
        let pad = "  ".repeat(indent);
        match &self.kind {
            ExprKind::Let { name, ty, value } => {
                if let Some(ty) = ty {
                    write!(f, "{}let {}: {} = ", pad, name, ty)?;
                } else {
                    write!(f, "{}let {} = ", pad, name)?;
                }
                value.format(f, 0)?;
            }
            ExprKind::Store(value, ptr) => {
                write!(f, "{}store(", pad)?;
                value.format(f, 0)?;
                write!(f, ", ")?;
                ptr.format(f, 0)?;
                write!(f, ")")?;
            }
            ExprKind::IfElse {
                condition,
                then_block,
                else_block,
            } => {
                write!(f, "{}if ", pad)?;
                condition.format(f, 0)?;
                writeln!(f, " {{")?;
                for expr in &then_block.expressions {
                    expr.format(f, indent + 1)?;
                    writeln!(f, ";")?;
                }
                write!(f, "{}}} else {{", pad)?;
                for expr in &else_block.expressions {
                    writeln!(f)?;
                    expr.format(f, indent + 1)?;
                    writeln!(f, ";")?;
                }
                write!(f, "{}}}", pad)?;
            }
            ExprKind::Binary(op, lhs, rhs) => {
                lhs.format(f, 0)?;
                write!(f, " {} ", op)?;
                rhs.format(f, 0)?;
            }
            ExprKind::Identifier(name) => write!(f, "{}{}", pad, name)?,
            ExprKind::IntLiteral(value) => write!(f, "{}{}", pad, value)?,
            ExprKind::FloatLiteral(value) => write!(f, "{}{}", pad, value)?,
            ExprKind::BoolLiteral(value) => write!(f, "{}{}", pad, value)?,
            ExprKind::Call { name, args } => {
                write!(f, "{}{}(", pad, name)?;
                for (idx, arg) in args.iter().enumerate() {
                    if idx > 0 {
                        write!(f, ", ")?;
                    }
                    arg.format(f, 0)?;
                }
                write!(f, ")")?;
            }
            ExprKind::Alloc(size) => {
                write!(f, "{}alloc(", pad)?;
                size.format(f, 0)?;
                write!(f, ")")?;
            }
            ExprKind::Free(ptr) => {
                write!(f, "{}free(", pad)?;
                ptr.format(f, 0)?;
                write!(f, ")")?;
            }
            ExprKind::Load(ptr) => {
                write!(f, "{}load(", pad)?;
                ptr.format(f, 0)?;
                write!(f, ")")?;
            }
            ExprKind::SizeOf(ty) => {
                write!(f, "{}sizeof({})", pad, ty)?;
            }
            ExprKind::Block(block) => {
                writeln!(f, "{}{{", pad)?;
                for expr in &block.expressions {
                    expr.format(f, indent + 1)?;
                    writeln!(f, ";")?;
                }
                write!(f, "{}}}", pad)?;
            }
        }
        Ok(())
    }
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fn {}(", self.name)?;
        for (idx, param) in self.params.iter().enumerate() {
            if idx > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}: {}", param.name, param.ty)?;
        }
        writeln!(f, ") -> {} {{", self.return_type)?;
        for expr in &self.body.expressions {
            expr.format(f, 1)?;
            writeln!(f, ";")?;
        }
        write!(f, "}}")
    }
}

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (idx, function) in self.functions.iter().enumerate() {
            if idx > 0 {
                writeln!(f)?;
                writeln!(f)?;
            }
            writeln!(f, "{}", function)?;
        }
        Ok(())
    }
}
