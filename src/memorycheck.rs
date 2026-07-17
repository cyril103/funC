use std::collections::{HashMap, HashSet};

use crate::ast::{Expr, ExprKind, Program};

#[derive(Debug, Clone)]
pub struct MemoryWarning {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

pub fn analyze(program: &Program) -> Vec<MemoryWarning> {
    let mut warnings = Vec::new();
    for function in &program.functions {
        let mut state = FunctionMemoryState::new();
        state.analyze_block(&function.body);
        warnings.extend(state.finalize());
    }
    warnings
}

#[derive(Default)]
struct FunctionMemoryState {
    scopes: Vec<HashMap<String, Option<usize>>>,
    alloc_sites: HashMap<usize, (usize, usize)>,
    alloc_freed: HashMap<usize, bool>,
    warned_allocs: HashSet<usize>,
    next_alloc_id: usize,
    local_allocs: Vec<usize>,
    warnings: Vec<MemoryWarning>,
}

impl FunctionMemoryState {
    fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            ..Default::default()
        }
    }

    fn analyze_block(&mut self, block: &crate::ast::Block) {
        self.scopes.push(HashMap::new());
        for expr in &block.expressions {
            self.analyze_expr(expr);
        }
        let _ = self.scopes.pop();
    }

    fn analyze_expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Let { name, value, .. } => {
                self.analyze_expr(value);
                let allocation = self.extract_allocation(value);
                self.scopes
                    .last_mut()
                    .expect("analysis scope missing")
                    .insert(name.clone(), allocation);
            }
            ExprKind::Assign { name, value } => {
                self.analyze_expr(value);
                let allocation = self.extract_allocation(value);
                self.assign_symbol(name, allocation, expr.line, expr.column);
            }
            ExprKind::Store(value, ptr) => {
                self.analyze_expr(value);
                self.analyze_expr(ptr);
            }
            ExprKind::IfElse {
                condition,
                then_block,
                else_block,
            } => {
                self.analyze_expr(condition);
                self.analyze_block(then_block);
                self.analyze_block(else_block);
            }
            ExprKind::While { condition, body } => {
                self.analyze_expr(condition);
                self.analyze_block(body);
            }
            ExprKind::For {
                init,
                condition,
                post,
                body,
            } => {
                if let Some(init) = init {
                    self.analyze_expr(init);
                }
                if let Some(condition) = condition {
                    self.analyze_expr(condition);
                }
                self.analyze_block(body);
                if let Some(post) = post {
                    self.analyze_expr(post);
                }
            }
            ExprKind::Return(value) => {
                if let Some(value) = value {
                    self.analyze_expr(value);
                }
            }
            ExprKind::Not(expr) => self.analyze_expr(expr),
            ExprKind::Binary(_, left, right) => {
                self.analyze_expr(left);
                self.analyze_expr(right);
            }
            ExprKind::Call { args, .. } => {
                for arg in args {
                    self.analyze_expr(arg);
                }
            }
            ExprKind::Alloc(size) => {
                self.analyze_expr(size);
            }
            ExprKind::Free(ptr) => {
                self.analyze_expr(ptr);
                self.process_free(ptr);
            }
            ExprKind::Load(ptr) => self.analyze_expr(ptr),
            ExprKind::SizeOf(_) => {}
            ExprKind::Index { array, index } => {
                self.analyze_expr(array);
                self.analyze_expr(index);
            }
            ExprKind::Block(block) => self.analyze_block(block),
            ExprKind::Identifier(_) | ExprKind::IntLiteral(_) | ExprKind::FloatLiteral(_) | ExprKind::BoolLiteral(_) => {}
        }
    }

    fn extract_allocation(&mut self, expr: &Expr) -> Option<usize> {
        match &expr.kind {
            ExprKind::Alloc(_) => Some(self.record_alloc(expr.line, expr.column)),
            ExprKind::Identifier(name) => self.resolve_symbol(name),
            _ => None,
        }
    }

    fn resolve_symbol(&self, name: &str) -> Option<usize> {
        for scope in self.scopes.iter().rev() {
            if let Some(allocation) = scope.get(name) {
                return *allocation;
            }
        }
        None
    }

    fn assign_symbol(&mut self, name: &str, new_allocation: Option<usize>, line: usize, column: usize) {
        let mut scope_index = None;
        let mut current_allocation = None;
        for (idx, scope) in self.scopes.iter().enumerate() {
            if scope.contains_key(name) {
                scope_index = Some(idx);
                current_allocation = scope.get(name).copied().flatten();
                break;
            }
        }

        if let Some(current_allocation) = current_allocation {
            let should_warn = if let Some(next_allocation) = new_allocation {
                current_allocation != next_allocation
            } else {
                true
            };
            if should_warn && !self.alloc_freed.get(&current_allocation).copied().unwrap_or(false) {
                let message = format!(
                    "la variable '{}' est réaffectée sans libérer l'allocation précédente",
                    name
                );
                self.warn(line, column, message);
                self.warned_allocs.insert(current_allocation);
            }
        }

        if let Some(idx) = scope_index {
            self.scopes[idx].insert(name.to_string(), new_allocation);
        }
    }

    fn process_free(&mut self, ptr: &Expr) {
        if let ExprKind::Identifier(name) = &ptr.kind {
            if let Some(allocation_id) = self.resolve_symbol(name) {
                if self.alloc_freed.get(&allocation_id).copied().unwrap_or(false) {
                    self.warn(ptr.line, ptr.column, format!("double free détecté sur '{}', risque d'UB", name));
                    return;
                }
                self.alloc_freed.insert(allocation_id, true);
            }
        }
    }

    fn record_alloc(&mut self, line: usize, column: usize) -> usize {
        let allocation_id = self.next_alloc_id;
        self.next_alloc_id += 1;
        self.local_allocs.push(allocation_id);
        self.alloc_sites.insert(allocation_id, (line, column));
        self.alloc_freed.insert(allocation_id, false);
        allocation_id
    }

    fn warn(&mut self, line: usize, column: usize, message: String) {
        self.warnings.push(MemoryWarning {
            line,
            column,
            message,
        });
    }

    fn finalize(&mut self) -> Vec<MemoryWarning> {
        let local_allocs: Vec<usize> = self.local_allocs.iter().copied().collect();
        for alloc_id in local_allocs {
            if self.warned_allocs.contains(&alloc_id) {
                continue;
            }
            if self.alloc_freed.get(&alloc_id).copied().unwrap_or(false) {
                continue;
            }
            let (line, column) = self.alloc_sites.get(&alloc_id).copied().unwrap_or((0, 0));
            self.warn(
                line,
                column,
                "allocation heap potentiellement non libérée (heuristique)".to_string(),
            );
        }

        self.warnings
            .drain(..)
            .collect::<Vec<_>>()
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::Program;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    use super::analyze;

    fn parse_program(source: &str) -> Program {
        let tokens = Lexer::new(source).tokenize().unwrap();
        Parser::new(tokens).parse_program().unwrap()
    }

    #[test]
    fn warn_when_alloc_not_freed() {
        let program = parse_program("fn main() -> void { let ptr = alloc(16); }");
        let warnings = analyze(&program);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("non libérée"));
    }

    #[test]
    fn no_warning_when_freed_by_alias() {
        let program = parse_program("fn main() -> void { let ptr = alloc(16); let alias = ptr; free(alias); }");
        let warnings = analyze(&program);
        assert!(warnings.is_empty());
    }

    #[test]
    fn warn_when_mutable_reassigned_without_free() {
        let program = parse_program("fn main() -> void { let mut ptr = alloc(8); ptr = alloc(8); }");
        let warnings = analyze(&program);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("réaffectée"));
    }
}
