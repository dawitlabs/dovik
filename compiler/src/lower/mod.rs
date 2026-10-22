mod call;
mod expr;
mod literal;
mod loops;
mod slots;
mod stmt;

use std::collections::HashMap;

use crate::ast::{FunctionDecl, StmtKind};
use crate::modules::Graph;
use crate::check::Typed;
use crate::ir;
use crate::resolve::{BindingId, Resolution, SymbolKind};

pub(crate) use slots::Place;
use crate::token::Span;
use crate::types::Type;

const ENTRY_NAME: &str = "main";

pub fn lower(graph: &Graph, resolution: &Resolution, typed: &Typed) -> ir::Program {
    let mut lowerer = Lowerer {
        graph,
        resolution,
        typed,
        program: ir::Program::default(),
        strings: HashMap::new(),
        globals: HashMap::new(),
        functions: HashMap::new(),
        slots: HashMap::new(),
        blocks: Vec::new(),
        registers: Vec::new(),
        locals: Vec::new(),
        current: 0,
        loops: Vec::new(),
    };

    lowerer.declare_globals();
    lowerer.declare_functions();
    lowerer.lower_functions();
    lowerer.lower_entry();
    lowerer.program
}

pub(crate) struct Lowerer<'a> {
    pub(crate) graph: &'a Graph,
    pub(crate) resolution: &'a Resolution,
    pub(crate) typed: &'a Typed,
    pub(crate) program: ir::Program,
    strings: HashMap<String, ir::StrId>,
    globals: HashMap<BindingId, ir::GlobalId>,
    functions: HashMap<BindingId, ir::FuncId>,
    slots: HashMap<BindingId, ir::LocalId>,
    pub(crate) blocks: Vec<ir::Block>,
    pub(crate) registers: Vec<Type>,
    pub(crate) locals: Vec<ir::Local>,
    pub(crate) current: ir::BlockId,
    pub(crate) loops: Vec<(ir::BlockId, ir::BlockId)>,
}

impl Lowerer<'_> {
    fn declare_globals(&mut self) {
        for symbol in &self.resolution.symbols {
            if symbol.kind != SymbolKind::Global {
                continue;
            }

            let Some(id) = self.resolution.binding_id(symbol.span) else {
                continue;
            };

            let ty = self
                .typed
                .var_types
                .get(&id)
                .cloned()
                .unwrap_or(Type::Unknown);

            self.globals.insert(id, self.program.globals.len());
            self.program.globals.push(ir::Global {
                name: symbol.name.clone(),
                ty,
            });
        }
    }

    fn declare_functions(&mut self) {
        for (index, unit) in self.graph.units.iter().enumerate() {
            for statement in &unit.ast.statements {
                let StmtKind::Function(decl) = &statement.kind else {
                    continue;
                };

                let Some(binding) = self.resolution.binding_id(decl.name.span) else {
                    continue;
                };

                if self.functions.contains_key(&binding) {
                    continue;
                }

                let id = self.program.functions.len();
                self.functions.insert(binding, id);
                self.program.functions.push(placeholder(&decl.name.name));

                if index == self.graph.entry() {
                    self.program.entry_functions.push(id);
                }
            }
        }
    }

    fn lower_functions(&mut self) {
        for unit in &self.graph.units {
            for statement in &unit.ast.statements {
                let StmtKind::Function(decl) = &statement.kind else {
                    continue;
                };

                let id = self
                    .resolution
                    .binding_id(decl.name.span)
                    .and_then(|binding| self.functions.get(&binding).copied());

                let Some(id) = id else {
                    continue;
                };

                if self.program.functions[id].blocks.is_empty() {
                    let function = self.lower_function(decl);
                    self.program.functions[id] = function;
                }
            }
        }
    }

    fn lower_function(&mut self, decl: &FunctionDecl) -> ir::Function {
        let signature = self
            .resolution
            .binding_id(decl.name.span)
            .and_then(|binding| self.typed.functions.get(&binding))
            .cloned();
        let return_type = signature.map(|s| s.result).unwrap_or(Type::Unknown);

        self.begin_body();

        for param in &decl.params {
            let ty = self.type_of_binding(param.name.span);
            self.declare_slot(param.name.span, &param.name.name, ty);
        }
        let param_count = self.locals.len();

        self.lower_statements(&decl.body.statements);

        self.seal(ir::Terminator::Return(None));

        ir::Function {
            name: decl.name.name.clone(),
            return_type,
            param_count,
            locals: std::mem::take(&mut self.locals),
            registers: std::mem::take(&mut self.registers),
            blocks: std::mem::take(&mut self.blocks),
        }
    }

    fn lower_entry(&mut self) {
        self.begin_body();

        for unit in &self.graph.units {
            for statement in &unit.ast.statements {
                if matches!(statement.kind, StmtKind::Function(_)) {
                    continue;
                }

                self.lower_stmt(statement);
            }
        }

        self.seal(ir::Terminator::Return(None));

        self.program.main = self.program.functions.len();
        self.program.functions.push(ir::Function {
            name: ENTRY_NAME.to_string(),
            return_type: Type::Unit,
            param_count: 0,
            locals: std::mem::take(&mut self.locals),
            registers: std::mem::take(&mut self.registers),
            blocks: std::mem::take(&mut self.blocks),
        });
    }

    fn begin_body(&mut self) {
        self.slots.clear();
        self.locals.clear();
        self.registers.clear();
        self.blocks = vec![ir::Block::default()];
        self.current = 0;
        self.loops.clear();
    }

    pub(crate) fn emit(&mut self, kind: ir::InstKind, span: Span) {
        self.blocks[self.current]
            .instructions
            .push(ir::Inst { kind, span });
    }

    pub(crate) fn register(&mut self, ty: Type) -> ir::RegId {
        self.registers.push(ty);
        self.registers.len() - 1
    }

    pub(crate) fn new_block(&mut self) -> ir::BlockId {
        self.blocks.push(ir::Block::default());
        self.blocks.len() - 1
    }

    pub(crate) fn seal(&mut self, terminator: ir::Terminator) {
        if matches!(
            self.blocks[self.current].terminator,
            ir::Terminator::Unreachable
        ) {
            self.blocks[self.current].terminator = terminator;
        }
    }

    pub(crate) fn terminated(&self) -> bool {
        !matches!(
            self.blocks[self.current].terminator,
            ir::Terminator::Unreachable
        )
    }

    pub(crate) fn switch_to(&mut self, block: ir::BlockId) {
        self.current = block;
    }

    pub(crate) fn intern(&mut self, text: &str) -> ir::StrId {
        if let Some(&id) = self.strings.get(text) {
            return id;
        }

        let id = self.program.strings.len();
        self.program.strings.push(text.to_string());
        self.strings.insert(text.to_string(), id);
        id
    }

    pub(crate) fn function_id(&self, span: Span) -> Option<ir::FuncId> {
        self.resolution
            .binding_id(span)
            .and_then(|binding| self.functions.get(&binding).copied())
    }
}

fn placeholder(name: &str) -> ir::Function {
    ir::Function {
        name: name.to_string(),
        return_type: Type::Unknown,
        param_count: 0,
        locals: Vec::new(),
        registers: Vec::new(),
        blocks: Vec::new(),
    }
}
