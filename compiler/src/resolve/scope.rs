use std::collections::HashMap;

use super::{BindingId, Resolver, Scope, ScopeKind, Symbol, SymbolKind};
use crate::ast::Ident;
use crate::error::ResolveErrorKind;

/// Every case variant of a built-in type name is reserved (spec 3.2).
const RESERVED_TYPE_NAMES: [&str; 7] = ["int", "float", "bool", "string", "list", "dict", "unit"];

enum Shadowing {
    Allowed,
    Builtin,
    Conflict(crate::token::Span),
}

impl Resolver {
    pub(crate) fn push_scope(&mut self, kind: ScopeKind) {
        self.scopes.push(Scope {
            kind,
            names: HashMap::new(),
        });
    }

    pub(crate) fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub(crate) fn lookup(&self, name: &str) -> Option<BindingId> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.names.get(name).copied())
    }

    pub(crate) fn declare(&mut self, ident: &Ident, kind: SymbolKind) -> Option<BindingId> {
        // Reported, but the name is still bound below: bailing out here would make every
        // later use of it a second, misleading "not defined" error.
        if RESERVED_TYPE_NAMES.contains(&ident.name.to_ascii_lowercase().as_str()) {
            self.error(
                ResolveErrorKind::ReservedTypeName(ident.name.clone()),
                ident.span,
            );
        }

        if let Some(previous) = self.current_scope().names.get(&ident.name).copied() {
            self.error(
                ResolveErrorKind::DuplicateDeclaration {
                    name: ident.name.clone(),
                    previous: self.symbols[previous].span,
                },
                ident.span,
            );
            return None;
        }

        match self.shadowing(&ident.name) {
            Shadowing::Allowed => {}
            Shadowing::Builtin => {
                self.error(
                    ResolveErrorKind::RedeclaresBuiltin(ident.name.clone()),
                    ident.span,
                );
                return None;
            }
            Shadowing::Conflict(previous) => {
                self.error(
                    ResolveErrorKind::Shadows {
                        name: ident.name.clone(),
                        previous,
                    },
                    ident.span,
                );
                return None;
            }
        }

        let id = self.symbols.len();
        self.symbols.push(Symbol {
            name: ident.name.clone(),
            kind,
            span: ident.span,
        });
        self.current_scope_mut().names.insert(ident.name.clone(), id);
        self.bindings.insert(ident.span, id);

        if self.current_scope().kind == ScopeKind::Module && kind == SymbolKind::Global {
            self.global_positions.insert(id, self.statement_index);
        }

        Some(id)
    }

    /// Spec 9.3: shadowing is banned, except that a parameter or local may reuse the name
    /// of a module-scope *variable*. A module-scope function name is not covered by that
    /// exception, so shadowing one stays an error.
    fn shadowing(&self, name: &str) -> Shadowing {
        let inside_function = self
            .scopes
            .iter()
            .any(|scope| scope.kind == ScopeKind::Function);

        for scope in self.scopes.iter().rev().skip(1) {
            let Some(&id) = scope.names.get(name) else {
                continue;
            };

            if scope.kind == ScopeKind::Builtin {
                return Shadowing::Builtin;
            }

            let is_global_variable =
                scope.kind == ScopeKind::Module && self.symbols[id].kind == SymbolKind::Global;

            if inside_function && is_global_variable {
                return Shadowing::Allowed;
            }

            return Shadowing::Conflict(self.symbols[id].span);
        }

        Shadowing::Allowed
    }

    pub(crate) fn current_scope(&self) -> &Scope {
        self.scopes.last().expect("the builtin scope is never popped")
    }

    fn current_scope_mut(&mut self) -> &mut Scope {
        self.scopes
            .last_mut()
            .expect("the builtin scope is never popped")
    }

    /// Locals declared at module scope are globals; everywhere else they are locals.
    pub(crate) fn variable_kind(&self) -> SymbolKind {
        match self.current_scope().kind {
            ScopeKind::Module => SymbolKind::Global,
            _ => SymbolKind::Local,
        }
    }
}
