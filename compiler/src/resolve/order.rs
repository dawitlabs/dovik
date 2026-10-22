use std::collections::HashSet;

use super::Resolver;
use crate::error::ResolveErrorKind;

impl Resolver {
    pub(crate) fn check_module_call_order(&mut self) {
        let mut found = Vec::new();

        for call in &self.graph.module_calls {
            let mut visited = HashSet::new();
            let mut pending = vec![call.function];

            while let Some(index) = pending.pop() {
                if !visited.insert(index) {
                    continue;
                }

                let node = &self.graph.nodes[index];

                if let Some(read) = node.global_reads.iter().find(|read| {
                    read.unit == call.unit && read.declared_at > call.statement_index
                }) {
                    found.push((
                        ResolveErrorKind::CallReachesLaterGlobal {
                            callee: node.name.clone(),
                            global: read.name.clone(),
                        },
                        call.span,
                    ));
                    break;
                }

                pending.extend(node.calls.iter().copied());
            }
        }

        for (kind, span) in found {
            self.error(kind, span);
        }
    }
}
