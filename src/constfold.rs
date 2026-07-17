use crate::ast::{BinaryOp, Block, Expr, ExprKind, Program};

pub fn fold_program(program: &mut Program) {
    for function in &mut program.functions {
        fold_block(&mut function.body);
    }
}

fn fold_block(block: &mut Block) {
    for expr in &mut block.expressions {
        fold_expr(expr);
    }
}

fn fold_expr(expr: &mut Expr) {
    match &mut expr.kind {
        ExprKind::Let { value, .. } => fold_expr(value),
        ExprKind::Assign { value, .. } => fold_expr(value),
        ExprKind::Store(value, ptr) => {
            fold_expr(value);
            fold_expr(ptr);
        }
        ExprKind::Free(ptr) => fold_expr(ptr),
        ExprKind::For {
            init,
            condition,
            post,
            body,
        } => {
            if let Some(init) = init.as_mut() {
                fold_expr(init);
            }
            if let Some(condition) = condition.as_mut() {
                fold_expr(condition);
            }
            if let Some(post) = post.as_mut() {
                fold_expr(post);
            }
            fold_block(body);
        }
        ExprKind::Return(value) => {
            if let Some(value) = value.as_mut() {
                fold_expr(value);
            }
        }
        ExprKind::While { condition, body } => {
            fold_expr(condition);
            fold_block(body);
        }
        ExprKind::IfElse {
            condition,
            then_block,
            else_block,
        } => {
            fold_expr(condition);
            fold_block(then_block);
            fold_block(else_block);
        }
        ExprKind::Not(expr) => {
            fold_expr(expr);
            if let ExprKind::BoolLiteral(value) = expr.kind {
                expr.kind = ExprKind::BoolLiteral(!value);
            }
        }
        ExprKind::Binary(op, left, right) => {
            fold_expr(left);
            fold_expr(right);

            if let Some(folded) = fold_binary(*op, &left.kind, &right.kind) {
                expr.kind = folded;
            }
        }
        ExprKind::Load(ptr) => fold_expr(ptr),
        ExprKind::Index { array, index } => {
            fold_expr(array);
            fold_expr(index);
        }
        ExprKind::Alloc(size) => fold_expr(size),
        ExprKind::Call { args, .. } => {
            for arg in args.iter_mut() {
                fold_expr(arg);
            }
        }
        ExprKind::Block(block) => fold_block(block),
        ExprKind::Identifier(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::SizeOf(_) => {}
    }
}

fn fold_binary(op: BinaryOp, left: &ExprKind, right: &ExprKind) -> Option<ExprKind> {
    let from_identity = match (op, left, right) {
        (BinaryOp::And, ExprKind::BoolLiteral(true), _) => Some(right.clone()),
        (BinaryOp::And, ExprKind::BoolLiteral(false), _) => Some(ExprKind::BoolLiteral(false)),
        (BinaryOp::And, _, ExprKind::BoolLiteral(true)) => Some(left.clone()),
        (BinaryOp::And, _, ExprKind::BoolLiteral(false)) => Some(ExprKind::BoolLiteral(false)),
        (BinaryOp::Or, ExprKind::BoolLiteral(true), _) => Some(ExprKind::BoolLiteral(true)),
        (BinaryOp::Or, ExprKind::BoolLiteral(false), _) => Some(right.clone()),
        (BinaryOp::Or, _, ExprKind::BoolLiteral(true)) => Some(ExprKind::BoolLiteral(true)),
        (BinaryOp::Or, _, ExprKind::BoolLiteral(false)) => Some(left.clone()),
        _ => None,
    };
    if from_identity.is_some() {
        return from_identity;
    }

    match (op, left, right) {
        (BinaryOp::Add, ExprKind::IntLiteral(lhs), ExprKind::IntLiteral(rhs)) => {
            Some(ExprKind::IntLiteral(lhs.wrapping_add(*rhs)))
        }
        (BinaryOp::Sub, ExprKind::IntLiteral(lhs), ExprKind::IntLiteral(rhs)) => {
            Some(ExprKind::IntLiteral(lhs.wrapping_sub(*rhs)))
        }
        (BinaryOp::Mul, ExprKind::IntLiteral(lhs), ExprKind::IntLiteral(rhs)) => {
            Some(ExprKind::IntLiteral(lhs.wrapping_mul(*rhs)))
        }
        (BinaryOp::Div, ExprKind::IntLiteral(_lhs), ExprKind::IntLiteral(0)) => None,
        (BinaryOp::Div, ExprKind::IntLiteral(lhs), ExprKind::IntLiteral(rhs)) if *lhs == i64::MIN && *rhs == -1 => None,
        (BinaryOp::Div, ExprKind::IntLiteral(lhs), ExprKind::IntLiteral(rhs)) => {
            Some(ExprKind::IntLiteral(lhs / rhs))
        }
        (BinaryOp::Mod, ExprKind::IntLiteral(_lhs), ExprKind::IntLiteral(0)) => None,
        (BinaryOp::Mod, ExprKind::IntLiteral(lhs), ExprKind::IntLiteral(rhs)) => {
            Some(ExprKind::IntLiteral(lhs % rhs))
        }
        (BinaryOp::Add, ExprKind::FloatLiteral(lhs), ExprKind::FloatLiteral(rhs)) => {
            Some(ExprKind::FloatLiteral(lhs + rhs))
        }
        (BinaryOp::Sub, ExprKind::FloatLiteral(lhs), ExprKind::FloatLiteral(rhs)) => {
            Some(ExprKind::FloatLiteral(lhs - rhs))
        }
        (BinaryOp::Mul, ExprKind::FloatLiteral(lhs), ExprKind::FloatLiteral(rhs)) => {
            Some(ExprKind::FloatLiteral(lhs * rhs))
        }
        (BinaryOp::Div, ExprKind::FloatLiteral(lhs), ExprKind::FloatLiteral(rhs)) => {
            Some(ExprKind::FloatLiteral(lhs / rhs))
        }
        (BinaryOp::Mod, ExprKind::FloatLiteral(lhs), ExprKind::FloatLiteral(rhs)) => {
            Some(ExprKind::FloatLiteral(lhs % rhs))
        }
        (BinaryOp::Eq, ExprKind::BoolLiteral(lhs), ExprKind::BoolLiteral(rhs)) => {
            Some(ExprKind::BoolLiteral(lhs == rhs))
        }
        (BinaryOp::NotEq, ExprKind::BoolLiteral(lhs), ExprKind::BoolLiteral(rhs)) => {
            Some(ExprKind::BoolLiteral(lhs != rhs))
        }
        (BinaryOp::Or, ExprKind::BoolLiteral(lhs), ExprKind::BoolLiteral(rhs)) => {
            Some(ExprKind::BoolLiteral(*lhs || *rhs))
        }
        (BinaryOp::And, ExprKind::BoolLiteral(lhs), ExprKind::BoolLiteral(rhs)) => {
            Some(ExprKind::BoolLiteral(*lhs && *rhs))
        }
        (BinaryOp::Eq, ExprKind::IntLiteral(lhs), ExprKind::IntLiteral(rhs)) => {
            Some(ExprKind::BoolLiteral(lhs == rhs))
        }
        (BinaryOp::NotEq, ExprKind::IntLiteral(lhs), ExprKind::IntLiteral(rhs)) => {
            Some(ExprKind::BoolLiteral(lhs != rhs))
        }
        (BinaryOp::Lt, ExprKind::IntLiteral(lhs), ExprKind::IntLiteral(rhs)) => {
            Some(ExprKind::BoolLiteral(lhs < rhs))
        }
        (BinaryOp::LtEq, ExprKind::IntLiteral(lhs), ExprKind::IntLiteral(rhs)) => {
            Some(ExprKind::BoolLiteral(lhs <= rhs))
        }
        (BinaryOp::Gt, ExprKind::IntLiteral(lhs), ExprKind::IntLiteral(rhs)) => {
            Some(ExprKind::BoolLiteral(lhs > rhs))
        }
        (BinaryOp::GtEq, ExprKind::IntLiteral(lhs), ExprKind::IntLiteral(rhs)) => {
            Some(ExprKind::BoolLiteral(lhs >= rhs))
        }
        (BinaryOp::Eq, ExprKind::FloatLiteral(lhs), ExprKind::FloatLiteral(rhs)) => {
            Some(ExprKind::BoolLiteral(lhs == rhs))
        }
        (BinaryOp::NotEq, ExprKind::FloatLiteral(lhs), ExprKind::FloatLiteral(rhs)) => {
            Some(ExprKind::BoolLiteral(lhs != rhs))
        }
        (BinaryOp::Lt, ExprKind::FloatLiteral(lhs), ExprKind::FloatLiteral(rhs)) => {
            Some(ExprKind::BoolLiteral(lhs < rhs))
        }
        (BinaryOp::LtEq, ExprKind::FloatLiteral(lhs), ExprKind::FloatLiteral(rhs)) => {
            Some(ExprKind::BoolLiteral(lhs <= rhs))
        }
        (BinaryOp::Gt, ExprKind::FloatLiteral(lhs), ExprKind::FloatLiteral(rhs)) => {
            Some(ExprKind::BoolLiteral(lhs > rhs))
        }
        (BinaryOp::GtEq, ExprKind::FloatLiteral(lhs), ExprKind::FloatLiteral(rhs)) => {
            Some(ExprKind::BoolLiteral(lhs >= rhs))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    use super::fold_program;
    use crate::ast::Program;

    fn parse_program(source: &str) -> Program {
        let tokens = Lexer::new(source).tokenize().unwrap();
        Parser::new(tokens).parse_program().unwrap()
    }

    #[test]
    fn fold_simple_integer_addition() {
        let mut program = parse_program("fn main() -> i64 { return 2 + 3; }");
        fold_program(&mut program);

        let return_expr = match &program.functions[0].body.expressions[0].kind {
            crate::ast::ExprKind::Return(Some(expr)) => &expr.kind,
            _ => panic!("expression de retour inattendue"),
        };
        assert!(matches!(return_expr, crate::ast::ExprKind::IntLiteral(5)));
    }

    #[test]
    fn fold_true_and_expression() {
        let mut program = parse_program("fn main(x: bool) -> bool { return true && x; }");
        fold_program(&mut program);

        let return_expr = match &program.functions[0].body.expressions[0].kind {
            crate::ast::ExprKind::Return(Some(expr)) => &expr.kind,
            _ => panic!("expression de retour inattendue"),
        };
        assert!(matches!(return_expr, crate::ast::ExprKind::Identifier(name) if name == "x"));
    }

    #[test]
    fn fold_expression_or_false() {
        let mut program = parse_program("fn main(x: bool) -> bool { return x || false; }");
        fold_program(&mut program);

        let return_expr = match &program.functions[0].body.expressions[0].kind {
            crate::ast::ExprKind::Return(Some(expr)) => &expr.kind,
            _ => panic!("expression de retour inattendue"),
        };
        assert!(matches!(return_expr, crate::ast::ExprKind::Identifier(name) if name == "x"));
    }
}
