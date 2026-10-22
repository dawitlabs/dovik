mod builtin;
mod call;
mod flow;
mod expr;
mod stmt;

use std::collections::HashMap;

use crate::ast::{FunctionDecl, StmtKind};
use crate::modules::Graph;
use flow::{always_returns, has_valued_return};
use crate::error::{TypeError, TypeErrorKind};
use crate::resolve::{BindingId, Resolution};
use crate::token::Span;
use crate::types::Type;

#[derive(Debug, Default)]
pub struct Typed {
    pub expr_types: HashMap<Span, Type>,
    pub var_types: HashMap<BindingId, Type>,
    pub functions: HashMap<BindingId, FunctionType>,
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

pub fn check(graph: &Graph, resolution: &Resolution) -> (Typed, Vec<TypeError>) {
    let mut checker = Checker {
        graph,
        resolution,
        signatures: Vec::new(),
        by_binding: HashMap::new(),
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

    checker.errors.sort_by_key(|error| error.span.start);

    let functions = checker
        .signatures
        .iter()
        .map(|signature| {
            (
                signature.binding,
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
    pub(crate) binding: BindingId,
    pub(crate) span: Span,
    pub(crate) params: Vec<Type>,
    pub(crate) return_type: Option<Type>,
}

pub(crate) struct Checker<'a> {
    pub(crate) graph: &'a Graph,
    pub(crate) resolution: &'a Resolution,
    pub(crate) signatures: Vec<Signature>,
    pub(crate) by_binding: HashMap<BindingId, usize>,
    pub(crate) var_types: HashMap<BindingId, Type>,
    pub(crate) errors: Vec<TypeError>,
    pub(crate) silent: bool,
    pub(crate) expected_return: Option<Type>,
    pub(crate) returns: Vec<(Type, Span)>,
    pub(crate) expr_types: HashMap<Span, Type>,
    pub(crate) progress: bool,
}

impl<'a> Checker<'a> {
    fn collect_signatures(&mut self) {
        for unit in &self.graph.units {
            for statement in &unit.ast.statements {
                let StmtKind::Function(decl) = &statement.kind else {
                    continue;
                };

                self.collect_signature(decl);
            }
        }
    }

    fn collect_signature(&mut self, decl: &FunctionDecl) {
        {
            let Some(binding) = self.resolution.binding_id(decl.name.span) else {
                return;
            };

            if self.by_binding.contains_key(&binding) {
                return;
            }

            let params = decl
                .params
                .iter()
                .map(|p| match Type::from_node(&p.ty) {
                    ty if ty.mentions_unit() => Type::Unknown,
                    ty => ty,
                })
                .collect();

            let return_type = match &decl.return_type {
                Some(node) => Some(Type::from_node(node)),
                None if !has_valued_return(&decl.body) => Some(Type::Unit),
                None => None,
            };

            self.by_binding.insert(binding, self.signatures.len());
            self.signatures.push(Signature {
                name: decl.name.name.clone(),
                binding,
                span: decl.name.span,
                params,
                return_type,
            });
        }
    }

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
        self.var_types.clear();
        self.expr_types.clear();

        for unit in &self.graph.units {
            for statement in &unit.ast.statements {
                match &statement.kind {
                    StmtKind::Function(decl) => self.check_function(decl),
                    _ => self.check_stmt(statement),
                }
            }
        }
    }

    fn check_function(&mut self, decl: &FunctionDecl) {
        let index = self
            .resolution
            .binding_id(decl.name.span)
            .and_then(|binding| self.by_binding.get(&binding).copied());

        let Some(index) = index else {
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
