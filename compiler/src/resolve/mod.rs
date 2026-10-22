mod order;
mod scope;
mod walk;

use std::collections::{HashMap, HashSet};

use crate::ast::{FunctionDecl, Program, StmtKind};
use crate::error::{ResolveError, ResolveErrorKind};
use crate::token::Span;

pub type BindingId = usize;

pub fn resolve(program: &Program) -> (Resolution, Vec<ResolveError>) {
    let mut resolver = Resolver::new();
    resolver.declare_builtins();
    resolver.collect_function_signatures(program);
    resolver.resolve_module(program);
    resolver.check_module_call_order();

    let resolution = Resolution {
        symbols: resolver.symbols,
        bindings: resolver.bindings,
        declares: resolver.declares,
    };

    (resolution, resolver.errors)
}

/// What later passes need from this one: which binding every name occurrence refers to,
/// and which assignments declare rather than reassign (spec 9.2).
#[derive(Debug, Clone)]
pub struct Resolution {
    pub symbols: Vec<Symbol>,
    pub bindings: HashMap<Span, BindingId>,
    pub declares: HashSet<Span>,
}

impl Resolution {
    pub fn binding(&self, span: Span) -> Option<&Symbol> {
        self.bindings.get(&span).map(|id| &self.symbols[*id])
    }

    pub fn declares(&self, span: Span) -> bool {
        self.declares.contains(&span)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Builtin,
    Function,
    Global,
    Local,
    Param,
    LoopVar,
}

impl SymbolKind {
    fn describe(self) -> &'static str {
        match self {
            Self::Builtin => "a built-in",
            Self::Function => "a function",
            _ => "a variable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScopeKind {
    Builtin,
    Module,
    Function,
    Block,
}

pub(crate) struct Scope {
    pub(crate) kind: ScopeKind,
    pub(crate) names: HashMap<String, BindingId>,
}

/// A module-scope call, and every function it can reach, for the ordering check in 8.5.
#[derive(Debug, Default)]
pub(crate) struct CallGraph {
    pub(crate) nodes: Vec<FunctionNode>,
    pub(crate) by_name: HashMap<String, usize>,
    pub(crate) module_calls: Vec<ModuleCall>,
}

#[derive(Debug, Default)]
pub(crate) struct FunctionNode {
    pub(crate) name: String,
    pub(crate) calls: Vec<usize>,
    pub(crate) global_reads: Vec<GlobalRead>,
}

#[derive(Debug)]
pub(crate) struct GlobalRead {
    pub(crate) name: String,
    pub(crate) declared_at: usize,
}

#[derive(Debug)]
pub(crate) struct ModuleCall {
    pub(crate) function: usize,
    pub(crate) statement_index: usize,
    pub(crate) span: Span,
}

pub(crate) struct Resolver {
    pub(crate) scopes: Vec<Scope>,
    pub(crate) symbols: Vec<Symbol>,
    pub(crate) bindings: HashMap<Span, BindingId>,
    pub(crate) declares: HashSet<Span>,
    pub(crate) errors: Vec<ResolveError>,
    pub(crate) graph: CallGraph,
    /// Module-statement index each global was declared at, for the 8.5 ordering check.
    pub(crate) global_positions: HashMap<BindingId, usize>,
    pub(crate) statement_index: usize,
    pub(crate) current_function: Option<usize>,
    pub(crate) loop_depth: usize,
}

impl Resolver {
    fn new() -> Self {
        Self {
            scopes: vec![
                Scope {
                    kind: ScopeKind::Builtin,
                    names: HashMap::new(),
                },
                Scope {
                    kind: ScopeKind::Module,
                    names: HashMap::new(),
                },
            ],
            symbols: Vec::new(),
            bindings: HashMap::new(),
            declares: HashSet::new(),
            errors: Vec::new(),
            graph: CallGraph::default(),
            global_positions: HashMap::new(),
            statement_index: 0,
            current_function: None,
            loop_depth: 0,
        }
    }

    fn declare_builtins(&mut self) {
        for name in ["print", "len", "to_int", "to_float"] {
            let id = self.symbols.len();
            self.symbols.push(Symbol {
                name: name.to_string(),
                kind: SymbolKind::Builtin,
                span: Span::new(0, 0),
            });
            self.scopes[0].names.insert(name.to_string(), id);
        }
    }

    /// Every module-level signature is collected before any body is checked, which is what
    /// lets a body call a function declared further down the file (spec 5.5).
    fn collect_function_signatures(&mut self, program: &Program) {
        for statement in &program.statements {
            let StmtKind::Function(decl) = &statement.kind else {
                continue;
            };

            if self.declare(&decl.name, SymbolKind::Function).is_none() {
                continue;
            }

            let index = self.graph.nodes.len();
            self.graph.nodes.push(FunctionNode {
                name: decl.name.name.clone(),
                ..FunctionNode::default()
            });
            self.graph.by_name.insert(decl.name.name.clone(), index);
        }
    }

    fn resolve_module(&mut self, program: &Program) {
        for (index, statement) in program.statements.iter().enumerate() {
            self.statement_index = index;

            match &statement.kind {
                StmtKind::Function(decl) => self.resolve_function(decl),
                _ => self.resolve_stmt(statement),
            }
        }
    }

    fn resolve_function(&mut self, decl: &FunctionDecl) {
        let Some(&index) = self.graph.by_name.get(&decl.name.name) else {
            return self.resolve_function_body(decl);
        };

        let previous = self.current_function.replace(index);
        self.resolve_function_body(decl);
        self.current_function = previous;
    }

    /// A function body is its own scope and its own loop nesting: `break` inside it can
    /// never see a loop outside it.
    pub(crate) fn resolve_function_body(&mut self, decl: &FunctionDecl) {
        let previous_depth = std::mem::replace(&mut self.loop_depth, 0);

        self.push_scope(ScopeKind::Function);
        for param in &decl.params {
            self.declare(&param.name, SymbolKind::Param);
        }
        for statement in &decl.body.statements {
            self.resolve_stmt(statement);
        }
        self.pop_scope();

        self.loop_depth = previous_depth;
    }

    pub(crate) fn error(&mut self, kind: ResolveErrorKind, span: Span) {
        self.errors.push(ResolveError { kind, span });
    }

    pub(crate) fn in_function(&self) -> bool {
        self.current_function.is_some()
    }
}
