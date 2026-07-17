use std::collections::HashMap;
use std::mem;

use crate::ast::{BinaryOp, Block, Expr, ExprKind, Program};

pub fn fold_program(program: &mut Program) {
    for function in &mut program.functions {
        let mut scopes: Vec<HashMap<String, ExprKind>> = Vec::new();
        fold_block(&mut function.body, &mut scopes);
    }
}

fn fold_block(block: &mut Block, scopes: &mut Vec<HashMap<String, ExprKind>>) {
    scopes.push(HashMap::new());
    let expressions = mem::take(&mut block.expressions);
    let mut optimized = Vec::new();
    let mut keep_processing = true;

    for expr in expressions {
        let mut expr = expr;
        fold_expr(&mut expr, scopes);

        if keep_processing {
            if matches!(expr.kind, ExprKind::Return(_)) {
                keep_processing = false;
            }
            optimized.push(expr);
        }
    }

    block.expressions = optimized;
    scopes.pop();
}

fn fold_expr(expr: &mut Expr, scopes: &mut Vec<HashMap<String, ExprKind>>) {
    match &mut expr.kind {
        ExprKind::Let {
            name,
            value,
            mutable,
            ..
        } => {
            fold_expr(value, scopes);
            if *mutable {
                if let Some(scope_index) = index_of_binding(scopes, name) {
                    scopes[scope_index].remove(name);
                }
            } else if let Some(constant) = infer_constant(&value.kind) {
                if let Some(scope) = scopes.last_mut() {
                    scope.insert(name.clone(), constant);
                }
            }
        }
        ExprKind::Assign { name, value } => {
            fold_expr(value, scopes);
            if let Some(scope_index) = index_of_binding(scopes, name) {
                scopes[scope_index].remove(name);
            }
        }
        ExprKind::Store(value, ptr) => {
            fold_expr(value, scopes);
            fold_expr(ptr, scopes);
        }
        ExprKind::Free(ptr) => fold_expr(ptr, scopes),
        ExprKind::For {
            init,
            condition,
            post,
            body,
        } => {
            if let Some(init) = init.as_mut() {
                fold_expr(init, scopes);
            }
            if let Some(condition) = condition.as_mut() {
                fold_expr(condition, scopes);
            }
            if let Some(condition) = condition.as_ref() {
                if let ExprKind::BoolLiteral(false) = condition.kind {
                    expr.kind = ExprKind::Block(Block {
                        expressions: init.take().map(|expression| vec![*expression]).unwrap_or_default(),
                    });
                    return;
                }
            }

            if let Some(post) = post.as_mut() {
                fold_expr(post, scopes);
            }
            fold_block(body, scopes);
        }
        ExprKind::Return(value) => {
            if let Some(value) = value.as_mut() {
                fold_expr(value, scopes);
            }
        }
        ExprKind::While { condition, body } => {
            fold_expr(condition, scopes);
            if let ExprKind::BoolLiteral(false) = condition.kind {
                expr.kind = ExprKind::Block(Block {
                    expressions: Vec::new(),
                });
            } else {
                fold_block(body, scopes);
            }
        }
        ExprKind::IfElse {
            condition,
            then_block,
            else_block,
        } => {
            fold_expr(condition, scopes);
            if let ExprKind::BoolLiteral(true) = condition.kind {
                fold_block(then_block, scopes);
                let then_block = mem::replace(then_block, Block {
                    expressions: Vec::new(),
                });
                expr.kind = ExprKind::Block(then_block);
                return;
            }

            if let ExprKind::BoolLiteral(false) = condition.kind {
                fold_block(else_block, scopes);
                let else_block = mem::replace(else_block, Block {
                    expressions: Vec::new(),
                });
                expr.kind = ExprKind::Block(else_block);
                return;
            }

            fold_block(then_block, scopes);
            fold_block(else_block, scopes);
        }
        ExprKind::Not(expr) => {
            fold_expr(expr, scopes);
            if let ExprKind::BoolLiteral(value) = expr.kind {
                expr.kind = ExprKind::BoolLiteral(!value);
            }
        }
        ExprKind::Binary(op, left, right) => {
            fold_expr(left, scopes);
            fold_expr(right, scopes);

            if let Some(folded) = fold_binary(*op, &left.kind, &right.kind) {
                expr.kind = folded;
            }
        }
        ExprKind::Load(ptr) => fold_expr(ptr, scopes),
        ExprKind::Index { array, index } => {
            fold_expr(array, scopes);
            fold_expr(index, scopes);
        }
        ExprKind::Alloc(size) => fold_expr(size, scopes),
        ExprKind::Call { args, .. } => {
            for arg in args.iter_mut() {
                fold_expr(arg, scopes);
            }
        }
        ExprKind::Block(block) => fold_block(block, scopes),
        ExprKind::Identifier(name) => {
            if let Some(constant) = lookup_constant(scopes, name) {
                expr.kind = constant;
            }
        }
        ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::SizeOf(_) => {}
    }
}

fn index_of_binding(scopes: &[HashMap<String, ExprKind>], name: &str) -> Option<usize> {
    scopes.iter().rposition(|scope| scope.contains_key(name))
}

fn lookup_constant(scopes: &[HashMap<String, ExprKind>], name: &str) -> Option<ExprKind> {
    scopes
        .iter()
        .rev()
        .find_map(|scope| scope.get(name))
        .cloned()
}

fn infer_constant(kind: &ExprKind) -> Option<ExprKind> {
    match kind {
        ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_) => Some(kind.clone()),
        _ => None,
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
        (BinaryOp::Div, ExprKind::IntLiteral(lhs), ExprKind::IntLiteral(rhs))
            if *lhs == i64::MIN && *rhs == -1 =>
        {
            None
        }
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

    #[test]
    fn fold_identifier_const_from_let() {
        let mut program = parse_program("fn main() -> i64 { let x = 2 + 3; return x + 1; }");
        fold_program(&mut program);

        let return_expr = match &program.functions[0].body.expressions[1].kind {
            crate::ast::ExprKind::Return(Some(expr)) => &expr.kind,
            _ => panic!("expression de retour inattendue"),
        };
        assert_eq!(return_expr, &crate::ast::ExprKind::IntLiteral(6));
    }

    #[test]
    fn fold_boolean_comparison_simple() {
        let mut program = parse_program("fn main() -> bool { let valid = true; return valid == true; }");
        fold_program(&mut program);

        let return_expr = match &program.functions[0].body.expressions[1].kind {
            crate::ast::ExprKind::Return(Some(expr)) => &expr.kind,
            _ => panic!("expression de retour inattendue"),
        };
        assert_eq!(return_expr, &crate::ast::ExprKind::BoolLiteral(true));
    }

    #[test]
    fn fold_unreachable_if_true_branch() {
        let mut program = parse_program(
            "fn main() -> i64 { if true { return 1; } else { return 2; } return 3; }",
        );
        fold_program(&mut program);

        let expr = &program.functions[0].body.expressions[0].kind;
        let then_block = match expr {
            crate::ast::ExprKind::Block(block) => block,
            _ => panic!("expression de branchement attendue"),
        };
        assert_eq!(then_block.expressions.len(), 1);
        assert!(matches!(
            then_block.expressions[0].kind,
            crate::ast::ExprKind::Return(Some(_))
        ));
    }

    #[test]
    fn fold_while_false_is_removed() {
        let mut program = parse_program("fn main() -> i64 { while false { return 1; } return 2; }");
        fold_program(&mut program);

        let expr = &program.functions[0].body.expressions[0].kind;
        let block = match expr {
            crate::ast::ExprKind::Block(block) => block,
            _ => panic!("expression de bloc attendue"),
        };
        assert_eq!(block.expressions.len(), 0);
    }

    #[test]
    fn fold_for_false_is_dead() {
        let mut program =
            parse_program("fn main() -> i64 { for (1 + 2; false; 3 + 4) { return 1; } return 2; }");
        fold_program(&mut program);

        let expr = &program.functions[0].body.expressions[0].kind;
        let block = match expr {
            crate::ast::ExprKind::Block(block) => block,
            _ => panic!("expression de bloc attendue"),
        };
        assert_eq!(block.expressions.len(), 1);
        assert_eq!(block.expressions[0], crate::ast::ExprKind::IntLiteral(3));
    }
}
