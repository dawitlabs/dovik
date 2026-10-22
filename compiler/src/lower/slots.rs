use super::Lowerer;
use crate::ir;
use crate::token::Span;
use crate::types::Type;

pub(crate) enum Place {
    Local(ir::LocalId),
    Global(ir::GlobalId),
    Missing,
}

impl Lowerer<'_> {
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
}
