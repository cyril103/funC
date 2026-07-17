use std::collections::{HashMap, HashSet};

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
    env.functions.insert(
        "realloc".to_string(),
        FunctionSignature {
            params: vec![Type::Pointer(Box::new(Type::I8)), Type::I64],
            return_type: Type::Pointer(Box::new(Type::I8)),
        },
    );
    env.functions.insert(
        "func::realloc".to_string(),
        FunctionSignature {
            params: vec![Type::Pointer(Box::new(Type::I8)), Type::I64],
            return_type: Type::Pointer(Box::new(Type::I8)),
        },
    );
    env.functions.insert(
        "assert".to_string(),
        FunctionSignature {
            params: vec![Type::Bool],
            return_type: Type::Void,
        },
    );
    env.functions.insert(
        "func::assert".to_string(),
        FunctionSignature {
            params: vec![Type::Bool],
            return_type: Type::Void,
        },
    );
    env.functions.insert(
        "panic".to_string(),
        FunctionSignature {
            params: vec![],
            return_type: Type::Void,
        },
    );
    env.functions.insert(
        "func::panic".to_string(),
        FunctionSignature {
            params: vec![],
            return_type: Type::Void,
        },
    );
    env.functions.insert(
        "func::free".to_string(),
        FunctionSignature {
            params: vec![Type::Pointer(Box::new(Type::I8))],
            return_type: Type::Void,
        },
    );
    env.functions.insert(
        "memcpy".to_string(),
        FunctionSignature {
            params: vec![
                Type::Pointer(Box::new(Type::I8)),
                Type::Pointer(Box::new(Type::I8)),
                Type::I64,
            ],
            return_type: Type::Pointer(Box::new(Type::I8)),
        },
    );
    env.functions.insert(
        "func::memcpy".to_string(),
        FunctionSignature {
            params: vec![
                Type::Pointer(Box::new(Type::I8)),
                Type::Pointer(Box::new(Type::I8)),
                Type::I64,
            ],
            return_type: Type::Pointer(Box::new(Type::I8)),
        },
    );
    env.functions.insert(
        "memset".to_string(),
        FunctionSignature {
            params: vec![
                Type::Pointer(Box::new(Type::I8)),
                Type::I8,
                Type::I64,
            ],
            return_type: Type::Pointer(Box::new(Type::I8)),
        },
    );
    env.functions.insert(
        "func::memset".to_string(),
        FunctionSignature {
            params: vec![
                Type::Pointer(Box::new(Type::I8)),
                Type::I8,
                Type::I64,
            ],
            return_type: Type::Pointer(Box::new(Type::I8)),
        },
    );
    env.functions.insert(
        "func::alloc".to_string(),
        FunctionSignature {
            params: vec![Type::I64],
            return_type: Type::Pointer(Box::new(Type::I8)),
        },
    );

    for struct_decl in &program.structs {
        if env.functions.contains_key(&struct_decl.name)
            || env.struct_types.contains(&struct_decl.name)
            || env.enum_types.contains(&struct_decl.name)
        {
            return Err(TypeError {
                message: format!("déclaration '{}' en conflit avec un symbole existant", struct_decl.name),
                line: 0,
                column: 0,
                suggestion: Some(
                    "Renommez la structure pour éviter un conflit avec une fonction existante."
                        .to_string(),
                ),
            });
        }
        env.struct_types.insert(struct_decl.name.clone());
        env.struct_layouts.insert(
            struct_decl.name.clone(),
            struct_decl.fields.iter().map(|field| field.ty.clone()).collect(),
        );
    }

    for enum_decl in &program.enums {
        if env.functions.contains_key(&enum_decl.name)
            || env.enum_types.contains(&enum_decl.name)
            || env.struct_types.contains(&enum_decl.name)
        {
            return Err(TypeError {
                message: format!(
                    "déclaration '{}' en conflit avec un symbole existant",
                    enum_decl.name
                ),
                line: 0,
                column: 0,
                suggestion: Some(
                    "Renommez l'énumération pour éviter un conflit avec une fonction existante."
                        .to_string(),
                ),
            });
        }
        env.enum_types.insert(enum_decl.name.clone());
        env.enum_variants.insert(enum_decl.name.clone(), enum_decl.variants.len());
    }

    for struct_decl in &program.structs {
        for field in &struct_decl.fields {
            ensure_type_known(&field.ty, &env.struct_types, &env.enum_types)?;
        }
    }

    for function in &program.functions {
        let normalized_return = normalize_user_type(
            &function.return_type,
            &env.struct_types,
            &env.enum_types,
        )?;
        let mut normalized_params = Vec::new();
        for param in &function.params {
            normalized_params.push(normalize_user_type(
                &param.ty,
                &env.struct_types,
                &env.enum_types,
            )?);
        }
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
                params: normalized_params,
                return_type: normalized_return.clone(),
            },
        );
    }

    let mut inferred = HashMap::new();
    for function in &program.functions {
        let normalized_return = normalize_user_type(
            &function.return_type,
            &env.struct_types,
            &env.enum_types,
        )?;
        let mut fn_env = env.clone_empty_scope();
        fn_env.locals.push(HashMap::new());
        for param in &function.params {
            let param_type = normalize_user_type(&param.ty, &env.struct_types, &env.enum_types)?;
            if fn_env.locals.last().unwrap().contains_key(&param.name) {
                return Err(TypeError {
                    message: format!(
                        "shadowing involontaire: le paramètre '{}' masque un symbole existant",
                        param.name
                    ),
                    line: 0,
                    column: 0,
                    suggestion: Some(format!(
                        "Choisissez un autre nom que '{}' pour ce paramètre.",
                        param.name
                    )),
                });
            }
            fn_env.locals.last_mut().unwrap().insert(
                param.name.clone(),
                Symbol {
                    ty: param_type,
                    mutable: false,
                    used: false,
                    line: 0,
                    column: 0,
                },
            );
        }
        fn_env.return_type = Some(normalized_return.clone());
        let body_completion = infer_block(&function.body, &mut fn_env, &env.functions, &mut inferred)?;
        let body_ty = body_completion.ty();
        if body_ty != normalized_return {
            let detail = type_mismatch_detail(&normalized_return, &body_ty)
                .map(|extra| format!(" ({extra})"))
                .unwrap_or_default();
            return Err(TypeError {
                message: format!(
                    "la fonction '{}' attend un retour '{}', mais le bloc retourne '{}'{}",
                    function.name, normalized_return, body_ty, detail
                ),
                line: 0,
                column: 0,
                suggestion: Some(
                    "Assurez-vous que toutes les branches retournent le type attendu par la fonction.".to_string(),
                ),
            });
        }
        if let Some((name, line, column)) = fn_env.first_unused_symbol() {
            return Err(TypeError {
                message: format!("la variable '{}' est déclarée mais jamais utilisée", name),
                line,
                column,
                suggestion: Some(
                    "Supprimez-la ou utilisez-la avant la fin de la fonction pour éviter un avertissement."
                        .to_string(),
                ),
            });
        }
    }

    Ok(inferred)
}

fn ensure_type_known(
    ty: &Type,
    struct_types: &HashSet<String>,
    enum_types: &HashSet<String>,
) -> Result<(), TypeError> {
    match ty {
        Type::Array(inner, _) => ensure_type_known(inner, struct_types, enum_types),
        Type::Pointer(inner) => ensure_type_known(inner, struct_types, enum_types),
        Type::Struct(name) => {
            if struct_types.contains(name) || enum_types.contains(name) {
                Ok(())
            } else {
                Err(TypeError {
                    message: format!("type inconnu: '{}'", name),
                    line: 0,
                    column: 0,
                    suggestion: Some(format!(
                        "Déclarez un type '{}' avec `struct` ou `enum` avant de l'utiliser.",
                        name
                    )),
                })
            }
        }
        Type::Enum(name) => {
            if enum_types.contains(name) || struct_types.contains(name) {
                Ok(())
            } else {
                Err(TypeError {
                    message: format!("type inconnu: '{}'", name),
                    line: 0,
                    column: 0,
                    suggestion: Some(format!(
                        "Déclarez un type '{}' avec `enum` avant de l'utiliser.",
                        name
                    )),
                })
            }
        }
        _ => Ok(()),
    }
}

fn normalize_user_type(
    ty: &Type,
    struct_types: &HashSet<String>,
    enum_types: &HashSet<String>,
) -> Result<Type, TypeError> {
    let normalized = match ty {
        Type::Array(inner, len) => Type::Array(
            Box::new(normalize_user_type(inner, struct_types, enum_types)?),
            *len,
        ),
        Type::Pointer(inner) => Type::Pointer(Box::new(normalize_user_type(
            inner,
            struct_types,
            enum_types,
        )?)),
        Type::Struct(name) => {
            if enum_types.contains(name) {
                Type::Enum(name.clone())
            } else if struct_types.contains(name) {
                Type::Struct(name.clone())
            } else {
                return Err(TypeError {
                    message: format!("type inconnu: '{}'", name),
                    line: 0,
                    column: 0,
                    suggestion: Some(format!(
                        "Déclarez un type '{}' avant de l'utiliser.",
                        name
                    )),
                });
            }
        }
        other => other.clone(),
    };
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn parse_program(source: &str) -> Program {
        let tokens = Lexer::new(source).tokenize().unwrap();
        Parser::new(tokens).parse_program().unwrap()
    }

    #[test]
    fn function_call_arity_mismatch() {
        let program = parse_program("fn id(x: i64) -> i64 { x } fn main() -> i64 { id(1, 2); }");
        let err = check(&program, "").unwrap_err();
        assert!(err.message.contains("attend 1 arguments"));
    }

    #[test]
    fn function_call_type_mismatch() {
        let program = parse_program("fn id(x: i64) -> i64 { x } fn main() -> i64 { id(true); }");
        let err = check(&program, "").unwrap_err();
        assert!(err.message.contains("arg 0"));
    }

    #[test]
    fn function_call_unknown_function() {
        let program = parse_program("fn main() -> i64 { foo(1); }");
        let err = check(&program, "").unwrap_err();
        assert!(err.message.contains("fonction 'foo' inconnue"));
    }

    #[test]
    fn function_return_type_mismatch() {
        let program = parse_program("fn main() -> i64 { return true; }");
        let err = check(&program, "").unwrap_err();
        assert!(err.message.contains("return de type"));
    }

    #[test]
    fn numeric_width_mismatch_is_reported() {
        let program = parse_program("fn main() -> i64 { let x: i32 = 1; return x; }");
        let err = check(&program, "").unwrap_err();
        assert!(err.message.contains("largeurs d'entiers diffèrent"));
    }

    #[test]
    fn signedness_mismatch_is_reported() {
        let program = parse_program("fn copy(x: i32) -> void { let y: u32 = x; }");
        let err = check(&program, "").unwrap_err();
        assert!(err.message.contains("mélange signé / non signé détecté"));
    }

    #[test]
    fn pointer_mismatch_is_reported() {
        let program = parse_program("fn main() -> void { let p = alloc(1); let x: *i32 = p; }");
        let err = check(&program, "").unwrap_err();
        assert!(err.message.contains("pointeurs ciblent des types différents"));
    }

    #[test]
    fn function_missing_non_void_return() {
        let program = parse_program("fn main() -> i64 { let x = 1; }");
        let err = check(&program, "").unwrap_err();
        assert!(err.message.contains("attend un retour"));
    }

    #[test]
    fn function_void_return_with_value() {
        let program = parse_program("fn main() -> void { return 1; }");
        let err = check(&program, "").unwrap_err();
        assert!(err.message.contains("return avec une valeur"));
    }

    #[test]
    fn sizeof_array_type_is_valid() {
        let program = parse_program("fn main() -> i64 { sizeof([i64; 4]) }");
        assert!(check(&program, "").is_ok());
    }

    #[test]
    fn array_index_is_valid_with_integer_literal() {
        let program = parse_program("fn main(values: [i64; 4]) -> i64 { return values[2]; }");
        assert!(check(&program, "").is_ok());
    }

    #[test]
    fn array_index_rejects_non_integer_index() {
        let program = parse_program("fn main(values: [i64; 4]) -> i64 { return values[true]; }");
        let err = check(&program, "").unwrap_err();
        assert!(err.message.contains("indexation attend un type entier"));
    }

    #[test]
    fn array_index_out_of_bounds_rejects() {
        let program = parse_program("fn main(values: [i64; 2]) -> i64 { return values[3]; }");
        let err = check(&program, "").unwrap_err();
        assert!(err.message.contains("hors limites"));
    }

    #[test]
    fn sizeof_struct_type_is_valid() {
        let program = parse_program(
            "struct Point { x: i64; y: i64; } fn main() -> i64 { sizeof(Point) }",
        );
        assert!(check(&program, "").is_ok());
    }

    #[test]
    fn sizeof_enum_type_is_valid() {
        let program = parse_program(
            "enum Color { Red, Green, Blue } fn main() -> i64 { sizeof(Color) }",
        );
        assert!(check(&program, "").is_ok());
    }

    #[test]
    fn sizeof_array_of_struct_is_valid() {
        let program = parse_program(
            "struct Pair { x: i32; y: i32; } fn main() -> i64 { sizeof([Pair; 3]) }",
        );
        assert!(check(&program, "").is_ok());
    }

    #[test]
    fn detect_shadowing_in_same_scope() {
        let program = parse_program("fn main() -> i64 { let x = 1; let x = 2; return x; }");
        let err = check(&program, "").unwrap_err();
        assert!(err.message.contains("shadowing"));
    }

    #[test]
    fn detect_shadowing_with_parameter() {
        let program = parse_program("fn main(x: i64) -> i64 { let x = 2; return x; }");
        let err = check(&program, "").unwrap_err();
        assert!(err.message.contains("shadowing"));
    }

    #[test]
    fn detect_unused_local_variable() {
        let program = parse_program("fn main() -> i64 { let x = 1; return 0; }");
        let err = check(&program, "").unwrap_err();
        assert!(err.message.contains("jamais utilisée"));
    }

    #[test]
    fn detect_unused_parameter() {
        let program = parse_program("fn main(x: i64) -> i64 { return 0; }");
        let err = check(&program, "").unwrap_err();
        assert!(err.message.contains("jamais utilisée"));
    }

    #[test]
    fn if_else_all_branches_return_same_type() {
        let program = parse_program("fn main() -> i64 { if true { 10 } else { 20 } }");
        assert!(check(&program, "").is_ok());
    }

    #[test]
    fn if_else_mismatched_completion_is_rejected() {
        let program = parse_program("fn main() -> i64 { if true { return 1; } else { 2; } }");
        let err = check(&program, "").unwrap_err();
        assert!(err.message.contains("incohérentes"));
    }

    #[test]
    fn struct_type_can_be_used_in_function_signature() {
        let program = parse_program(
            "struct Point { x: i32; y: i32; } fn main(p: Point) -> Point { return p; }",
        );
        assert!(check(&program, "").is_ok());
    }

    #[test]
    fn enum_type_can_be_used_in_function_signature() {
        let program = parse_program(
            "enum Color { Red, Green, Blue } fn main(c: Color) -> Color { return c; }",
        );
        assert!(check(&program, "").is_ok());
    }

    #[test]
    fn unknown_user_type_is_rejected() {
        let program = parse_program("fn main(p: Unknown) -> Unknown { return p; }");
        let err = check(&program, "").unwrap_err();
        assert!(err.message.contains("type inconnu"));
    }

    #[test]
    fn memory_builtin_calls_are_available() {
        let program = parse_program(
            "fn main() -> void { let p = alloc(16); let q = realloc(p, 32); memcpy(q, p, 16); memset(q, 0, 8); free(q); }",
        );
        assert!(check(&program, "").is_ok());
    }

    #[test]
    fn style_helpers_assert_and_panic_are_available() {
        let program = parse_program("fn main() -> void { assert(true); panic(); }");
        assert!(check(&program, "").is_ok());
    }

    #[test]
    fn assert_requires_bool_argument() {
        let program = parse_program("fn main() -> void { assert(42); }");
        let err = check(&program, "").unwrap_err();
        assert!(err.message.contains("arg 0 de 'assert' attend bool"));
    }

    #[test]
    fn std_namespace_memory_and_runtime_helpers_are_available() {
        let program = parse_program(
            "fn main() -> void { let p = func::alloc(16); let q = func::realloc(p, 32); func::memcpy(q, p, 16); func::memset(q, 0, 8); func::free(q); func::assert(true); func::panic(); }",
        );
        assert!(check(&program, "").is_ok());
    }
}

#[derive(Clone)]
struct TypeEnvironment {
    locals: Vec<HashMap<String, Symbol>>,
    functions: HashMap<String, FunctionSignature>,
    struct_types: HashSet<String>,
    enum_types: HashSet<String>,
    struct_layouts: HashMap<String, Vec<Type>>,
    enum_variants: HashMap<String, usize>,
    return_type: Option<Type>,
    saw_return: bool,
}

#[derive(Clone)]
struct Symbol {
    ty: Type,
    mutable: bool,
    used: bool,
    line: usize,
    column: usize,
}

#[derive(Clone)]
enum ExprCompletion {
    Value(Type),
    Returns(Type),
}

impl ExprCompletion {
    fn ty(&self) -> Type {
        match self {
            ExprCompletion::Value(ty) => ty.clone(),
            ExprCompletion::Returns(ty) => ty.clone(),
        }
    }
}

impl TypeEnvironment {
    fn new() -> Self {
        Self {
            locals: vec![HashMap::new()],
            functions: HashMap::new(),
            struct_types: HashSet::new(),
            enum_types: HashSet::new(),
            struct_layouts: HashMap::new(),
            enum_variants: HashMap::new(),
            return_type: None,
            saw_return: false,
        }
    }

    fn clone_empty_scope(&self) -> Self {
        let functions = self.functions.clone();
        let struct_types = self.struct_types.clone();
        let enum_types = self.enum_types.clone();
        let struct_layouts = self.struct_layouts.clone();
        let enum_variants = self.enum_variants.clone();
        let return_type = self.return_type.clone();
        let saw_return = self.saw_return;
        Self {
            locals: vec![HashMap::new()],
            functions,
            struct_types,
            enum_types,
            struct_layouts,
            enum_variants,
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

    fn is_declared(&self, name: &str) -> bool {
        self.locals.iter().any(|scope| scope.contains_key(name))
    }

    fn define(&mut self, name: String, ty: Type, mutable: bool, line: usize, column: usize) {
        if let Some(scope) = self.locals.last_mut() {
            scope.insert(
                name,
                Symbol {
                    ty,
                    mutable,
                    used: false,
                    line,
                    column,
                },
            );
        }
    }

    fn resolve_symbol_for_use(&mut self, name: &str) -> Option<(Type, bool)> {
        for scope in self.locals.iter_mut().rev() {
            if let Some(symbol) = scope.get_mut(name) {
                symbol.used = true;
                return Some((symbol.ty.clone(), symbol.mutable));
            }
        }
        None
    }

    fn first_unused_symbol_in_scope(&self) -> Option<(String, usize, usize)> {
        if let Some(scope) = self.locals.last() {
            for (name, symbol) in scope {
                if !symbol.used {
                    return Some((name.clone(), symbol.line, symbol.column));
                }
            }
        }
        None
    }

    fn first_unused_symbol(&self) -> Option<(String, usize, usize)> {
        for scope in self.locals.iter() {
            for (name, symbol) in scope {
                if !symbol.used {
                    return Some((name.clone(), symbol.line, symbol.column));
                }
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
) -> Result<ExprCompletion, TypeError> {
    let mut last = ExprCompletion::Value(Type::Void);
    let mut terminated = false;
    env.push_scope();
    for expr in &block.expressions {
        let expr_completion = infer_expr(expr, env, functions, inferred)?;
        if !terminated {
            if let ExprCompletion::Returns(_) = expr_completion {
                terminated = true;
            }
            last = expr_completion;
        }
    }
    if let Some((name, line, column)) = env.first_unused_symbol_in_scope() {
        return Err(TypeError {
            message: format!("la variable '{}' est déclarée mais jamais utilisée", name),
            line,
            column,
            suggestion: Some(
                "Supprimez-la ou utilisez-la avant la fin du bloc pour éviter un avertissement."
                    .to_string(),
            ),
        });
    }
    env.pop_scope();

    Ok(last)
}

fn infer_expr(
    expr: &Expr,
    env: &mut TypeEnvironment,
    functions: &HashMap<String, FunctionSignature>,
    inferred: &mut HashMap<usize, Type>,
) -> Result<ExprCompletion, TypeError> {
    let completion = match &expr.kind {
        ExprKind::Let {
            name,
            ty,
            value,
            mutable,
        } => {
            if env.is_declared(name) {
                return Err(TypeError {
                    message: format!(
                        "shadowing involontaire: '{}' masque une variable existante",
                        name
                    ),
                    line: expr.line,
                    column: expr.column,
                    suggestion: Some(
                        "Choisissez un nom différent pour éviter l'ombre d'une variable existante."
                            .to_string(),
                    ),
                });
            }
            let init_ty = infer_expr(value, env, functions, inferred)?.ty();
            if let Some(ann) = ty {
                let normalized_ann = normalize_user_type(ann, &env.struct_types, &env.enum_types)?;
                if init_ty != normalized_ann {
                    let detail = type_mismatch_detail(ann, &init_ty)
                        .map(|extra| format!(" ({extra})"))
                        .unwrap_or_default();
                    return Err(TypeError {
                        message: format!(
                            "mauvaise annotation: la variable '{}' attend {}, trouvé {}{}",
                            name, ann, init_ty, detail
                        ),
                        line: expr.line,
                        column: expr.column,
                        suggestion: Some(format!(
                            "Déclarez '{}' avec le type '{}' ou ajustez son initialisation.",
                            name, ann
                        )),
                    });
                }
                env.define(
                    name.clone(),
                    normalized_ann.clone(),
                    *mutable,
                    expr.line,
                    expr.column,
                );
                ExprCompletion::Value(normalized_ann)
            } else {
                env.define(
                    name.clone(),
                    init_ty.clone(),
                    *mutable,
                    expr.line,
                    expr.column,
                );
                ExprCompletion::Value(init_ty)
            }
        }
        ExprKind::Assign { name, value } => {
            let rhs_ty = infer_expr(value, env, functions, inferred)?.ty();
            let (decl_ty, mutable) = env.resolve_symbol_for_use(name).ok_or_else(|| TypeError {
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
                let detail = type_mismatch_detail(&decl_ty, &rhs_ty)
                    .map(|extra| format!(" ({extra})"))
                    .unwrap_or_default();
                return Err(TypeError {
                    message: format!(
                        "affectation incompatible: '{}' attendu '{}', trouvé '{}'{}",
                        name, decl_ty, rhs_ty, detail
                    ),
                    line: expr.line,
                    column: expr.column,
                    suggestion: Some(format!(
                        "Retournez/affectez une valeur de type '{}' pour '{}'.",
                        decl_ty, name
                    )),
                });
            }
            ExprCompletion::Value(decl_ty)
        }
        ExprKind::Store(value, ptr) => {
            let value_ty = infer_expr(value, env, functions, inferred)?.ty();
            let ptr_ty = infer_expr(ptr, env, functions, inferred)?.ty();
            match ptr_ty {
                Type::Pointer(inner) => {
                    if *inner != value_ty {
                        let detail = type_mismatch_detail(&Type::Pointer(Box::new((*inner).clone())), &Type::Pointer(Box::new(value_ty.clone())))
                            .map(|extra| format!(" ({extra})"))
                            .unwrap_or_default();
                        return Err(TypeError {
                            message: format!(
                                "store: le pointeur cible attend un élément '{}', mais reçoit '{}'{detail}",
                                inner, value_ty
                            ),
                            line: expr.line,
                            column: expr.column,
                            suggestion: Some("Assurez-vous que la valeur et le pointeur ont le même type élémentaire.".to_string()),
                        });
                    }
                }
                _ => {
                    let detail = type_mismatch_detail(
                        &Type::Pointer(Box::new(Type::Void)),
                        &ptr_ty,
                    )
                    .map(|extra| format!(" ({extra})"))
                    .unwrap_or_default();
                    return Err(TypeError {
                        message: format!(
                            "store attend un pointeur en second argument, trouvé {}{detail}",
                            ptr_ty
                        ),
                        line: expr.line,
                        column: expr.column,
                        suggestion: Some(
                            "Passez un pointeur créé par alloc/load en second argument de store.".to_string(),
                        ),
                    });
                }
            }
            ExprCompletion::Value(Type::Void)
        }
        ExprKind::IfElse {
            condition,
            then_block,
            else_block,
        } => {
            let cond_ty = infer_expr(condition, env, functions, inferred)?.ty();
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

            let then_completion =
                infer_block(then_block, &mut env.clone_empty_scope(), functions, inferred)?;
            let else_completion =
                infer_block(else_block, &mut env.clone_empty_scope(), functions, inferred)?;

            match (&then_completion, &else_completion) {
                (ExprCompletion::Value(then_ty), ExprCompletion::Value(else_ty)) => {
                    if then_ty != else_ty {
                        return Err(TypeError {
                            message: format!(
                                "branches if/else de types différents: {} vs {}",
                                then_ty, else_ty
                            ),
                            line: expr.line,
                            column: expr.column,
                            suggestion: Some(
                                "Faites retourner le même type dans les branches then/else."
                                    .to_string(),
                            ),
                        });
                    }
                }
                (ExprCompletion::Returns(then_ty), ExprCompletion::Returns(else_ty)) => {
                    if then_ty != else_ty {
                        return Err(TypeError {
                            message: format!(
                                "branches if/else incohérentes: deux retours de types différents: {} vs {}",
                                then_ty, else_ty
                            ),
                            line: expr.line,
                            column: expr.column,
                            suggestion: Some(
                                "Alignez le type de return attendu par la fonction sur les deux branches."
                                    .to_string(),
                            ),
                        });
                    }
                }
                _ => {
                    return Err(TypeError {
                        message:
                            "branches if/else incohérentes: un chemin retourne explicitement, l'autre retourne une valeur"
                                .to_string(),
                        line: expr.line,
                        column: expr.column,
                        suggestion: Some(
                            "Gardez un mode de retour homogène entre les branches (toutes expressions ou tous return)."
                                .to_string(),
                        ),
                    });
                }
            }

            match (then_completion, else_completion) {
                (ExprCompletion::Value(then_ty), ExprCompletion::Value(_)) => {
                    ExprCompletion::Value(then_ty)
                }
                (ExprCompletion::Returns(then_ty), ExprCompletion::Returns(_)) => {
                    ExprCompletion::Returns(then_ty)
                }
                _ => unreachable!(),
            }
        }
        ExprKind::While { condition, body } => {
            let cond_ty = infer_expr(condition, env, functions, inferred)?.ty();
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
            let _ = infer_block(body, &mut env.clone_empty_scope(), functions, inferred)?;
            ExprCompletion::Value(Type::Void)
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
                let cond_ty = infer_expr(condition, env, functions, inferred)?.ty();
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
            ExprCompletion::Value(Type::Void)
        }
        ExprKind::Return(value) => {
            let expected = env
                .return_type
                .clone()
                .unwrap_or_else(|| Type::Void);
            env.saw_return = true;

            match (value, expected) {
                (None, Type::Void) => ExprCompletion::Value(Type::Void),
                (None, _expected) => {
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
                    let actual = infer_expr(value, env, functions, inferred)?.ty();
                    if actual != expected {
                        let detail = type_mismatch_detail(&expected, &actual)
                            .map(|extra| format!(" ({extra})"))
                            .unwrap_or_default();
                        return Err(TypeError {
                            message: format!(
                                "return de type '{}' attendu '{}'{}",
                                actual, expected, detail
                            ),
                            line: expr.line,
                            column: expr.column,
                            suggestion: Some(
                                "Retournez une expression du type déclaré par la fonction."
                                    .to_string(),
                            ),
                        });
                    }
                    ExprCompletion::Returns(expected)
                }
            }
        }
        ExprKind::Not(expr_arg) => {
            let operand_ty = infer_expr(expr_arg, env, functions, inferred)?.ty();
            if operand_ty != Type::Bool {
                return Err(TypeError {
                    message: format!("! attend bool, trouvé {}", operand_ty),
                    line: expr.line,
                    column: expr.column,
                    suggestion: Some("L'opérateur ! s'applique uniquement aux booléens.".to_string()),
                });
            }
            ExprCompletion::Value(Type::Bool)
        }
        ExprKind::Binary(op, left, right) => {
            ExprCompletion::Value(infer_binary(*op, left, right, env, functions, inferred)?)
        }
        ExprKind::Identifier(name) => {
            let (ty, _) = env
                .resolve_symbol_for_use(name)
                .ok_or_else(|| TypeError {
                    message: format!("identifiant '{}' inconnu", name),
                    line: expr.line,
                    column: expr.column,
                    suggestion: Some(
                        "Déclarez d'abord la variable avec let avant de l'utiliser.".to_string(),
                    ),
                })?;
            ExprCompletion::Value(ty)
        }
        ExprKind::IntLiteral(_) => ExprCompletion::Value(Type::I64),
        ExprKind::FloatLiteral(_) => ExprCompletion::Value(Type::F64),
        ExprKind::BoolLiteral(_) => ExprCompletion::Value(Type::Bool),
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
                    let arg_ty = infer_expr(arg, env, functions, inferred)?.ty();
                if arg_ty != sig.params[idx] {
                    let detail = type_mismatch_detail(&sig.params[idx], &arg_ty)
                        .map(|extra| format!(" ({extra})"))
                        .unwrap_or_default();
                    return Err(TypeError {
                        message: format!(
                            "arg {} de '{}' attend {}, trouvé {}{}",
                            idx, name, sig.params[idx], arg_ty, detail
                        ),
                        line: expr.line,
                        column: expr.column,
                        suggestion: Some(format!("Corrigez le type de l'argument {} attendu par {}.", idx, name)),
                    });
                }
            }
            ExprCompletion::Value(sig.return_type.clone())
        }
        ExprKind::Alloc(size) => {
            let size_ty = infer_expr(size, env, functions, inferred)?.ty();
            if size_ty != Type::I64 && size_ty != Type::I32 {
                let detail = type_mismatch_detail(&Type::I64, &size_ty)
                    .map(|extra| format!(" ({extra})"))
                    .unwrap_or_default();
                return Err(TypeError {
                    message: format!("alloc attend un entier de taille, trouvé {}{detail}", size_ty),
                    line: expr.line,
                    column: expr.column,
                    suggestion: Some(
                        "Passez un entier (i32 ou i64) représentant une taille en octets.".to_string(),
                    ),
                });
            }
            ExprCompletion::Value(Type::Pointer(Box::new(Type::I8)))
        }
        ExprKind::Free(ptr) => {
            let ptr_ty = infer_expr(ptr, env, functions, inferred)?.ty();
            match ptr_ty {
                Type::Pointer(_) => ExprCompletion::Value(Type::Void),
                _ => {
                    let detail = type_mismatch_detail(&Type::Pointer(Box::new(Type::Void)), &ptr_ty)
                        .map(|extra| format!(" ({extra})"))
                        .unwrap_or_default();
                    return Err(TypeError {
                        message: format!("free attend un pointeur, trouvé {}{detail}", ptr_ty),
                        line: expr.line,
                        column: expr.column,
                        suggestion: Some("Passez un pointeur alloué par alloc à free.".to_string()),
                    });
                }
            }
        }
        ExprKind::Load(ptr) => {
            let ptr_completion = infer_expr(ptr, env, functions, inferred)?;
            match ptr_completion {
                ExprCompletion::Value(Type::Pointer(inner)) => ExprCompletion::Value(*inner),
                _ => {
                    let ptr_ty = ptr_completion.ty();
                    let detail = type_mismatch_detail(
                        &Type::Pointer(Box::new(Type::Void)),
                        &ptr_ty,
                    )
                    .map(|extra| format!(" ({extra})"))
                    .unwrap_or_default();
                    return Err(TypeError {
                        message: format!("load attend un pointeur, trouvé {}{detail}", ptr_ty),
                        line: expr.line,
                        column: expr.column,
                        suggestion: Some("Chargez une valeur depuis un pointeur existant.".to_string()),
                    });
                }
            }
        }
            ExprKind::Index { array, index } => {
                let array_ty = infer_expr(array, env, functions, inferred)?.ty();
                let index_ty = infer_expr(index, env, functions, inferred)?.ty();
                if !index_ty.is_integer() {
                    return Err(TypeError {
                        message: format!("indexation attend un type entier, trouvé {}", index_ty),
                        line: expr.line,
                        column: expr.column,
                        suggestion: Some(
                            "Utilisez un indice de type entier pour accéder à un tableau.".to_string(),
                        ),
                    });
                }
                match array_ty {
                    Type::Array(inner, len) => {
                        if let ExprKind::IntLiteral(index_literal) = &index.kind {
                            if *index_literal < 0 {
                                return Err(TypeError {
                                    message: "indexation négative interdite".to_string(),
                                    line: index.line,
                                    column: index.column,
                                    suggestion: Some(
                                        "Utilisez un indice non négatif.".to_string(),
                                    ),
                                });
                            }
                            if let Ok(index_value) = usize::try_from(*index_literal) {
                                if index_value >= len {
                                    return Err(TypeError {
                                        message: "index hors limites du tableau".to_string(),
                                        line: index.line,
                                        column: index.column,
                                        suggestion: Some(format!(
                                            "Utilisez un indice entre 0 et {}.",
                                            len.saturating_sub(1)
                                        )),
                                    });
                                }
                            }
                        }
                        ExprCompletion::Value(*inner)
                    }
                    _ => {
                        let detail = type_mismatch_detail(&Type::Array(Box::new(Type::I64), 0), &array_ty)
                            .map(|extra| format!(" ({extra})"))
                            .unwrap_or_default();
                        return Err(TypeError {
                            message: format!("indexation attend un tableau, trouvé {}{detail}", array_ty),
                            line: expr.line,
                            column: expr.column,
                            suggestion: Some(
                                "Indexez un tableau de la forme `[T; N]`.".to_string(),
                            ),
                        });
                    }
                }
            }
        ExprKind::SizeOf(ty) => {
            let size = size_of_type(&ty, &env.struct_layouts, &env.enum_variants).ok_or_else(|| {
                TypeError {
                    message: format!("sizeof ne sait pas évaluer la taille de '{ty}'"),
                    line: expr.line,
                    column: expr.column,
                    suggestion: Some(
                        "Utilisez un type dont la taille est connue: bool, entiers, flottants, pointeurs, tableaux ou types déclarés."
                            .to_string(),
                    ),
                }
            })?;
            let _bytes = size;
            inferred.insert(expr.id, Type::I64);
            ExprCompletion::Value(Type::I64)
        }
        ExprKind::Block(block) => infer_block(block, env, functions, inferred)?,
    };

    inferred.insert(expr.id, completion.ty());
    Ok(completion)
}

fn infer_binary(
    op: BinaryOp,
    left: &Expr,
    right: &Expr,
    env: &mut TypeEnvironment,
    functions: &HashMap<String, FunctionSignature>,
    inferred: &mut HashMap<usize, Type>,
) -> Result<Type, TypeError> {
    let left_ty = infer_expr(left, env, functions, inferred)?.ty();
    let right_ty = infer_expr(right, env, functions, inferred)?.ty();

    if left_ty != right_ty {
        let detail = type_mismatch_detail(&left_ty, &right_ty)
            .map(|extra| format!(" ({extra})"))
            .unwrap_or_default();
        return Err(TypeError {
            message: format!(
                "opération '{}' incompatible entre {} et {}{}",
                op, left_ty, right_ty, detail
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

fn numeric_profile(ty: &Type) -> Option<(u8, bool)> {
    match ty {
        Type::I8 => Some((8, true)),
        Type::I16 => Some((16, true)),
        Type::I32 => Some((32, true)),
        Type::I64 => Some((64, true)),
        Type::U8 => Some((8, false)),
        Type::U16 => Some((16, false)),
        Type::U32 => Some((32, false)),
        Type::U64 => Some((64, false)),
        _ => None,
    }
}

fn pointer_depth(ty: &Type) -> usize {
    let mut depth = 0usize;
    let mut current = ty;
    while let Type::Pointer(inner) = current {
        depth += 1;
        current = inner.as_ref();
    }
    depth
}

fn pointee_type(ty: &Type) -> &Type {
    let mut current = ty;
    while let Type::Pointer(inner) = current {
        current = inner.as_ref();
    }
    current
}

fn type_mismatch_detail(expected: &Type, found: &Type) -> Option<String> {
    if expected == found {
        return None;
    }

    if let (Some((expected_bits, expected_signed)), Some((found_bits, found_signed))) =
        (numeric_profile(expected), numeric_profile(found))
    {
        if expected_bits != found_bits {
            return Some(format!(
                "les largeurs d'entiers diffèrent ({expected_bits} bits vs {found_bits} bits)"
            ));
        }
        if expected_signed != found_signed {
            return Some("mélange signé / non signé détecté".to_string());
        }
    }

    match (expected, found) {
        (Type::Pointer(_), Type::Pointer(_)) => {
            let expected_depth = pointer_depth(expected);
            let found_depth = pointer_depth(found);
            if expected_depth != found_depth {
                return Some(format!(
                    "les niveaux de pointeur diffèrent ({expected_depth} vs {found_depth})"
                ));
            }

            let expected_pointee = pointee_type(expected);
            let found_pointee = pointee_type(found);
            if expected_pointee != found_pointee {
                return Some(format!(
                    "les pointeurs ciblent des types différents ({} vs {})",
                    expected_pointee, found_pointee
                ));
            }
            None
        }
        (Type::Pointer(_), _) => Some("un pointeur est attendu ici".to_string()),
        (_, Type::Pointer(_)) => Some("une valeur non-pointeur est attendue ici".to_string()),
        _ => None,
    }
}

fn size_of_type(
    ty: &Type,
    struct_layouts: &HashMap<String, Vec<Type>>,
    enum_variants: &HashMap<String, usize>,
) -> Option<i64> {
    match ty {
        Type::Void => Some(0),
        Type::Bool => Some(1),
        Type::I8 | Type::U8 => Some(1),
        Type::I16 | Type::U16 => Some(2),
        Type::I32 | Type::U32 | Type::F32 => Some(4),
        Type::I64 | Type::U64 | Type::F64 => Some(8),
        Type::Pointer(_) => Some(8),
        Type::Struct(name) => struct_layouts.get(name).and_then(|fields| {
            fields.iter().try_fold(0i64, |acc, field_ty| {
                size_of_type(field_ty, struct_layouts, enum_variants).and_then(|field_size| {
                    acc.checked_add(field_size)
                })
            })
        }),
        Type::Enum(name) => enum_variants.get(name).map(|variants_count| {
            if *variants_count <= 2 {
                1
            } else if *variants_count <= 255 {
                4
            } else {
                8
            }
        }),
        Type::Array(inner, len) => {
            size_of_type(inner, struct_layouts, enum_variants).and_then(|inner_size| {
                inner_size.checked_mul(*len as i64)
            })
        }
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
