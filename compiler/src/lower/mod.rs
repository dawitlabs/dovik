mod expr;
mod stmt;

use std::collections::HashMap;

use crate::ast::{FunctionDecl, Program as Ast, StmtKind};
use crate::check::Typed;
use crate::ir;
use crate::resolve::{BindingId, Resolution, SymbolKind};
use crate::token::Span;
use crate::types::Type;

const ENTRY_NAME: &str = "main";

pub fn lower(ast: &Ast, resolution: &Resolution, typed: &Typed) -> ir::Program {
    let mut lowerer = Lowerer {
        ast,
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
    pub(crate) ast: &'a Ast,
    pub(crate) resolution: &'a Resolution,
    pub(crate) typed: &'a Typed,
    pub(crate) program: ir::Program,
    strings: HashMap<String, ir::StrId>,
    globals: HashMap<BindingId, ir::GlobalId>,
    functions: HashMap<String, ir::FuncId>,
    /// Local slot for each binding in the function being lowered.
    slots: HashMap<BindingId, ir::LocalId>,
    pub(crate) blocks: Vec<ir::Block>,
    pub(crate) registers: Vec<Type>,
    pub(crate) locals: Vec<ir::Local>,
    pub(crate) current: ir::BlockId,
    /// Header and exit of each enclosing loop, for break and continue.
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

    /// Reserve an id for every function before lowering any body, so a call to a function
    /// declared further down the file has somewhere to point (spec 5.5).
    fn declare_functions(&mut self) {
        for statement in &self.ast.statements {
            let StmtKind::Function(decl) = &statement.kind else {
                continue;
            };

            if self.functions.contains_key(&decl.name.name) {
                continue;
            }

            self.functions
                .insert(decl.name.name.clone(), self.program.functions.len());
            self.program.functions.push(placeholder(&decl.name.name));
        }
    }

    fn lower_functions(&mut self) {
        let ast = self.ast;

        for statement in &ast.statements {
            let StmtKind::Function(decl) = &statement.kind else {
                continue;
            };

            let Some(&id) = self.functions.get(&decl.name.name) else {
                continue;
            };

            if self.program.functions[id].blocks.is_empty() {
                let function = self.lower_function(decl);
                self.program.functions[id] = function;
            }
        }
    }

    fn lower_function(&mut self, decl: &FunctionDecl) -> ir::Function {
        let signature = self.typed.functions.get(&decl.name.name).cloned();
        let return_type = signature.map(|s| s.result).unwrap_or(Type::Unknown);

        self.begin_body();

        for param in &decl.params {
            let ty = self.type_of_binding(param.name.span);
            self.declare_slot(param.name.span, &param.name.name, ty);
        }
        let param_count = self.locals.len();

        self.lower_statements(&decl.body.statements);

        // Spec 5.3: a `Unit` function may simply run off the end.
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

    /// Spec 8.5: top-level statements run in source order, so they become one function.
    fn lower_entry(&mut self) {
        let ast = self.ast;
        self.begin_body();

        for statement in &ast.statements {
            if matches!(statement.kind, StmtKind::Function(_)) {
                continue;
            }

            self.lower_stmt(statement);
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

    /// Finishes the current block if nothing has terminated it yet. A block whose
    /// terminator is already set has returned or jumped, and must not be overwritten.
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

    /// A slot the lowerer needs for itself — a loop cursor, a short-circuit result. It
    /// deliberately claims no binding, so it can never be found by a name lookup.
    pub(crate) fn temp_slot(&mut self, name: &str, ty: Type) -> ir::LocalId {
        self.locals.push(ir::Local {
            name: name.to_string(),
            ty,
        });
        self.locals.len() - 1
    }

    pub(crate) fn declare_slot(&mut self, span: Span, name: &str, ty: Type) -> ir::LocalId {
        let slot = self.locals.len();
        self.locals.push(ir::Local {
            name: name.to_string(),
            ty,
        });

        if let Some(id) = self.resolution.binding_id(span) {
            self.slots.insert(id, slot);
        }

        slot
    }

    /// Where a name lives. A binding is either a module-scope global or a slot in the
    /// function being lowered; functions never write globals (spec 9.4).
    pub(crate) fn place_of(&mut self, span: Span, name: &str) -> Place {
        let Some(id) = self.resolution.binding_id(span) else {
            return Place::Missing;
        };

        if let Some(&slot) = self.slots.get(&id) {
            return Place::Local(slot);
        }

        if let Some(&global) = self.globals.get(&id) {
            return Place::Global(global);
        }

        let ty = self.type_of_binding(span);
        Place::Local(self.declare_slot(span, name, ty))
    }

    pub(crate) fn type_of_binding(&self, span: Span) -> Type {
        self.resolution
            .binding_id(span)
            .and_then(|id| self.typed.var_types.get(&id))
            .cloned()
            .unwrap_or(Type::Unknown)
    }

    pub(crate) fn function_id(&self, name: &str) -> Option<ir::FuncId> {
        self.functions.get(name).copied()
    }
}

pub(crate) enum Place {
    Local(ir::LocalId),
    Global(ir::GlobalId),
    Missing,
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
