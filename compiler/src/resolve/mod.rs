mod order;
mod scope;
mod walk;

use std::collections::{HashMap, HashSet};

use crate::ast::{FunctionDecl, Program, StmtKind};
use crate::modules::Graph;
use crate::error::{ResolveError, ResolveErrorKind};
use crate::token::Span;

pub type BindingId = usize;

pub fn resolve(graph: &Graph) -> (Resolution, Vec<ResolveError>) {
    let mut resolver = Resolver::new();
    resolver.declare_builtins();

    for index in 0..graph.units.len() {
        resolver.resolve_unit(graph, index);
    }

    resolver.check_module_call_order();

    let resolution = Resolution {
        symbols: resolver.symbols,
        bindings: resolver.bindings,
        declares: resolver.declares,
    };

    (resolution, resolver.errors)
}

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

    pub fn binding_id(&self, span: Span) -> Option<BindingId> {
        self.bindings.get(&span).copied()
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

#[derive(Debug, Default)]
pub(crate) struct CallGraph {
    pub(crate) nodes: Vec<FunctionNode>,
    pub(crate) by_binding: HashMap<BindingId, usize>,
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
    pub(crate) unit: usize,
    pub(crate) declared_at: usize,
}

#[derive(Debug)]
pub(crate) struct ModuleCall {
    pub(crate) function: usize,
    pub(crate) unit: usize,
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
    pub(crate) global_positions: HashMap<BindingId, (usize, usize)>,
    pub(crate) exports: Vec<HashMap<String, BindingId>>,
    pub(crate) imported_here: HashSet<BindingId>,
    pub(crate) unit: usize,
    pub(crate) statement_index: usize,
    pub(crate) current_function: Option<usize>,
    pub(crate) loop_depth: usize,
}

impl Resolver {
    fn new() -> Self {
        Self {
            scopes: vec![Scope {
                kind: ScopeKind::Builtin,
                names: HashMap::new(),
            }],
            symbols: Vec::new(),
            bindings: HashMap::new(),
            declares: HashSet::new(),
            errors: Vec::new(),
            graph: CallGraph::default(),
            global_positions: HashMap::new(),
            exports: Vec::new(),
            imported_here: HashSet::new(),
            unit: 0,
            statement_index: 0,
            current_function: None,
            loop_depth: 0,
        }
    }

    fn declare_builtins(&mut self) {
        for name in crate::builtins::NAMES {
            let id = self.symbols.len();
            self.symbols.push(Symbol {
                name: name.to_string(),
                kind: SymbolKind::Builtin,
                span: Span::new(0, 0),
            });
            self.scopes[0].names.insert(name.to_string(), id);
        }
    }

    fn resolve_unit(&mut self, graph: &Graph, index: usize) {
        let unit = &graph.units[index];
        self.unit = index;
        self.imported_here.clear();
        self.push_scope(ScopeKind::Module);

        let mut imported: Vec<String> = Vec::new();

        for (import, &from) in unit.ast.imports.iter().zip(&unit.imports) {
            for name in &import.names {
                let local = name.local();

                let Some(&id) = self.exports[from].get(&name.exported.name) else {
                    self.error(
                        ResolveErrorKind::NoSuchExport {
                            name: name.exported.name.clone(),
                            module: graph.units[from]
                                .path
                                .file_name()
                                .map(|part| part.to_string_lossy().to_string())
                                .unwrap_or_default(),
                        },
                        name.exported.span,
                    );
                    continue;
                };

                if self.bind_import(local, id) {
                    imported.push(local.name.clone());
                    self.imported_here.insert(id);
                }
            }
        }

        self.collect_function_signatures(&unit.ast);
        self.resolve_module(&unit.ast);

        let mut exports = self.current_scope().names.clone();
        for name in imported {
            exports.remove(&name);
        }

        self.exports.push(exports);
        self.pop_scope();
    }

    fn collect_function_signatures(&mut self, program: &Program) {
        for statement in &program.statements {
            let StmtKind::Function(decl) = &statement.kind else {
                continue;
            };

            if self.declare(&decl.name, SymbolKind::Function).is_none() {
                continue;
            }

            let Some(binding) = self.lookup(&decl.name.name) else {
                continue;
            };

            let index = self.graph.nodes.len();
            self.graph.nodes.push(FunctionNode {
                name: decl.name.name.clone(),
                ..FunctionNode::default()
            });
            self.graph.by_binding.insert(binding, index);
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
        let index = self
            .lookup(&decl.name.name)
            .and_then(|binding| self.graph.by_binding.get(&binding).copied());

        let Some(index) = index else {
            return self.resolve_function_body(decl);
        };

        let previous = self.current_function.replace(index);
        self.resolve_function_body(decl);
        self.current_function = previous;
    }

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
