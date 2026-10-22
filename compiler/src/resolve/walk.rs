use super::{BindingId, GlobalRead, ModuleCall, Resolver, ScopeKind, SymbolKind};
use crate::ast::{AssignTarget, Block, Expr, ExprKind, Stmt, StmtKind};
use crate::error::ResolveErrorKind;

impl Resolver {
    pub(crate) fn resolve_stmt(&mut self, statement: &Stmt) {
        match &statement.kind {
            StmtKind::Assign { target, value, .. } => {
                self.resolve_expr(value);
                self.resolve_assign(target);
            }
            StmtKind::Expr(expr) => self.resolve_expr(expr),
            StmtKind::Function(decl) => {
                self.resolve_function_body(decl);
            }
            StmtKind::If { arms, else_block } => {
                for (condition, block) in arms {
                    self.resolve_expr(condition);
                    self.resolve_block(block);
                }
                if let Some(block) = else_block {
                    self.resolve_block(block);
                }
            }
            StmtKind::While { condition, body } => {
                self.resolve_expr(condition);
                self.loop_depth += 1;
                self.resolve_block(body);
                self.loop_depth -= 1;
            }
            StmtKind::For {
                variable,
                iterable,
                body,
            } => {
                self.resolve_expr(iterable);
                self.loop_depth += 1;
                self.push_scope(ScopeKind::Block);
                self.declare(variable, SymbolKind::LoopVar);
                for inner in &body.statements {
                    self.resolve_stmt(inner);
                }
                self.pop_scope();
                self.loop_depth -= 1;
            }
            StmtKind::Return(value) => {
                if let Some(expr) = value {
                    self.resolve_expr(expr);
                }
            }
            StmtKind::Break => {
                if self.loop_depth == 0 {
                    self.error(ResolveErrorKind::BreakOutsideLoop, statement.span);
                }
            }
            StmtKind::Continue => {
                if self.loop_depth == 0 {
                    self.error(ResolveErrorKind::ContinueOutsideLoop, statement.span);
                }
            }
        }
    }

    pub(crate) fn resolve_block(&mut self, block: &Block) {
        self.push_scope(ScopeKind::Block);
        for statement in &block.statements {
            self.resolve_stmt(statement);
        }
        self.pop_scope();
    }

    fn resolve_assign(&mut self, target: &AssignTarget) {
        match target {
            AssignTarget::Name(ident) => match self.lookup(&ident.name) {
                Some(id) => self.reassign(ident, id),
                None => {
                    let kind = self.variable_kind();
                    if self.declare(ident, kind).is_some() {
                        self.declares.insert(ident.span);
                    }
                }
            },
            AssignTarget::Index { object, index, .. } => {
                self.resolve_expr(object);
                self.resolve_expr(index);
                self.check_mutation_target(object);
            }
        }
    }

    fn reassign(&mut self, ident: &crate::ast::Ident, id: BindingId) {
        self.bindings.insert(ident.span, id);
        let symbol = self.symbols[id].kind;

        if self.imported_here.contains(&id) {
            self.error(
                ResolveErrorKind::NotAssignable {
                    name: ident.name.clone(),
                    what: "an imported name",
                },
                ident.span,
            );
            return;
        }

        let error = match symbol {
            SymbolKind::Global if self.in_function() => {
                Some(ResolveErrorKind::AssignToGlobal(ident.name.clone()))
            }
            SymbolKind::LoopVar => Some(ResolveErrorKind::AssignToLoopVariable(
                ident.name.clone(),
            )),
            SymbolKind::Builtin => Some(ResolveErrorKind::RedeclaresBuiltin(ident.name.clone())),
            SymbolKind::Function if !self.in_function() => {
                Some(ResolveErrorKind::DuplicateDeclaration {
                    name: ident.name.clone(),
                    previous: self.symbols[id].span,
                })
            }
            SymbolKind::Function => Some(ResolveErrorKind::NotAssignable {
                name: ident.name.clone(),
                what: symbol.describe(),
            }),
            _ => None,
        };

        if let Some(error) = error {
            self.error(error, ident.span);
        }
    }

    fn check_mutation_target(&mut self, object: &Expr) {
        if !self.in_function() {
            return;
        }

        let Some(id) = self.root_binding(object) else {
            return;
        };

        if self.symbols[id].kind != SymbolKind::Global {
            return;
        }

        let name = self.symbols[id].name.clone();
        self.error(ResolveErrorKind::MutateGlobalInFunction(name), object.span);
    }

    fn root_binding(&self, expr: &Expr) -> Option<BindingId> {
        match &expr.kind {
            ExprKind::Name(_) => self.bindings.get(&expr.span).copied(),
            ExprKind::Index { object, .. } => self.root_binding(object),
            _ => None,
        }
    }

    pub(crate) fn resolve_expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::Str(_) | ExprKind::Bool(_) => {}
            ExprKind::Name(name) => self.resolve_name(name, expr.span),
            ExprKind::Unary { operand, .. } => self.resolve_expr(operand),
            ExprKind::Binary { left, right, .. } => {
                self.resolve_expr(left);
                self.resolve_expr(right);
            }
            ExprKind::Call { callee, args } => {
                self.resolve_expr(callee);
                for arg in args {
                    self.resolve_expr(arg);
                }
                self.record_call(callee);
            }
            ExprKind::Index { object, index } => {
                self.resolve_expr(object);
                self.resolve_expr(index);
            }
            ExprKind::List(items) => {
                for item in items {
                    self.resolve_expr(item);
                }
            }
            ExprKind::Dict(entries) => {
                for (key, value) in entries {
                    self.resolve_expr(key);
                    self.resolve_expr(value);
                }
            }
        }
    }

    fn resolve_name(&mut self, name: &str, span: crate::token::Span) {
        let Some(id) = self.lookup(name) else {
            self.error(ResolveErrorKind::UndefinedName(name.to_string()), span);
            return;
        };

        self.bindings.insert(span, id);
        self.record_global_read(id);
    }

    fn record_global_read(&mut self, id: BindingId) {
        let Some(function) = self.current_function else {
            return;
        };

        if self.symbols[id].kind != SymbolKind::Global {
            return;
        }

        let Some(&(unit, declared_at)) = self.global_positions.get(&id) else {
            return;
        };

        self.graph.nodes[function].global_reads.push(GlobalRead {
            name: self.symbols[id].name.clone(),
            unit,
            declared_at,
        });
    }

    fn record_call(&mut self, callee: &Expr) {
        let ExprKind::Name(name) = &callee.kind else {
            return;
        };

        let target = self
            .lookup(name)
            .and_then(|binding| self.graph.by_binding.get(&binding).copied());

        let Some(target) = target else {
            return;
        };

        match self.current_function {
            Some(caller) => self.graph.nodes[caller].calls.push(target),
            None => self.graph.module_calls.push(ModuleCall {
                function: target,
                unit: self.unit,
                statement_index: self.statement_index,
                span: callee.span,
            }),
        }
    }
}
