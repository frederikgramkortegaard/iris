use crate::ast::Statement;
use crate::mir::analysis::cfg;
use crate::mir::{BlockId, Function, Operand, Reg};
use std::collections::{HashMap, HashSet};

pub type Header = BlockId;
pub type Latch = BlockId;

/// An induction variable: starts at start, increments by step each iteration
#[derive(Debug, Clone)]
pub struct InductionVar {
    pub start: Operand,
    pub step: Operand,
}

/// Trip count of a loop
#[derive(Debug, Clone, Default)]
pub enum TripCount {
    Constant(i64),
    Symbolic(Operand),
    #[default]
    Unknown,
}

#[derive(Debug)]
pub struct Loop {
    // Core structure (required)
    pub header: BlockId,
    pub latches: Vec<BlockId>,
    pub body: HashSet<BlockId>,
    pub parent: Option<BlockId>,

    //
    pub ast: Option<Statement>,

    // Analysis results (populated later)
    pub invariants: HashSet<Reg>,
    pub ivs: HashMap<Reg, InductionVar>,
    pub trip_count: TripCount,
    pub exits: Vec<BlockId>,
    pub preheader: Option<BlockId>,
    pub depth: usize,
}

impl Loop {
    pub fn new(
        header: BlockId,
        latches: Vec<BlockId>,
        body: HashSet<BlockId>,
        parent: Option<BlockId>,
    ) -> Self {
        Loop {
            header,
            latches,
            body,
            parent,
            ast: None,
            invariants: HashSet::new(),
            ivs: HashMap::new(),
            trip_count: TripCount::default(),
            exits: Vec::new(),
            preheader: None,
            depth: 0,
        }
    }
}

/// Find all back edges in the CFG
/// A back edge is an edge from a node to one of its dominators
pub fn find_back_edges(
    function: &Function,
    successors: &cfg::Successors,
    dominators: &cfg::DominatorSets,
) -> HashMap<Header, Vec<Latch>> {
    let empty: HashSet<BlockId> = HashSet::new();

    let mut back_edges: HashMap<Header, Vec<Latch>> = HashMap::new();
    for (id, _) in function.arena.iter() {
        for succ in successors.get(&id).unwrap_or(&vec![]) {
            if dominators.get(&id).unwrap_or(&empty).contains(succ) {
                back_edges.entry(*succ).or_default().push(id);
            }
        }
    }

    back_edges
}

/// Compute the body of a natural loop given its header and latches
pub fn compute_body(
    header: BlockId,
    latches: &[BlockId],
    predecessors: &cfg::Predecessors,
) -> HashSet<BlockId> {
    let mut body = HashSet::new();
    body.insert(header);
    let mut stack: Vec<BlockId> = latches.to_vec();

    while let Some(node) = stack.pop() {
        if !body.contains(&node) {
            body.insert(node);
            if let Some(preds) = predecessors.get(&node) {
                stack.extend(preds.iter().copied());
            }
        }
    }
    body
}

/// Find all natural loops in the function
pub fn find_loops(
    back_edges: &HashMap<Header, Vec<Latch>>,
    predecessors: &cfg::Predecessors,
) -> Vec<Loop> {
    // First compute all bodies
    let mut loop_data: Vec<(BlockId, Vec<BlockId>, HashSet<BlockId>)> = vec![];
    for (header, latches) in back_edges {
        let body = compute_body(*header, latches, predecessors);
        loop_data.push((*header, latches.clone(), body));
    }

    // Sort by body size descending (outer loops first)
    loop_data.sort_by_key(|x| std::cmp::Reverse(x.2.len()));

    let mut loops = vec![];
    for (header, latches, body) in loop_data {
        // Find parent: smallest existing loop that contains our header
        let parent = loops
            .iter()
            .filter(|l: &&Loop| l.body.contains(&header))
            .min_by_key(|l| l.body.len())
            .map(|l| l.header);

        loops.push(Loop::new(header, latches, body, parent));
    }

    loops
}

/// Analyze a function and return all loops
pub fn analyze(function: &Function) -> Vec<Loop> {
    let (predecessors, successors) = cfg::compute_cfg(function);
    let dominators = cfg::compute_dominators(function, &predecessors);
    let back_edges = find_back_edges(function, &successors, &dominators);
    find_loops(&back_edges, &predecessors)
}
