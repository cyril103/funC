use std::collections::HashMap;

use crate::ast::{BinaryOp, Block, Expr, ExprKind, Program, Type};

#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub params: Vec<Type>,
    pub return_type: Type,
}

#[derive(Debug, Clone)]
pub struct TypeError {
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub suggestion: Option<String>,
}

pub fn check(program: &Program, _source: &str) -> Result<HashMap<usize, Type>, TypeError> {
    let mut env = TypeEnvironment::new();

    for function in &program.functions {
        if env.functions.contains_key(&function.name) {
            return Err(TypeError {
                message: format!("fonction '{}' déjà déclarée", function.name),
                line: 0,
                column: 0,
                suggestion: Some("Renommez une fonction en doublon ou supprimez la redéfinition.".to_string()),
            });
        }
        env.functions.insert(
            function.name.clone(),
            FunctionSignature {
                params: function.params.iter().map(|p| p.ty.clone()).collect(),
                return_type: function.return_type.clone(),
            },
        );
    }

    let mut inferred = HashMap::new();
    for function in &program.functions {
        let mut fn_env = env.clone_empty_scope();
        for param in &function.params {
            fn_env.locals.push(HashMap::new());
            fn_env.locals.last_mut().unwrap().insert(
                param.name.clone(),
                (param.ty.clone(), false),
            );
        }
        fn_env.return_type = Some(function.return_type.clone());
        let body_ty = infer_block(&function.body, &mut fn_env, &env.functions, &mut inferred)?;
        if !fn_env.saw_return && body_ty != function.return_type {
            return Err(TypeError {
                message: format!(
                    "la fonction '{}' attend un retour '{}', mais le bloc retourne '{}'",
                    function.name, function.return_type, body_ty
                ),
                line: 0,
                column: 0,
                suggestion: Some(
                    "Assurez-vous que toutes les branches retournent le type attendu par la fonction.".to_string(),
                ),
            });
        }
    }

    Ok(inferred)
}

#[derive(Clone)]
struct TypeEnvironment {
    locals: Vec<HashMap<String, (Type, bool)>>,
    functions: HashMap<String, FunctionSignature>,
    return_type: Option<Type>,
    saw_return: bool,
}

impl TypeEnvironment {
    fn new() -> Self {
        Self {
            locals: vec![HashMap::new()],
            functions: HashMap::new(),
            return_type: None,
            saw_return: false,
        }
    }

    fn clone_empty_scope(&self) -> Self {
        let functions = self.functions.clone();
        let return_type = self.return_type.clone();
        let saw_return = self.saw_return;
        Self {
            locals: vec![HashMap::new()],
            functions,
            return_type,
            saw_return,
        }
    }

    fn push_scope(&mut self) {
        self.locals.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.locals.pop();
    }

    fn define(&mut self, name: String, ty: Type, mutable: bool) {
        if let Some(scope) = self.locals.last_mut() {
            scope.insert(name, (ty, mutable));
        }
    }

    fn resolve(&self, name: &str) -> Option<Type> {
        for scope in self.locals.iter().rev() {
            if let Some((ty, _)) = scope.get(name) {
                return Some(ty.clone());
            }
        }
        None
    }

    fn resolve_symbol(&self, name: &str) -> Option<(Type, bool)> {
        for scope in self.locals.iter().rev() {
            if let Some(info) = scope.get(name) {
                return Some(info.clone());
            }
        }
        None
    }
}

fn infer_block(
    block: &Block,
    env: &mut TypeEnvironment,
    functions: &HashMap<String, FunctionSignature>,
    inferred: &mut HashMap<usize, Type>,
) -> Result<Type, TypeError> {
    let mut last = Type::Void;
    env.push_scope();
    for expr in &block.expressions {
        last = infer_expr(expr, env, functions, inferred)?;
    }
    env.pop_scope();
    Ok(last)
}

fn infer_expr(
    expr: &Expr,
    env: &mut TypeEnvironment,
    functions: &HashMap<String, FunctionSignature>,
    inferred: &mut HashMap<usize, Type>,
) -> Result<Type, TypeError> {
    let ty = match &expr.kind {
        ExprKind::Let {
            name,
            ty,
            value,
            mutable,
        } => {
            let init_ty = infer_expr(value, env, functions, inferred)?;
            if let Some(ann) = ty {
                if &init_ty != ann {
                    return Err(TypeError {
                        message: format!(
                            "mauvaise annotation: la variable '{}' attend {}, trouvé {}",
                            name, ann, init_ty
                        ),
                        line: expr.line,
                        column: expr.column,
                        suggestion: Some(format!("Déclarez '{}' avec le type '{}' ou ajustez son initialisation.", name, init_ty).to_string()),
                    });
                }
            }
            let final_ty = ty.clone().unwrap_or(init_ty);
            env.define(name.clone(), final_ty.clone(), *mutable);
            final_ty
        }
        ExprKind::Assign { name, value } => {
            let rhs_ty = infer_expr(value, env, functions, inferred)?;
            let (decl_ty, mutable) = env.resolve_symbol(name).ok_or_else(|| TypeError {
                message: format!("identifiant '{}' inconnu", name),
                line: expr.line,
                column: expr.column,
                suggestion: Some("Déclarez d'abord la variable avec let avant de l'assigner.".to_string()),
            })?;
            if !mutable {
                return Err(TypeError {
                    message: format!("la variable '{}' n'est pas mutable", name),
                    line: expr.line,
                    column: expr.column,
                    suggestion: Some(format!(
                        "déclarez '{}' avec 'let mut {}' pour autoriser une réassignation.",
                        name, name
                    )),
                });
            }
            if rhs_ty != decl_ty {
                return Err(TypeError {
                    message: format!(
                        "affectation incompatible: '{}' attendu '{}', trouvé '{}'",
                        name, decl_ty, rhs_ty
                    ),
                    line: expr.line,
                    column: expr.column,
                    suggestion: Some(format!(
                        "Retournez/affectez une valeur de type '{}' pour '{}'.",
                        decl_ty, name
                    )),
                });
            }
            decl_ty
        }
        ExprKind::Store(value, ptr) => {
            let value_ty = infer_expr(value, env, functions, inferred)?;
            let ptr_ty = infer_expr(ptr, env, functions, inferred)?;
            match ptr_ty {
                Type::Pointer(inner) => {
                    if *inner != value_ty {
                        return Err(TypeError {
                            message: format!(
                                "type incompatible in store: pointeur vers {}, valeur {}",
                                inner, value_ty
                            ),
                            line: expr.line,
                            column: expr.column,
                            suggestion: Some("Assurez-vous que la valeur et le pointeur ont le même type élémentaire.".to_string()),
                        });
                    }
                }
                _ => {
                    return Err(TypeError {
                        message: "store attend un pointeur en second argument".to_string(),
                        line: expr.line,
                        column: expr.column,
                        suggestion: Some(
                            "Passez un pointeur créé par alloc/load en second argument de store.".to_string(),
                        ),
                    });
                }
            }
            Type::Void
        }
        ExprKind::IfElse {
            condition,
            then_block,
            else_block,
        } => {
            let cond_ty = infer_expr(condition, env, functions, inferred)?;
            if cond_ty != Type::Bool {
                return Err(TypeError {
                    message: format!(
                        "condition if doit être bool, trouvé {}",
                        cond_ty
                    ),
                    line: expr.line,
                    column: expr.column,
                    suggestion: Some("Utilisez une expression booléenne dans la condition de if.".to_string()),
                });
            }

            let then_ty = infer_block(then_block, &mut env.clone_empty_scope(), functions, inferred)?;
            let else_ty = infer_block(else_block, &mut env.clone_empty_scope(), functions, inferred)?;

            if then_ty != else_ty {
                return Err(TypeError {
                    message: format!(
                        "branches if/else de types différents: {} vs {}",
                        then_ty, else_ty
                    ),
                    line: expr.line,
                    column: expr.column,
                    suggestion: Some(
                        "Faites retourner le même type dans les branches then/else.".to_string(),
                    ),
                });
            }
            then_ty
        }
        ExprKind::While { condition, body } => {
            let cond_ty = infer_expr(condition, env, functions, inferred)?;
            if cond_ty != Type::Bool {
                return Err(TypeError {
                    message: format!("condition while doit être bool, trouvé {}", cond_ty),
                    line: expr.line,
                    column: expr.column,
                    suggestion: Some(
                        "Utilisez une expression booléenne pour la condition du while.".to_string(),
                    ),
                });
            }
            let _body_ty = infer_block(body, &mut env.clone_empty_scope(), functions, inferred)?;
            Type::Void
        }
        ExprKind::For {
            init,
            condition,
            post,
            body,
        } => {
            if let Some(init) = init {
                infer_expr(init, env, functions, inferred)?;
            }
            if let Some(condition) = condition {
                let cond_ty = infer_expr(condition, env, functions, inferred)?;
                if cond_ty != Type::Bool {
                    return Err(TypeError {
                        message: format!("condition for doit être bool, trouvé {}", cond_ty),
                        line: expr.line,
                        column: expr.column,
                        suggestion: Some(
                            "Utilisez une expression booléenne pour la condition du for.".to_string(),
                        ),
                    });
                }
            }
            let _body_ty = infer_block(body, &mut env.clone_empty_scope(), functions, inferred)?;
            if let Some(post) = post {
                infer_expr(post, env, functions, inferred)?;
            }
            Type::Void
        }
        ExprKind::Return(value) => {
            let expected = env
                .return_type
                .clone()
                .unwrap_or_else(|| Type::Void);
            env.saw_return = true;

            match (value, expected) {
                (None, Type::Void) => Type::Void,
                (None, expected) => {
                    return Err(TypeError {
                        message: "return sans valeur dans une fonction non-void".to_string(),
                        line: expr.line,
                        column: expr.column,
                        suggestion: Some(
                            "Ajoutez une expression de retour correspondant au type de la fonction."
                                .to_string(),
                        ),
                    });
                }
                (Some(_), Type::Void) => {
                    return Err(TypeError {
                        message: "return avec une valeur dans une fonction void".to_string(),
                        line: expr.line,
                        column: expr.column,
                        suggestion: Some(
                            "Retirez la valeur du return, ou changez le type de la fonction."
                                .to_string(),
                        ),
                    });
                }
                (Some(value), expected) => {
                    let actual = infer_expr(value, env, functions, inferred)?;
                    if actual != expected {
                        return Err(TypeError {
                            message: format!(
                                "return de type '{}' attendu '{}'",
                                actual, expected
                            ),
                            line: expr.line,
                            column: expr.column,
                            suggestion: Some(
                                "Retournez une expression du type déclaré par la fonction."
                                    .to_string(),
                            ),
                        });
                    }
                    expected
                }
            }
        }
        ExprKind::Not(expr_arg) => {
            let operand_ty = infer_expr(expr_arg, env, functions, inferred)?;
            if operand_ty != Type::Bool {
                return Err(TypeError {
                    message: format!("! attend bool, trouvé {}", operand_ty),
                    line: expr.line,
                    column: expr.column,
                    suggestion: Some("L'opérateur ! s'applique uniquement aux booléens.".to_string()),
                });
            }
            Type::Bool
        }
        ExprKind::Binary(op, left, right) => infer_binary(*op, left, right, env, functions, inferred)?,
        ExprKind::Identifier(name) => env
            .resolve(name)
            .ok_or_else(|| TypeError {
                message: format!("identifiant '{}' inconnu", name),
                line: expr.line,
                column: expr.column,
                suggestion: Some("Déclarez d'abord la variable avec let avant de l'utiliser.".to_string()),
            })?,
        ExprKind::IntLiteral(_) => Type::I64,
        ExprKind::FloatLiteral(_) => Type::F64,
        ExprKind::BoolLiteral(_) => Type::Bool,
        ExprKind::Call { name, args } => {
            let sig = functions.get(name).ok_or_else(|| TypeError {
                message: format!("fonction '{}' inconnue", name),
                line: expr.line,
                column: expr.column,
                suggestion: Some("Vérifiez le nom de la fonction ou définissez-la avant l'appel.".to_string()),
            })?;
            if sig.params.len() != args.len() {
                return Err(TypeError {
                    message: format!(
                        "appel à '{}' attend {} arguments, reçu {}",
                        name,
                        sig.params.len(),
                        args.len()
                    ),
                    line: expr.line,
                    column: expr.column,
                    suggestion: Some("Ajustez le nombre d'arguments pour appeler la fonction.".to_string()),
                });
            }
            for (idx, arg) in args.iter().enumerate() {
                let arg_ty = infer_expr(arg, env, functions, inferred)?;
                if arg_ty != sig.params[idx] {
                    return Err(TypeError {
                        message: format!(
                            "arg {} de '{}' attend {}, trouvé {}",
                            idx, name, sig.params[idx], arg_ty
                        ),
                        line: expr.line,
                        column: expr.column,
                        suggestion: Some(format!("Corrigez le type de l'argument {} attendu par {}.", idx, name)),
                    });
                }
            }
            sig.return_type.clone()
        }
        ExprKind::Alloc(size) => {
            let size_ty = infer_expr(size, env, functions, inferred)?;
            if size_ty != Type::I64 && size_ty != Type::I32 {
                return Err(TypeError {
                    message: "alloc attend un entier de taille".to_string(),
                    line: expr.line,
                    column: expr.column,
                    suggestion: Some(
                        "Passez un entier (i32 ou i64) représentant une taille en octets.".to_string(),
                    ),
                });
            }
            Type::Pointer(Box::new(Type::I8))
        }
        ExprKind::Free(ptr) => {
            let ptr_ty = infer_expr(ptr, env, functions, inferred)?;
            match ptr_ty {
                Type::Pointer(_) => Type::Void,
                _ => {
                    return Err(TypeError {
                        message: "free attend un pointeur".to_string(),
                        line: expr.line,
                        column: expr.column,
                        suggestion: Some("Passez un pointeur alloué par alloc à free.".to_string()),
                    });
                }
            }
        }
        ExprKind::Load(ptr) => {
            match infer_expr(ptr, env, functions, inferred)? {
                Type::Pointer(inner) => *inner,
                _ => {
                    return Err(TypeError {
                        message: "load attend un pointeur".to_string(),
                        line: expr.line,
                        column: expr.column,
                        suggestion: Some("Chargez une valeur depuis un pointeur existant.".to_string()),
                    });
                }
            }
        }
        ExprKind::SizeOf(ty) => {
            let _bytes = match ty {
                Type::Void => 0,
                Type::Bool => 1,
                Type::I8 | Type::U8 => 1,
                Type::I16 | Type::U16 => 2,
                Type::I32 | Type::U32 | Type::F32 => 4,
                Type::I64 | Type::U64 | Type::F64 => 8,
                Type::Pointer(inner) => 8 + size_of_type(inner.as_ref()).unwrap_or(8),
            };
            inferred.insert(expr.id, Type::I64);
            Type::I64
        }
        ExprKind::Block(block) => infer_block(block, env, functions, inferred)?,
    };

    inferred.insert(expr.id, ty.clone());
    Ok(ty)
}

fn infer_binary(
    op: BinaryOp,
    left: &Expr,
    right: &Expr,
    env: &mut TypeEnvironment,
    functions: &HashMap<String, FunctionSignature>,
    inferred: &mut HashMap<usize, Type>,
) -> Result<Type, TypeError> {
    let left_ty = infer_expr(left, env, functions, inferred)?;
    let right_ty = infer_expr(right, env, functions, inferred)?;

    if left_ty != right_ty {
        return Err(TypeError {
            message: format!(
                "opération '{}' incompatible entre {} et {}",
                op, left_ty, right_ty
            ),
            line: right.line,
            column: right.column,
            suggestion: Some("Assurez-vous que les deux opérandes ont le même type.".to_string()),
        });
    }

    let ty = match op {
        BinaryOp::Or | BinaryOp::And => {
            if left_ty != Type::Bool {
                return Err(TypeError {
                    message: "&& et || attendent des booléens".to_string(),
                    line: right.line,
                    column: right.column,
                    suggestion: Some("Utilisez || et && seulement avec des booléens.".to_string()),
                });
            }
            Type::Bool
        }
        BinaryOp::Eq | BinaryOp::NotEq => {
            if !left_ty.is_numeric() && left_ty != Type::Bool && !matches!(left_ty, Type::Pointer(_)) {
                return Err(TypeError {
                    message: "comparaison indisponible sur ce type".to_string(),
                    line: right.line,
                    column: right.column,
                    suggestion: Some("Comparez seulement des types numériques, booléens ou pointeurs.".to_string()),
                });
            }
            Type::Bool
        }
        BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq => {
            if !(left_ty.is_numeric() || matches!(left_ty, Type::Pointer(_))) {
                return Err(TypeError {
                    message: "comparaison relationnelle attend un type numérique ou un pointeur".to_string(),
                    line: right.line,
                    column: right.column,
                    suggestion: Some("Utilisez < > <= >= avec nombres entiers/flottants ou pointeurs.".to_string()),
                });
            }
            Type::Bool
        }
        BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
            if !left_ty.is_numeric() {
                return Err(TypeError {
                    message: format!("opération arithmétique non supportée sur {}", left_ty),
                    line: right.line,
                    column: right.column,
                    suggestion: Some("Utilisez des types numériques pour les opérations arithmétiques.".to_string()),
                });
            }
            left_ty
        }
    };

    Ok(ty)
}

fn size_of_type(ty: &Type) -> Option<i64> {
    match ty {
        Type::Void => Some(0),
        Type::Bool => Some(1),
        Type::I8 | Type::U8 => Some(1),
        Type::I16 | Type::U16 => Some(2),
        Type::I32 | Type::U32 | Type::F32 => Some(4),
        Type::I64 | Type::U64 | Type::F64 => Some(8),
        Type::Pointer(_) => Some(8),
    }
}

impl std::fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.column, self.message)?;
        if let Some(suggestion) = &self.suggestion {
            write!(f, "\n  suggestion: {suggestion}")?;
        }
        Ok(())
    }
}

impl std::error::Error for TypeError {}
