use crate::mir::analysis::callgraph::Callgraph;
use crate::mir::FuncId;
use std::collections::HashSet;

/// True if the function participates in any call cycle.
/// Seeded from the function's callees, not the function itself, otherwise
/// every function would trivially "reach" itself at depth zero.
pub fn can_function_reach_itself(function: FuncId, callgraph: &Callgraph) -> bool {
    let mut visited: HashSet<FuncId> = HashSet::new();
    let mut worklist: Vec<FuncId> = callgraph
        .get(&function)
        .map(|children| children.iter().copied().collect())
        .unwrap_or_default();

    while let Some(current) = worklist.pop() {
        if current == function {
            return true;
        }
        if !visited.insert(current) {
            continue;
        }
        if let Some(children) = callgraph.get(&current) {
            worklist.extend(children.iter().copied());
        }
    }

    false
}
