use super::Checker;
use crate::ast::{AssignTarget, Block, Expr, Stmt, StmtKind};
use crate::error::TypeErrorKind;
use crate::types::Type;

impl Checker<'_> {
    pub(crate) fn check_stmt(&mut self, statement: &Stmt) {
        match &statement.kind {
            StmtKind::Assign { target, ty, value } => self.check_assign(target, ty.as_ref(), value),
            StmtKind::Expr(expr) => {
                self.check_expr(expr, None);
            }
            StmtKind::Function(_) => {}
            StmtKind::If { arms, else_block } => {
                for (condition, block) in arms {
                    self.check_condition(condition);
                    self.check_block(block);
                }
                if let Some(block) = else_block {
                    self.check_block(block);
                }
            }
            StmtKind::While { condition, body } => {
                self.check_condition(condition);
                self.check_block(body);
            }
            StmtKind::For {
                variable,
                iterable,
                body,
            } => {
                let element = self.element_type(iterable);
                self.bind(variable.span, element);
                self.check_block(body);
            }
            StmtKind::Return(value) => self.check_return(value.as_ref(), statement.span),
            StmtKind::Break | StmtKind::Continue => {}
        }
    }

    fn check_block(&mut self, block: &Block) {
        for statement in &block.statements {
            self.check_stmt(statement);
        }
    }

    fn check_condition(&mut self, condition: &Expr) {
        let found = self.value_of(condition, Some(&Type::Bool));

        if !found.matches(&Type::Bool) {
            self.report(TypeErrorKind::ConditionNotBool(found), condition.span);
        }
    }

    fn element_type(&mut self, iterable: &Expr) -> Type {
        let found = self.value_of(iterable, None);

        match &found {
            Type::List(item) => (**item).clone(),
            Type::Str => Type::Str,
            Type::Unknown => Type::Unknown,
            _ => {
                self.report(TypeErrorKind::NotIterable(found), iterable.span);
                Type::Unknown
            }
        }
    }

    fn check_assign(
        &mut self,
        target: &AssignTarget,
        annotation: Option<&crate::ast::TypeNode>,
        value: &Expr,
    ) {
        match target {
            AssignTarget::Name(name) => match annotation {
                Some(node) => {
                    let annotated = self.annotation_type(node);
                    let found = self.value_of(value, Some(&annotated));

                    if !found.matches(&annotated) {
                        let kind = TypeErrorKind::AnnotationMismatch {
                            annotated: annotated.clone(),
                            found,
                        };
                        self.report(kind, value.span);
                    }

                    self.bind(name.span, annotated);
                }
                None if self.resolution.declares(name.span) => {
                    let found = self.value_of(value, None);
                    self.bind(name.span, found);
                }
                None => {
                    let expected = self.type_of_binding(name.span);
                    let found = self.value_of(value, Some(&expected));

                    if !found.matches(&expected) {
                        let kind = TypeErrorKind::ReassignMismatch {
                            name: name.name.clone(),
                            expected,
                            found,
                        };
                        self.report(kind, value.span);
                    }
                }
            },
            AssignTarget::Index {
                object,
                index,
                span,
            } => self.check_index_assign(object, index, value, *span),
        }
    }

    fn check_index_assign(&mut self, object: &Expr, index: &Expr, value: &Expr, span: crate::token::Span) {
        let container = self.value_of(object, None);

        let (key, element) = match &container {
            Type::List(item) => (Type::Int, (**item).clone()),
            Type::Dict(key, item) => ((**key).clone(), (**item).clone()),
            Type::Str => {
                self.report(TypeErrorKind::StringsAreImmutable, span);
                (Type::Int, Type::Unknown)
            }
            Type::Unknown => (Type::Unknown, Type::Unknown),
            _ => {
                self.report(TypeErrorKind::NotIndexable(container), object.span);
                (Type::Unknown, Type::Unknown)
            }
        };

        let found_key = self.value_of(index, Some(&key));
        if !found_key.matches(&key) {
            let kind = TypeErrorKind::IndexType {
                expected: key,
                found: found_key,
            };
            self.report(kind, index.span);
        }

        let found = self.value_of(value, Some(&element));
        if !found.matches(&element) {
            let kind = TypeErrorKind::ReassignMismatch {
                name: "this element".to_string(),
                expected: element,
                found,
            };
            self.report(kind, value.span);
        }
    }

    fn check_return(&mut self, value: Option<&Expr>, span: crate::token::Span) {
        let Some(expected) = self.expected_return.clone() else {
            self.report(TypeErrorKind::ReturnOutsideFunction, span);
            return;
        };

        let found = match value {
            Some(expr) => self.value_of(expr, Some(&expected)),
            None => Type::Unit,
        };

        self.returns.push((found.clone(), span));

        if !found.matches(&expected) {
            let kind = TypeErrorKind::ReturnTypeMismatch { expected, found };
            self.report(kind, span);
        }
    }
}
