use crate::diagnostics::DiagnosticCollector;
use crate::mir::analysis::cfg;
use crate::mir::passes::MirPass;
use crate::mir::visitor::MirVisitor;
use crate::mir::Program;
use crate::mir::{BlockId, Function, Opcode, Operand, Reg};
use std::collections::{BTreeMap, HashMap};

#[derive(Hash, PartialEq, Eq, Clone)]
enum CSEOperand {
    Reg(Reg),
    ImmI64(i64),
    ImmF64Bits(u64), // store as bits for hashing
    ImmBool(bool),
    Label(String),
    Pair(BlockId, Box<CSEOperand>), // Used for Phi nodes
}
#[derive(Hash, PartialEq, Eq, Clone)]
struct CSEKey {
    op: Opcode,
    args: Vec<CSEOperand>,
}
pub type CSEValue = Reg;

impl From<&Operand> for CSEOperand {
    fn from(op: &Operand) -> Self {
        match op {
            Operand::Reg(r) => CSEOperand::Reg(*r),
            Operand::ImmI64(i) => CSEOperand::ImmI64(*i),
            Operand::ImmF64(f) => CSEOperand::ImmF64Bits(f.to_bits()),
            Operand::ImmBool(b) => CSEOperand::ImmBool(*b),
            Operand::Label(s) => CSEOperand::Label(s.clone()),
            Operand::Pair(block, inner) => {
                CSEOperand::Pair(*block, Box::new(CSEOperand::from(inner.as_ref())))
            }
        }
    }
}

/// Common Subexpression Elimination, scoped by the dominator tree.
/// Note this is not real value numbering: keys are register names, so
/// a + b and a2 + b don't merge even if a2 is a copy of a, and there is
/// no commutative canonicalization. Used to be called GVN which oversold it.
pub struct MirCSEPass {
    diagnostics: DiagnosticCollector,
    valuemap: HashMap<CSEKey, CSEValue>,
    changed: bool,
}

impl Default for MirCSEPass {
    fn default() -> Self {
        Self::new()
    }
}

impl MirCSEPass {
    pub fn new() -> Self {
        MirCSEPass {
            diagnostics: DiagnosticCollector::new(),
            valuemap: HashMap::new(),
            changed: false,
        }
    }

    /// Whether the last run modified the program
    pub fn changed(&self) -> bool {
        self.changed
    }

    fn walk_domtree(
        &mut self,
        child_dtree: &BTreeMap<BlockId, Vec<BlockId>>,
        function: &mut Function,
        blockid: BlockId,
    ) {
        let mut added: Vec<CSEKey> = vec![];
        let block = function.arena.get_mut(blockid);

        for instruction in &mut block.instructions {
            // Calls are impure, merging two identical calls would change
            // how often they run. DCE already treats them as side-effecting.
            if instruction.op == Opcode::Call {
                continue;
            }

            let key = CSEKey {
                op: instruction.op.clone(),
                args: instruction.args.iter().map(CSEOperand::from).collect(),
            };

            if let Some(r) = self.valuemap.get(&key) {
                instruction.op = Opcode::Copy;
                instruction.args = vec![Operand::Reg(*r)];
                self.changed = true;
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

impl MirVisitor for MirCSEPass {
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
        let mut child_dtree: BTreeMap<BlockId, Vec<BlockId>> = BTreeMap::new();
        for (&child, &parent) in &dtree {
            child_dtree.entry(parent).or_default().push(child);
        }

        self.walk_domtree(&child_dtree, function, function.entry);
    }
}
impl MirPass for MirCSEPass {
    fn run(&mut self, program: &mut Program) {
        self.visit_program(program);
    }

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }
}
