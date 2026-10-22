mod call;
mod expr;
mod stmt;

use std::collections::HashMap;

use crate::ast::{Block, FunctionDecl, Program, Stmt, StmtKind};
use crate::error::{TypeError, TypeErrorKind};
use crate::resolve::{BindingId, Resolution};
use crate::token::Span;
use crate::types::Type;

/// What lowering needs from this pass: the type of every expression, of every binding,
/// and of every function signature once inference has settled.
#[derive(Debug, Default)]
pub struct Typed {
    pub expr_types: HashMap<Span, Type>,
    pub var_types: HashMap<BindingId, Type>,
    pub functions: HashMap<String, FunctionType>,
}

#[derive(Debug, Clone)]
pub struct FunctionType {
    pub params: Vec<Type>,
    pub result: Type,
}

impl Typed {
    pub fn type_of(&self, span: Span) -> Type {
        self.expr_types.get(&span).cloned().unwrap_or(Type::Unknown)
    }
}

pub fn check(program: &Program, resolution: &Resolution) -> (Typed, Vec<TypeError>) {
    let mut checker = Checker {
        program,
        resolution,
        signatures: Vec::new(),
        by_name: HashMap::new(),
        var_types: HashMap::new(),
        errors: Vec::new(),
        silent: true,
        expected_return: None,
        returns: Vec::new(),
        expr_types: HashMap::new(),
        progress: false,
    };

    checker.collect_signatures();
    checker.infer_return_types();
    checker.check_everything();

    // Return-type errors are found after the walk, so sort to keep the report in source
    // order.
    checker.errors.sort_by_key(|error| error.span.start);

    let functions = checker
        .signatures
        .iter()
        .map(|signature| {
            (
                signature.name.clone(),
                FunctionType {
                    params: signature.params.clone(),
                    result: signature.return_type.clone().unwrap_or(Type::Unknown),
                },
            )
        })
        .collect();

    let typed = Typed {
        expr_types: checker.expr_types,
        var_types: checker.var_types,
        functions,
    };

    (typed, checker.errors)
}

pub(crate) struct Signature {
    pub(crate) name: String,
    pub(crate) span: Span,
    pub(crate) params: Vec<Type>,
    /// `None` until inference determines it (spec 5.2).
    pub(crate) return_type: Option<Type>,
}

pub(crate) struct Checker<'a> {
    pub(crate) program: &'a Program,
    pub(crate) resolution: &'a Resolution,
    pub(crate) signatures: Vec<Signature>,
    pub(crate) by_name: HashMap<String, usize>,
    pub(crate) var_types: HashMap<BindingId, Type>,
    pub(crate) errors: Vec<TypeError>,
    /// Inference runs the same walk repeatedly; only the final pass reports.
    pub(crate) silent: bool,
    pub(crate) expected_return: Option<Type>,
    pub(crate) returns: Vec<(Type, Span)>,
    pub(crate) expr_types: HashMap<Span, Type>,
    pub(crate) progress: bool,
}

impl<'a> Checker<'a> {
    /// Spec 5.5: signatures first, so a body may call a function declared further down.
    fn collect_signatures(&mut self) {
        for statement in &self.program.statements {
            let StmtKind::Function(decl) = &statement.kind else {
                continue;
            };

            if self.by_name.contains_key(&decl.name.name) {
                continue;
            }

            // A `Unit` parameter is rejected in `check_function`; the signature holds
            // `Unknown` so calls to it do not each report a second error.
            let params = decl
                .params
                .iter()
                .map(|p| match Type::from_node(&p.ty) {
                    ty if ty.mentions_unit() => Type::Unknown,
                    ty => ty,
                })
                .collect();

            // Spec 5.3: no `return` with a value anywhere means the return type is `Unit`,
            // which also gives inference somewhere to start.
            let return_type = match &decl.return_type {
                Some(node) => Some(Type::from_node(node)),
                None if !has_valued_return(&decl.body) => Some(Type::Unit),
                None => None,
            };

            self.by_name
                .insert(decl.name.name.clone(), self.signatures.len());
            self.signatures.push(Signature {
                name: decl.name.name.clone(),
                span: decl.name.span,
                params,
                return_type,
            });
        }
    }

    /// Spec 5.5: infer what the current knowledge allows, then repeat. A return whose type
    /// depends on a function still being inferred comes back `Unknown` and teaches nothing,
    /// so each round either learns a return type or ends the loop.
    fn infer_return_types(&mut self) {
        for _ in 0..=self.signatures.len() {
            self.progress = false;
            self.walk_module();

            if !self.progress {
                return;
            }
        }
    }

    fn check_everything(&mut self) {
        self.silent = false;
        self.walk_module();

        for index in 0..self.signatures.len() {
            if self.signatures[index].return_type.is_some() {
                continue;
            }

            let signature = &self.signatures[index];
            let kind = TypeErrorKind::CannotInferReturnType(signature.name.clone());
            let span = signature.span;
            self.report(kind, span);
        }
    }

    fn walk_module(&mut self) {
        let program = self.program;
        self.var_types.clear();
        self.expr_types.clear();

        for statement in &program.statements {
            match &statement.kind {
                StmtKind::Function(decl) => self.check_function(decl),
                _ => self.check_stmt(statement),
            }
        }
    }

    fn check_function(&mut self, decl: &FunctionDecl) {
        let Some(&index) = self.by_name.get(&decl.name.name) else {
            return;
        };

        for param in &decl.params {
            let ty = self.annotation_type(&param.ty);
            self.bind(param.name.span, ty);
        }

        let previous_expected = self
            .expected_return
            .replace(self.signatures[index].return_type.clone().unwrap_or(Type::Unknown));
        let previous_returns = std::mem::take(&mut self.returns);

        for statement in &decl.body.statements {
            self.check_stmt(statement);
        }

        let returns = std::mem::replace(&mut self.returns, previous_returns);
        self.expected_return = previous_expected;

        self.settle_return_type(index, returns, decl);
    }

    fn settle_return_type(
        &mut self,
        index: usize,
        returns: Vec<(Type, Span)>,
        decl: &FunctionDecl,
    ) {
        if self.signatures[index].return_type.is_none() {
            if let Some((ty, _)) = returns.iter().find(|(ty, _)| !ty.is_unknown()) {
                self.signatures[index].return_type = Some(ty.clone());
                self.progress = true;
            }
            return;
        }

        let declared = self.signatures[index].return_type.clone().unwrap_or(Type::Unknown);

        if self.silent || declared == Type::Unit || declared.is_unknown() {
            return;
        }

        // Spec 5.3: a function returning anything but `Unit` must return on every path.
        if !always_returns(&decl.body) {
            let kind = TypeErrorKind::MissingReturn {
                name: self.signatures[index].name.clone(),
                declared,
            };
            self.report(kind, decl.name.span);
        }
    }

    pub(crate) fn report(&mut self, kind: TypeErrorKind, span: Span) {
        if self.silent {
            return;
        }

        self.errors.push(TypeError { kind, span });
    }

    pub(crate) fn bind(&mut self, span: Span, ty: Type) {
        if let Some(id) = self.resolution.binding_id(span) {
            self.var_types.insert(id, ty);
        }
    }

    pub(crate) fn type_of_binding(&self, span: Span) -> Type {
        self.resolution
            .binding_id(span)
            .and_then(|id| self.var_types.get(&id))
            .cloned()
            .unwrap_or(Type::Unknown)
    }

    /// Converts an annotation and reports what is wrong with it. A `Unit` annotation comes
    /// back as `Unknown` so the initializer is not then reported as a second mismatch; an
    /// invalid key type keeps its shape, which still tells the literal what to be.
    pub(crate) fn annotation_type(&mut self, node: &crate::ast::TypeNode) -> Type {
        let ty = Type::from_node(node);

        if ty.mentions_unit() {
            self.report(TypeErrorKind::UnitAnnotation, node.span);
            return Type::Unknown;
        }

        self.reject_invalid_dict_keys(&ty, node.span);
        ty
    }

    pub(crate) fn reject_invalid_dict_keys(&mut self, ty: &Type, span: Span) {
        match ty {
            Type::List(item) => self.reject_invalid_dict_keys(item, span),
            Type::Dict(key, value) => {
                if !key.is_valid_dict_key() {
                    let kind = TypeErrorKind::InvalidDictKeyType((**key).clone());
                    self.report(kind, span);
                }
                self.reject_invalid_dict_keys(value, span);
            }
            _ => {}
        }
    }
}

fn has_valued_return(block: &Block) -> bool {
    block.statements.iter().any(statement_has_valued_return)
}

fn statement_has_valued_return(statement: &Stmt) -> bool {
    match &statement.kind {
        StmtKind::Return(value) => value.is_some(),
        StmtKind::If { arms, else_block } => {
            arms.iter().any(|(_, block)| has_valued_return(block))
                || else_block.as_ref().is_some_and(has_valued_return)
        }
        StmtKind::While { body, .. } | StmtKind::For { body, .. } => has_valued_return(body),
        _ => false,
    }
}

/// Whether control cannot leave this block without returning. Loops never count: the
/// compiler does not reason about whether one runs (spec 8.2).
fn always_returns(block: &Block) -> bool {
    block.statements.iter().any(statement_always_returns)
}

fn statement_always_returns(statement: &Stmt) -> bool {
    match &statement.kind {
        StmtKind::Return(_) => true,
        StmtKind::If { arms, else_block } => {
            let Some(else_block) = else_block else {
                return false;
            };

            arms.iter().all(|(_, block)| always_returns(block)) && always_returns(else_block)
        }
        _ => false,
    }
}
