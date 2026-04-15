use crate::diagnostics::DiagnosticCollector;
use crate::mir::analysis::cfg;
use crate::mir::passes::MirPass;
use crate::mir::visitor::MirVisitor;
use crate::mir::Program;
use crate::mir::{BlockId, Function, Opcode, Operand, Reg};
use std::collections::HashMap;

#[derive(Hash, PartialEq, Eq, Clone)]
enum GVNOperand {
    Reg(Reg),
    ImmI64(i64),
    ImmF64Bits(u64), // store as bits for hashing
    ImmBool(bool),
    Label(String),
    Pair(BlockId, Box<GVNOperand>), // Used for Phi nodes
}
#[derive(Hash, PartialEq, Eq, Clone)]
struct GVNKey {
    op: Opcode,
    args: Vec<GVNOperand>,
}
pub type GVNValue = Reg;

impl From<&Operand> for GVNOperand {
    fn from(op: &Operand) -> Self {
        match op {
            Operand::Reg(r) => GVNOperand::Reg(*r),
            Operand::ImmI64(i) => GVNOperand::ImmI64(*i),
            Operand::ImmF64(f) => GVNOperand::ImmF64Bits(f.to_bits()),
            Operand::ImmBool(b) => GVNOperand::ImmBool(*b),
            Operand::Label(s) => GVNOperand::Label(s.clone()),
            Operand::Pair(block, inner) => {
                GVNOperand::Pair(*block, Box::new(GVNOperand::from(inner.as_ref())))
            }
        }
    }
}

/// GVN (Global Value Numbering)
pub struct MirGVNPass {
    diagnostics: DiagnosticCollector,
    valuemap: HashMap<GVNKey, GVNValue>,
}

impl Default for MirGVNPass {
    fn default() -> Self {
        Self::new()
    }
}

impl MirGVNPass {
    pub fn new() -> Self {
        MirGVNPass {
            diagnostics: DiagnosticCollector::new(),
            valuemap: HashMap::new(),
        }
    }

    fn walk_domtree(
        &mut self,
        child_dtree: &HashMap<BlockId, Vec<BlockId>>,
        function: &mut Function,
        blockid: BlockId,
    ) {
        let mut added: Vec<GVNKey> = vec![];
        let block = function.arena.get_mut(blockid);

        for instruction in &mut block.instructions {
            let key = GVNKey {
                op: instruction.op.clone(),
                args: instruction.args.iter().map(GVNOperand::from).collect(),
            };

            if let Some(r) = self.valuemap.get(&key) {
                instruction.op = Opcode::Copy;
                instruction.args = vec![Operand::Reg(*r)]
            } else {
                added.push(key.clone());
                self.valuemap.insert(key, instruction.dest);
            }
        }

        for child in child_dtree.get(&blockid).unwrap_or(&vec![]) {
            self.walk_domtree(child_dtree, function, *child)
        }

        self.valuemap.retain(|k, _| !added.contains(k));
    }
}

// The visitor is used to mark liveness and build the defmap
impl MirVisitor for MirGVNPass {
    type Output = ();

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }

    fn diagnostics_mut(&mut self) -> &mut DiagnosticCollector {
        &mut self.diagnostics
    }

    fn visit_function(&mut self, function: &mut Function) -> Self::Output {
        let (preds, succs) = cfg::compute_cfg(function);
        let doms = cfg::compute_dominators(function, &preds);
        let dtree = cfg::compute_dominator_tree(function, &doms, &succs);
        let mut child_dtree: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
        for (&child, &parent) in &dtree {
            child_dtree.entry(parent).or_default().push(child);
        }

        self.walk_domtree(&child_dtree, function, function.entry);
    }
}
impl MirPass for MirGVNPass {
    fn run(&mut self, program: &mut Program) {
        self.visit_program(program);
    }

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }
}
