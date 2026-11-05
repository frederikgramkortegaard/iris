use crate::mir::{FuncId, FunctionArena, Opcode, Operand};
use log::debug;
use std::collections::{BTreeMap, BTreeSet};

pub type Callgraph = BTreeMap<FuncId, BTreeSet<FuncId>>;

pub fn compute_callgraph(f_arena: &FunctionArena) -> Callgraph {
    let mut callgraph: Callgraph = f_arena
        .iter()
        .map(|(fid, _)| (fid, BTreeSet::new()))
        .collect();
    let name_to_id: BTreeMap<String, FuncId> = f_arena
        .iter()
        .map(|(fid, f)| (f.name.clone(), fid))
        .collect();

    for (fid, func) in f_arena.iter() {
        for (_, block) in func.arena.iter() {
            for name in block
                .instructions
                .iter()
                .filter_map(|i| match (&i.op, i.args.first()) {
                    (Opcode::Call, Some(Operand::Label(name))) => Some(name),
                    _ => None,
                })
            {
                let Some(&callee) = name_to_id.get(name) else {
                    debug!("call to unknown function '{}', skipping edge", name);
                    continue;
                };
                callgraph
                    .get_mut(&fid)
                    .expect("function pre-inserted")
                    .insert(callee);
            }
        }
    }

    callgraph
}
