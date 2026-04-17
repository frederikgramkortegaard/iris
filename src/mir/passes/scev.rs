use crate::diagnostics::DiagnosticCollector;
use crate::mir::analysis::dfg::DFGAnalysis;
use crate::mir::analysis::iv;
use crate::mir::analysis::loops::{InductionVar, Loop, TripCount};
use crate::mir::passes::MirPass;
use crate::mir::visitor::MirVisitor;
use crate::mir::{Function, Instruction, Opcode, Operand, Program, Terminator};

pub struct MirSCEVPass {
    diagnostics: DiagnosticCollector,
}

impl Default for MirSCEVPass {
    fn default() -> Self {
        Self::new()
    }
}

// Helper functions (no &self needed)
fn get_exit_condition<'a>(
    f: &'a Function,
    o: &'a Operand,
    dfg: &DFGAnalysis,
) -> Option<&'a Instruction> {
    let Operand::Reg(r) = o else { return None };
    let inst = dfg.get_instruction(f, *r)?;
    if inst.op == Opcode::Copy {
        return get_exit_condition(f, inst.args.first()?, dfg);
    }
    Some(inst)
}

fn get_iv<'a>(o: &Operand, l: &'a Loop) -> Option<&'a InductionVar> {
    let Operand::Reg(r) = o else { return None };
    l.ivs.get(r)
}

fn get_numeric(o: &Operand) -> Option<f64> {
    match o {
        Operand::ImmI64(v) => Some(*v as f64),
        Operand::ImmF64(v) => Some(*v),
        _ => None,
    }
}

fn try_get_trip_count(f: &Function, l: &Loop, dfg: &DFGAnalysis) -> Option<TripCount> {
    let Terminator::BrIf { cond, .. } = &f.block(l.header).terminator else {
        return None;
    };

    let exit = get_exit_condition(f, cond, dfg)?;

    let left_iv = get_iv(&exit.args[0], l);
    let right_iv = get_iv(&exit.args[1], l);

    let (iv, bound) = match (left_iv, right_iv) {
        (Some(iv), None) => (iv, &exit.args[1]),
        (None, Some(iv)) => (iv, &exit.args[0]),
        _ => return None,
    };

    let start = get_numeric(&iv.start)?;
    let step = get_numeric(&iv.step)?;

    match exit.op {
        Opcode::Lt | Opcode::Le => {
            if step <= 0.0 {
                return None;
            }
            let adjust = if exit.op == Opcode::Le { 1.0 } else { 0.0 };

            match get_numeric(bound) {
                Some(b) => {
                    let count = ((b + adjust - start) / step).ceil() as i64;
                    Some(TripCount::Constant(count.max(0)))
                }
                None => Some(TripCount::Symbolic(bound.clone())),
            }
        }

        Opcode::Gt | Opcode::Ge => {
            if step >= 0.0 {
                return None;
            }
            let adjust = if exit.op == Opcode::Ge { 1.0 } else { 0.0 };

            match get_numeric(bound) {
                Some(b) => {
                    let count = ((start - b + adjust) / (-step)).ceil() as i64;
                    Some(TripCount::Constant(count.max(0)))
                }
                None => Some(TripCount::Symbolic(bound.clone())),
            }
        }

        Opcode::Ne => {
            if step == 0.0 {
                return None;
            }

            match get_numeric(bound) {
                Some(b) => {
                    let diff = b - start;
                    if (diff > 0.0 && step <= 0.0) || (diff < 0.0 && step >= 0.0) {
                        return None;
                    }
                    let count = (diff / step) as i64;
                    Some(TripCount::Constant(count.max(0)))
                }
                None => Some(TripCount::Symbolic(bound.clone())),
            }
        }

        _ => None,
    }
}

impl MirSCEVPass {
    pub fn new() -> Self {
        MirSCEVPass {
            diagnostics: DiagnosticCollector::new(),
        }
    }

    fn get_trip_count(&self, f: &Function, l: &Loop, dfg: &DFGAnalysis) -> TripCount {
        try_get_trip_count(f, l, dfg).unwrap_or(TripCount::Unknown)
    }
}

impl MirVisitor for MirSCEVPass {
    type Output = ();

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }

    fn diagnostics_mut(&mut self) -> &mut DiagnosticCollector {
        &mut self.diagnostics
    }

    fn visit_function(&mut self, function: &mut Function) -> Self::Output {
        let Some(mut loops) = function.loops.take() else {
            return;
        };

        if loops.is_empty() {
            return;
        }

        let dfg = DFGAnalysis::compute(function);

        for lop in &mut loops {
            lop.ivs = iv::compute(function, lop, &dfg);
            lop.trip_count = self.get_trip_count(function, lop, &dfg);

            if crate::is_verbose() {
                println!("Loop {:?}:", lop.header);
                for (reg, iv) in &lop.ivs {
                    println!("  IV r{}: {:?}", reg, iv);
                }
                println!("  Trip count: {:?}", lop.trip_count);
            }
        }

        function.loops = Some(loops);
    }
}

impl MirPass for MirSCEVPass {
    fn run(&mut self, program: &mut Program) {
        self.visit_program(program);
    }

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }
}
