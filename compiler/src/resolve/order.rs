use std::collections::HashSet;

use super::Resolver;
use crate::error::ResolveErrorKind;

impl Resolver {
    /// Spec 8.5: a module-scope call must not be able to reach a function that reads a
    /// module-scope variable declared after that call, because the initializer has not run
    /// yet. A body may call a function declared further down the file (5.5), so this is not
    /// implied by textual order and needs the call graph.
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

                if let Some(read) = node
                    .global_reads
                    .iter()
                    .find(|read| read.declared_at > call.statement_index)
                {
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
