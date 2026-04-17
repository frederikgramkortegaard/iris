# Iris Compiler Study Guide

A comprehensive study guide for understanding the Iris compiler architecture, designed for interview preparation.

---

## The One-Sentence Pitch

Iris is a compiler I built from scratch in Rust — no dependencies — that takes a custom language, builds an SSA-based IR, runs a suite of optimization passes to a fixed point, then uses Ramsey's 2022 algorithm to recover structured control flow and emit WebAssembly.

---

## The Pipeline

```
Source (.iris)
  |
  +-- Lexer -> Parser
  |
  v
 HIR (High-level IR)
  |-- Type Checking & Inference
  |-- AST Simplification
  |
  v
 MIR (Mid-level IR, SSA form)
  |-- Analysis:
  |     CFG Construction
  |     Dominator Trees & Frontiers
  |-- SSA:
  |     Phi Insertion (iterated dominance frontiers)
  |     Variable Renaming (dominator tree walk)
  |-- Optimization (iterated):
  |     Constant Propagation (SCCP)
  |     Loop Invariant Code Motion
  |     Global Value Numbering
  |     Copy Propagation
  |     Dead Code Elimination
  |-- Tail Call Optimization
  |-- SSA Deconstruction (parallel copies)
  |-- Register Compaction
  |-- Dead Block Elimination
  |
  v
 WAT (WebAssembly Text)
  |-- Ramsey (2022) Structure Recovery
  |-- MIR -> WAT IR Lowering
  |-- Peephole Optimization
  |-- Text Emission
  |
  v
 .wat output
```

---

## CFG Construction (cfg.rs)

### What Was Built

CFG construction computes the directed graph of basic blocks by walking the terminators (branch instructions) of each block and recording predecessor and successor relationships.

### How It Works

The `compute_cfg` function:
1. Initializes empty predecessor and successor lists for every block
2. Iterates over each block and examines its terminator
3. For `Br { target }`, adds an edge block → target
4. For `BrIf { then_bb, else_bb, ... }`, adds edges to both branches
5. Returns two maps: predecessors and successors

This is the foundation for all downstream graph analyses.

### How to Explain It in an Interview

Say: "I start with a function containing basic blocks. Each block ends with a branch terminator. I walk every block, look at its terminator, and record who it jumps to. This builds a predecessor map (for each block, who jumps here?) and a successor map (for each block, who do we jump to?). This graph is the input to dominator analysis and loop detection."

### Common Follow-Ups

- **What about edges you can't statically determine?** Iris is a compiler for a structured language with explicit control flow — no indirect jumps, no computed branches. Every jump target is known at compile time.
- **Do you handle exceptions or other control flow?** Not in the current design. The terminator types are Br, BrIf, Ret, and Unreachable.

---

## Dominator Analysis (cfg.rs)

Dominance is central to SSA construction and optimization. A node X dominates node Y if every path from the entry block to Y must pass through X.

### Computing Dominator Sets (Iterative Intersection Algorithm)

**What it does:** Builds a set Dom[block] = all blocks that dominate that block (including the block itself).

**Algorithm (standard textbook approach):**
1. Initialize Dom[entry] = {entry} and Dom[all others] = all blocks
2. Iterate until fixpoint:
   - For each block B (except entry):
     - Compute Dom[B] = {B} ∪ (intersect Dom[P] for all predecessors P)
3. Stop when no Dom set changes

**Why it works:** The intersection of predecessor dominators captures the common ancestors. Adding the block itself ensures self-inclusion. Fixed-point iteration guarantees convergence.

**Key insight for interviews:** This is O(N²) per iteration in the worst case, but converges quickly in practice. More efficient algorithms like Lengauer-Tarjan are O(N log N), but for compiler passes on small-to-medium IR, the simple approach is good enough.

**Code walkthrough:** In cfg.rs lines 115–157, the initialization sets up Dom sets, then the loop intersects predecessor dominators until stabilization.

### Computing the Dominator Tree (Finding Immediate Dominator)

**What it does:** For each block, finds its immediate dominator (idom) — the unique strict dominator closest to it.

**Algorithm:**
1. Compute all strict dominators for each block (dominators except itself)
2. For each block B:
   - Candidate idoms are all strict dominators
   - The idom is the one that does NOT dominate any other strict dominator
   - In other words, it's the "closest" strict dominator

**Why it works:** If candidate C dominates another strict dominator D, then C is not the immediate dominator (D is in between). The one that doesn't dominate another strict dominator must be the closest.

**Key insight:** The dominator tree is a tree, not a DAG. Every non-entry node has exactly one idom. This tree is used heavily in SSA renaming.

**Code walkthrough:** cfg.rs lines 183–217. Compute strict dominators by filtering, then for each candidate check if it dominates another (if yes, skip it).

### Computing Dominance Frontiers (Walking Up from Predecessors)

**What it does:** For each block, computes its dominance frontier — the set of blocks where dominance by that block "ends."

**Intuition:** DF[X] = blocks where X dominates some but not all predecessors.

**Algorithm (classic):**
1. For each block Y with 2+ predecessors (join points):
   - For each predecessor P of Y:
     - Start runner = P
     - Walk up the dominator tree from P until reaching idom(Y)
     - Add Y to DF[runner] for each runner visited
     - Move runner = idom(runner)

**Why it works:** A value defined at X is live at Y if it flows through different predecessors. Those predecessors are exactly where dominance by X changes. Walking up the dominator tree from a predecessor efficiently finds all such blocks.

**Key insight:** Dominance frontiers are the join points relative to a definition — exactly where phi nodes need to go.

**Code walkthrough:** cfg.rs lines 160–181. For each block with 2+ preds, walk up from each pred until hitting idom(block).

### If Asked "Why Not Lengauer-Tarjan?"

Good answer: "Lengauer-Tarjan is O(N log N) or O(N α(N)) versus our O(N²) iterative approach. For large programs, that's significant. However, Iris currently targets small-to-medium IR sizes, and the iterative algorithm is simpler to understand and debug. If I were scaling to 100K+ blocks, I'd profile first — the dominator computation might not be the bottleneck. When it is, Lengauer-Tarjan would be the obvious upgrade."

---

## SSA Construction (ssa.rs)

SSA (Static Single Assignment) form ensures each register is assigned exactly once. Phi nodes are the mechanism for joining multiple definitions at merge points.

### Phase 1: Phi Insertion via Iterated Dominance Frontiers

**What it does:** Determines where phi nodes must be inserted.

**Key insight:** A phi is needed at block B for variable V if B is in the dominance frontier of some block that defines V.

**Algorithm:**
1. Build a map of which blocks define each register
2. For each register with multiple definitions:
   - Initialize a worklist with all defining blocks
   - Iterate:
     - For each block in worklist, add its dominance frontier blocks to the worklist
     - For each frontier block, insert a phi for that register
     - If the frontier block was not previously in has_phi, add it to the worklist (fixpoint)
3. Stop when the worklist is empty

**Why it works:** Dominance frontier is transitive. Inserting a phi at a frontier block might create a new definition, requiring phis higher up. The fixpoint iteration ensures all necessary phis are found.

**Code walkthrough:** ssa.rs lines 174–216. Build definitions map, then for each register with 2+ defs, iterate until all frontier phis are inserted.

### Phase 2: Variable Renaming via Dominator Tree Walk

**What it does:** Renames each register use/def so that the register number reflects which assignment it refers to.

**Algorithm (recursive dominator tree traversal):**
1. Maintain a counter and stack for each register
2. Call rename(entry_block):
   - For each phi in the block:
     - Increment counter[register]
     - Allocate a fresh register number
     - Push fresh register onto stack[register]
     - Update phi.dest to the fresh number
   - For each instruction:
     - Replace register uses with the top of stack[register]
     - Allocate a fresh register for the destination
     - Push fresh register onto stack[register]
   - Fill phi args in successor blocks using the current stack state
   - Recurse to dominator tree children
   - Pop everything we pushed (restore stack when leaving)

**Why it works:** The dominator tree traversal ensures dominators are processed before their children. Stack-based renaming captures scope: when you enter a block, you see all definitions from dominators; when you leave, you restore the state.

**Key insight:** Phi nodes get renamed in the phi phase, then instruction uses/defs get renamed. This ensures phis correctly capture the joining values.

**Code walkthrough:** ssa.rs lines 30–172. Invert the dominator tree to get children, then recursive rename function with counter/stack management.

---

## Constant Propagation (const_prop.rs)

### What It Does

Forwards constants through copy instructions. If you see `r5 = copy r3` and r3 is a known constant, then r5 becomes a constant too. All uses of r5 are then replaced with that constant.

### What It DOESN'T Do

- **No constant folding:** Doesn't evaluate `r5 = add 2 3` to `r5 = 5`. That's left to DCE to remove.
- **No full SCCP:** Doesn't track a lattice with top/constant/bottom states, doesn't track branch reachability or conditional constants.
- **README calls it SCCP but the implementation is simpler:** True. Real SCCP is more sophisticated. This pass is "forward constant propagation" or "copy constant propagation."

### How to Explain the Difference if Asked About SCCP

Say: "Real SCCP (Sparse Conditional Constant Propagation) models each variable as a lattice: top (unknown), constant (a specific value), or bottom (multiple conflicting constants). It then uses a worklist algorithm to propagate facts through the CFG and across branches, marking unreachable branches. My implementation is simpler: I just track constants flowing through copy instructions. I don't evaluate arbitrary expressions, and I don't track branch reachability. For a full SCCP upgrade, I'd need to add expression evaluation, a lattice, and CFG-aware propagation."

### Code Walkthrough

const_prop.rs: Maintain a constant_map: register → constant. For each instruction, replace uses of known constants. When you see a copy of a constant, add to the map.

---

## Global Value Numbering (gvn.rs)

### What It Does

Assigns a "value number" to each computation. If two instructions compute the same value (same opcode, same arguments), they get the same value number. The second one is replaced with a copy of the first.

### Hash-Based Implementation

**Data structure:** A HashMap from (opcode, args) → register.

**Key detail:** f64 values are stored as their bit pattern (u64) for hashing, since f64 doesn't implement Hash directly.

**Algorithm:**
1. Walk the dominator tree
2. For each block:
   - For each instruction:
     - Compute key = (opcode, args converted to GVNOperand)
     - If key exists in valuemap, replace instruction with Copy of existing result
     - Otherwise, insert key → dest into valuemap
   - Recurse to dominator children
   - Remove entries from valuemap that were added in this block (scoping)

### Why the Dominator Tree Walk?

Correctness: A value is only safe to reuse if it dominates the current instruction. Walking the tree ensures we only see values from dominating blocks. Removing entries when exiting ensures we don't reuse values from sibling subtrees.

### Code Walkthrough

gvn.rs lines 60–89: walk_domtree maintains the valuemap, inserting new values and removing them when leaving a subtree. Lines 25–38 convert Operand to GVNOperand, handling f64 as bits.

---

## Loop Invariant Code Motion (loops.rs)

### Finding Natural Loops via Back Edges

**Definition:** A back edge is an edge where the target dominates the source. Back edges identify loop headers.

**Algorithm:**
1. Compute dominators and successors
2. For each edge (A → B):
   - If A dominates B, it's a back edge (B is the loop header, A is the latch)
3. Group back edges by header

**Why it works:** In a structured program, if A dominates B and jumps to B, then A must loop back to B (or be unreachable). So A → B is a back edge.

### Finding Invariants via Fixed-Point Iteration

**Definition:** A register is invariant if all its operands are either constants or loop-external (not defined in the loop body).

**Algorithm:**
1. Mark all loop-external registers as invariant
2. Iterate:
   - For each non-call instruction in the loop:
     - If all its operands are invariant, mark its destination as invariant
3. Stop when no new invariants are found

**Why it works:** Fixed-point ensures we find the closure of the invariant set. Non-call because calls can have side effects.

### Hoisting to Preheader

**What happens:**
1. Create a preheader block (empty block that jumps to the loop header)
2. Redirect all external predecessors of the loop header to the preheader instead
3. Collect all invariant instructions from the loop body
4. Move them to the preheader

**Code walkthrough:** loops.rs lines 153–247. Identify invariants, create preheader, redirect edges, move instructions.

### Safety Caveat

The pass doesn't check if an instruction might fault (e.g., divide by zero in a conditional). If an invariant instruction is inside a conditional that might not always execute, hoisting it could introduce a fault. Modern compilers handle this by tracking speculative execution or only hoisting from unconditional paths.

---

## Dead Code Elimination (dce.rs)

### Algorithm: Mark-and-Sweep

**Seed phase (mark live):**
1. Start with return values (they must be live)
2. Start with side-effecting instructions (calls are live)
3. For each live register, walk backwards through its def-use chain and mark all operands as live

**Working backwards:** If a register R is live, find the instruction that defines R. Mark all its operands as live. Recursively process those operands.

**Sweep phase:**
1. Remove any instruction whose destination is not in the live set
2. Remove any phi whose destination is not in the live set

### Why SSA Makes This Clean

In SSA, each register has exactly one definition. So the "def" of a register is unique and easy to find (stored in defmap). In conventional code, a register might have multiple definitions, and determining if a definition is live requires more sophisticated techniques (like data flow).

### Code Walkthrough

dce.rs: Visit function builds defmap (reg → block, instruction index). Seed live set from terminators and side-effecting instructions. propagate_worklist walks backwards through uses. Sweep removes unmarked instructions.

---

## Copy Propagation (copy_prop.rs)

### What It Does

Records Copy src → dst in a map. Replaces all uses of dst with src.

### Two-Pass Strategy for Phis

**Why two passes?** Phi arguments are Pair(block, value). You need the complete copy map before rewriting phi args, otherwise you might miss transitive copies.

**Algorithm:**
1. First pass: visit all blocks and instructions, build the copy_map
2. Second pass: rewrite phi arguments using the complete map

### Code Walkthrough

copy_prop.rs: Visit instructions, accumulating copy_map. Then iterate over phi nodes and rewrite their inner operands.

---

## Tail Call Optimization (tailcall.rs)

### What It Does

Detects self-recursive tail calls and replaces them with branches, avoiding stack growth.

### Pattern Recognition

A tail call is:
1. A call instruction that is the last instruction in a block
2. The return value of the block is the result of the call
3. The call is to the same function (recursive)

### Transformation

Replace:
```
block:
  r_result = call @func(args)
  ret r_result
```

With:
```
block:
  r_param_0 = copy arg_0
  r_param_1 = copy arg_1
  ...
  br entry_block
```

The parameters are updated with the new arguments, then we branch directly to the entry block, effectively restarting the function with new parameters.

### Code Walkthrough

tailcall.rs lines 38–83: Match blocks with ret-call pattern, extract call args, generate copy instructions for parameters, replace terminator with branch to entry.

---

## SSA Deconstruction (deconstruct.rs) — THE IMPORTANT ONE

This is the most sophisticated pass. It handles the hard problem of eliminating phi nodes without breaking correctness.

### The Conceptual Problem

Phi nodes are conceptually **simultaneous**. The instruction `r3 = phi [r1, block0], [r4, block2]` means "r3 is assigned r1 if we come from block0, and r4 if we come from block2, **at the same time**."

In conventional code, assignments are sequential. When you deconstruct a phi, you must replace it with sequential copies that preserve the simultaneous semantics.

### The Swap Problem

The classic example: two phis that swap values:
```
block1:
  r5 = phi(..., r7)
  r7 = phi(..., r5)
```

If you naively emit:
```
r5 = copy r7    # r7 gets overwritten... but we still need r7!
r7 = copy r5
```

The second copy reads the already-written r7, so the swap fails.

### The Lost-Copy Problem

Less obvious but critical. Consider:
- Block A has two successors: B and C
- B has a phi: `r5 = phi(..., r7_from_A)`
- C has a phi: `r5 = phi(..., r3_from_A)` but the copy does `r5 = r3`, overwriting r5
- When we emit copies at the end of A, if we emit C's copy before B's branch, the copy clobbers r5 before B can read it

**Solution:** Split critical edges. Insert an empty block on edges from blocks with multiple successors to blocks with multiple predecessors. This ensures copies for each edge execute in isolation.

**Note:** Iris avoids this by design. The frontend creates merge blocks proactively (explicit structured control flow), so critical edges don't exist. deconstruct.rs comments on line 208–210 explicitly note this.

### DFS-Based Sequentialization with Grey/Black Coloring

The algorithm in deconstruct.rs uses DFS with colors to handle both the swap problem and the general DAG case.

**Colors:**
- None: not yet visited
- Grey: currently on the DFS stack (being explored)
- Black: fully done, copy already emitted

**Algorithm:**
1. For each destination register in the copy list:
   - Call dfs(dst)
2. dfs(node):
   - Mark node Grey
   - Find all registers that read from node (neighbors where src == node)
   - For each neighbor:
     - If Grey: back edge (cycle). Break by:
       - Allocate tmp
       - Emit: tmp = copy neighbor (save neighbor before it's overwritten)
       - Emit: neighbor = copy tmp (satisfy neighbor's copy)
       - Mark neighbor Black
     - If Black: skip (already done)
     - If None: recurse into neighbor
   - Emit node's own copy (post-order)
   - Mark node Black

**Why it works:**
- Post-order emission ensures all readers of node are done before node's copy
- Grey detection catches cycles early
- Tmp register breaks cycles by saving the value before it's clobbered

### Code Walkthrough

deconstruct.rs lines 66–192: collect_phi_copies extracts (dst, src, typ) triples from phis. sequentialize_copies uses DFS with color tracking. The dfs function handles the recursion and cycle detection.

### How to Explain This in an Interview

"Phi nodes are simultaneous assignments. When deconstructing, I convert them to sequential copies. The challenge is the swap problem — if two phis swap values, naive sequentialization clobbers one before the other reads it.

I use a DFS-based approach: treat the copies as a directed graph. Traverse with three colors: None (unvisited), Grey (on stack), Black (done). During traversal, if I hit a Grey node, I've found a cycle. I break it by allocating a temp register, saving the value before it's overwritten, then using the temp to satisfy the cycle.

Post-order emission ensures all readers of a node are done before the node's own copy. In practice, Iris doesn't hit the lost-copy problem because the frontend creates merge blocks proactively, preventing critical edges. But the algorithm handles it anyway."

---

## Ramsey Structure Recovery (ramsey.rs)

### Based on Ramsey 2022 Paper

The paper "Unified Synthesis of Iterative and Recursive Programs via Modular Unification" (Ramsey et al., 2022) describes a recursive algorithm to convert unstructured CFGs into structured control flow.

### Recursive Algorithm on Dominator Tree

**Input:** A block, dominator tree, predecessors, successors.

**Algorithm:**
1. Compute the region dominated by this block (subtree in dominator tree)
2. Find successors within the region (outs)
3. Determine if there's a back edge (a successor that reaches back to this block through dominance)
4. **If back edge:** This is a loop header
   - Separate outs into body_succs (reach back) and exit_succs (don't reach back)
   - Recursively structure the body
   - Create a Loop node
   - Recursively structure exits and sequence the loop with exits
5. **Else if 2 outs:** If-else
   - Recursively structure both branches
   - Create an If node
6. **Else if 1 out:** Sequence
   - Recursively structure the successor
   - Create a Sequence with this block and the successor
7. **Else (0 outs):** Leaf block
   - Return just the Block node

**Why it works:** The dominator tree ensures the recursion respects dominance. Dominance guarantees that a back edge to the current block is the loop structure. Two successors guarantee an if-else structure. One successor is a chain.

### What "Reducible" Means and Why the Frontend Guarantees It

A CFG is **reducible** if every strongly connected component (loop) has a unique header that dominates all nodes in the loop. In other words, there are no "weird" irreducible control flow structures like crossings or overlapping cycles.

Ramsey structuring works only on reducible CFGs. Iris guarantees reducibility because the **frontend generates CFGs from structured ASTs**. The source language has if/else, while, etc. — no goto. The translation from AST to CFG preserves structure, so the CFG is always reducible.

### Code Walkthrough

ramsey.rs lines 22–105: ramsey_structuring is the main function. It computes the region, finds outs, checks for back edges, then dispatches on the number of outs and presence of back edges.

---

## Dead Block Elimination (dbe.rs)

### Algorithm: Reachability Walk

1. Start from the entry block (virtual_entry)
2. Worklist-based DFS: for each block, add its successors to the worklist
3. Mark all reachable blocks
4. Delete unreachable blocks from the arena

### Why It's Needed

Optimization passes like constant propagation and GVN can make branches unconditional, leaving some branches unreachable. Ramsey structuring expects to traverse only reachable blocks, so dead blocks must be eliminated first.

### Code Walkthrough

dbe.rs: reachable_blocks walks the CFG from entry, marking visited. Then iterate over the arena and remove unmarked blocks.

---

## Register Compaction (reg_compact.rs)

### What It Does

Optimization passes create gaps in register numbering. A function might use r0, r2, r4, r7 (skipping r1, r3, r5, r6). Register compaction renumbers them densely: r0, r1, r2, r3.

### Algorithm

1. Collect all used registers (from params, instruction dests, and operand uses)
2. Sort them (BTreeSet gives sorted order)
3. Build a mapping old → new (r0→0, r2→1, r4→2, ...)
4. Rewrite all register references using the mapping

### Why It Matters

Dense register numbering reduces memory usage and improves cache locality in the codegen phase. It also makes the WebAssembly output more readable.

### Code Walkthrough

reg_compact.rs: collect_operand_regs walks the entire function and gathers used registers into a BTreeSet. Lines 45–49 build the mapping. Lines 57–68 rewrite all registers.

---

## Peephole Optimization (peephole.rs)

### Pattern: LocalSet(x) followed by LocalGet(x)

The only peephole rule currently implemented:

```
local.set $x
local.get $x
```

can be optimized away, leaving the value on the stack (it was already on the stack before the set).

### Why It Exists

Earlier passes (SSA deconstruction, copy propagation, etc.) may generate spill/reload pairs. WebAssembly has a stack, not a register file, so local.set/get are used to interact with locals. If we set then immediately get the same local, we can remove both.

### Algorithm

1. Iterate over instructions with a peekable iterator
2. When you see LocalSet(a), check if the next instruction is LocalGet(a)
3. If yes, consume the LocalGet and skip both instructions
4. Recurse into nested control flow (blocks, loops, if-else)

### Code Walkthrough

peephole.rs: optimize function iterates, and optimize_nested recurses into block bodies.

---

## Architecture Decisions Worth Mentioning

### Block Arena (HashMap-Based, Typed BlockId)

The function.arena is a HashMap<BlockId, BasicBlock>. BlockId is a newtype wrapper around usize, providing type safety.

**Why not a Vec?** A Vec would require dense block IDs. Using a HashMap allows sparse IDs (useful during construction when blocks might be allocated out of order).

**Trade-off:** HashMap lookups are O(1) average but have more overhead than Vec. For typical function sizes, this is fine.

### Visitor Pattern (MirVisitor Trait)

The pass infrastructure uses a visitor pattern: each pass implements MirVisitor and overrides the visit_* methods it cares about. The trait provides walk_* methods that traverse the entire tree.

**Advantage:** Separates traversal from transformation. Easy to write custom passes.

**Code:** Look at dce.rs and gvn.rs to see how different passes override different visit_* methods.

### Fixed-Point Iteration of Passes

Optimization passes run to a fixed point: keep running the suite (const_prop, gvn, licm, copy_prop, dce) until no changes are made.

**Why:** One pass enables another (dce exposes constants for const_prop; const_prop exposes redundancies for gvn; gvn creates copies for copy_prop).

**Trade-off:** Fixed-point is slow if not careful (could iterate many times). But correctness is more important. Early exit heuristics can help (e.g., stop after N iterations or if the last pass changed < 1% of instructions).

### Virtual Entry Block

The function.virtual_entry is a special block that represents "before the function starts." It's used as the entry node for dominator analysis.

**Why:** Dominator analysis typically assumes a single entry. By creating a virtual entry that jumps to the actual entry, we normalize the CFG structure.

### No External Dependencies

The compiler is built in Rust with no external crates (no serde, no petgraph, etc.). Everything is implemented from scratch.

**Advantage:** Full control, easy to audit, no supply chain risk.

**Disadvantage:** Some things (like a HashMap) are slower than specialized libraries, and there's no off-the-shelf algorithms for complex things like dominator computation or SSA construction.

---

## Quick-Fire Interview Q&A

### Q: What's SSA and why use it?

A: SSA is Single Static Assignment — each register is assigned exactly once. This makes data flow explicit and enables many optimizations (GVN, DCE, constant propagation). The trade-off is you need phis and deconstruction.

### Q: How do phi nodes work?

A: Phi nodes join multiple definitions at merge points. `r = phi [r1, block0], [r2, block1]` means r takes r1 if we come from block0, r2 if we come from block1. They're placed at dominance frontiers because that's where definitions meet.

### Q: Why dominance frontiers for phis?

A: A phi is needed at a block if multiple definitions reach that block via different paths. Dominance frontier is exactly the set of blocks where that happens. The algorithm computes the transitive closure via fixed-point iteration.

### Q: What's the hardest optimization to get right?

A: SSA deconstruction. The swap problem is subtle — when you eliminate phis, you might have two registers swapping values, and naive sequentialization clobbers one before the other reads it. The DFS-based approach with temp registers handles it.

### Q: Why WebAssembly?

A: It's a well-specified target with excellent tooling (wasmtime, wasm-opt), runs in browsers and runtimes like Node.js. It forces structured control flow (blocks, loops, if-else), which aligns well with Ramsey structure recovery. Easy to validate and emit readable text.

### Q: What optimizations do you regret not doing?

A: Strength reduction, induction variable elimination, and speculative optimization for faulting instructions. Full SCCP with branch reachability would be nice. And profiling-guided optimization (PGO) to guide inlining and branching heuristics.

### Q: What's the performance bottleneck?

A: Probably dominator computation (O(N²) iterative) or the fixed-point loop if you have many passes. For small functions, it's fine. For 100K+ blocks, you'd need Lengauer-Tarjan and smarter termination heuristics.

### Q: Irreducible control flow?

A: Ramsey structuring only works on reducible CFGs. If the IR had unstructured gotos with overlapping loops (irreducible), the algorithm would fail. In practice, if the frontend generates from a structured language, the CFG is always reducible.

---

## How To Actually Study This (Exercises)

Work through these by hand, on paper, without looking at the code until you've tried.

### Exercise 1: Hand-Trace Dominators

Use the triangular() function from the README:

```
block0:
  r1 = 0
  r2 = 1
  br block1

block1:
  r3 = phi [r1, block0] [r4, block2]
  r5 = phi [r2, block0] [r6, block2]
  r7 = le r5, r0
  br_if r7, block2, block3

block2:
  r4 = add r3, r5
  r6 = add r5, 1
  br block1

block3:
  ret r3
```

**Task:** Draw the CFG on paper (4 blocks, edges marked). Compute dominators by hand using the iterative algorithm. Fill in a table:

| Block | Dominators |
|-------|-----------|
| entry (virtual) | {entry} |
| block0 | ? |
| block1 | ? |
| block2 | ? |
| block3 | ? |

**Answer:** entry dominates all. block0 only entry. block1: entry, block0 (every path goes through block0). block2: entry, block0, block1 (loop body, only reachable from block1). block3: entry, block0, block1.

Then compute the dominator tree (immediate dominator for each):
| Block | idom |
|-------|------|
| block0 | entry |
| block1 | block0 |
| block2 | block1 |
| block3 | block1 |

Then compute dominance frontiers by walking predecessors:
| Block | DF |
|-------|-----|
| block0 | {} |
| block1 | {block1} (block1 is a join; block2 reaches back to block1) |
| block2 | {} |
| block3 | {} |

### Exercise 2: Hand-Trace DCE

Use the dead code example from the README:

```
fn main() -> f64 {
  var x = 5
  var y = 10
  var z = x + y

  var dead1 = 100
  var dead2 = dead1 + 1
  var dead3 = dead2 * 2

  return z
}
```

In MIR form (roughly):

```
block0:
  r1 = copy 5
  r2 = copy 10
  r3 = add r1, r2
  r4 = copy 100
  r5 = add r4, 1
  r6 = mul r5, 2
  ret r3
```

**Task:** Trace DCE by hand.

1. Seed live set: The return value r3 is live.
2. Walk backwards: r3 is defined by the return. Mark r1, r2 (operands of add) as live.
3. r1 is defined by copy 5 (constant, no operands to mark).
4. r2 is defined by copy 10 (constant).
5. Done: live = {r1, r2, r3}.
6. Sweep: r4, r5, r6 are not live. Remove instructions defining them.

**Result:**
```
block0:
  r1 = copy 5
  r2 = copy 10
  r3 = add r1, r2
  ret r3
```

### Exercise 3: THE SWAP PROBLEM

Write down two phis that swap values:

```
block1:
  r5 = phi [r7, block0], ...
  r7 = phi [r5, block0], ...
```

**Task 1:** Try naive sequentialization:
```
r5 = copy r7    # Now r7 is modified... but r7 is still needed!
r7 = copy r5
```

Show that this fails (r7 was overwritten, so the second copy reads garbage).

**Task 2:** Trace the DFS-based fix:

1. dfs(r5):
   - Mark r5 Grey
   - Neighbors of r5 (who read r5): r7 (because r7's copy is from r5)
   - dfs(r7):
     - Mark r7 Grey
     - Neighbors of r7: r5 (because r5's copy is from r7)
     - r5 is Grey: back edge! Allocate tmp = r8
     - Emit: r8 = copy r5 (save r5's current value)
     - Emit: r5 = copy r8 (use saved value for r5's copy)
     - Mark r5 Black
     - Back to r7: post-order, emit r7's copy
     - Emit: r7 = copy r5
     - Mark r7 Black
   - Back to r5: r5 is already Black, skip post-order

**Result:**
```
r8 = copy r5    # Save r5
r5 = copy r8    # Restore r5's value
r7 = copy r5    # Now r7 = original r5, and r5 = original r5
```

(Actually, r8 = copy r5 and r5 = copy r8 is a no-op, but the algorithm is general.)

### Exercise 4: THE LOST-COPY PROBLEM (theory)

**Setup:** Block A has two successors, B and C.
- B has a phi: r5 = phi(..., r7_from_A, ...)
- C has a phi: r5 = phi(..., r3_from_A, ...)

Both phis write to r5. Deconstruction creates two copies:
- One at the end of A on the edge to B: r5 = copy r7
- One at the end of A on the edge to C: r5 = copy r3

**Problem:** If you emit both copies at the end of A:
```
block A:
  ...
  r5 = copy r7    # Edge to B
  r5 = copy r3    # Edge to C — overwrites r5, but B still needs to see r7!
```

When B executes, r5 = r3, not r7. The phi's semantics are violated.

**Solution:** Split critical edges. Insert an empty block on the edge A→C:

```
block A_to_C:
  r5 = copy r3
  br block_C
```

Now each edge has its own copy, executed in isolation.

**Iris specifics:** Iris doesn't hit this because the frontend creates merge blocks proactively. When you have an if-else, the frontend creates:

```
block_if_body: ...
  br block_merge

block_else_body: ...
  br block_merge

block_merge: ...
```

The merge block is a single successor from both branches. So there are no critical edges (edges from a block with 2+ successors to a block with 2+ predecessors).

### Exercise 5: Trace Deconstruction on Paper

Go back to the swap example from Exercise 3. Trace the DFS algorithm in detail:

```
copies = [(r5, r7), (r7, r5)]
colors = {}
result = []

dfs(r5):
  colors[r5] = Grey
  neighbors = [r7]
  dfs(r7):
    colors[r7] = Grey
    neighbors = [r5]
    Check r5: colors[r5] = Grey -> back edge
      tmp = allocate() = r8
      emit r8 = copy r5
      emit r5 = copy r8
      colors[r5] = Black
    Post-order r7: colors[r7] != Black, so emit r7's copy
    emit r7 = copy r5
    colors[r7] = Black
  Back to r5: colors[r5] = Black, skip post-order

result = [r8 = copy r5, r5 = copy r8, r7 = copy r5]
```

Now open deconstruct.rs and map each line of the dfs function to your trace.

### Exercise 6: Explain Each Pass in 30 Seconds

For each of these, time yourself explaining it out loud in 2-3 sentences:

1. **CFG Construction:** Walk block terminators, record edges.
2. **Dominators:** Iterative fixed-point to find blocks that must be visited to reach each block.
3. **Dominance Frontiers:** Walk up from predecessors to find join points.
4. **Phi Insertion:** Place phis at dominance frontiers of multi-defined registers.
5. **SSA Renaming:** Rename registers via dominator tree walk to reflect which definition they refer to.
6. **Constant Propagation:** Forward constants through copies.
7. **GVN:** Hash instructions by (opcode, args); replace duplicates.
8. **LICM:** Hoist loop-invariant instructions to preheader.
9. **DCE:** Mark live registers from terminators and side effects, sweep unmarked instructions.
10. **Copy Propagation:** Replace copy destinations with their sources.
11. **Tail Call Optimization:** Detect recursive tail calls, replace with branches.
12. **Deconstruction:** Sequentialize parallel phi copies using DFS; handle cycles with temps.
13. **Ramsey:** Recursively structure CFG via dominator tree; detect loops (back edges), if-else (2 successors), sequences (1 successor).

If you can't explain one in 30 seconds, simplify your explanation.

### Exercise 7: Data Structure Inventory

For each pass, write down:

(a) **What problem it solves** (e.g., "ensures each register assigned once", "removes unused instructions")
(b) **What data structure it uses** (e.g., "HashMap reg → constant", "BTreeSet of used registers")
(c) **Why it's correct in SSA form** (e.g., "each register has one definition, so use chains are unambiguous")

Example:
- **DCE:** Problem: remove dead instructions. Data structure: HashMap (reg → def location), HashSet (live registers). SSA: each register has one definition, so tracing uses back is unambiguous.

Do this for all 13 passes.

### Exercise 8: Phi Insertion From Scratch

Take this program and convert it to SSA by hand:

```
fn foo(x: f64) -> f64 {
  var a = x
  if (x > 0) {
    a = a + 1
  } else {
    a = a - 1
  }
  return a
}
```

Steps:
1. Draw the CFG (entry, then-block, else-block, merge-block)
2. Identify which blocks define `a` (entry, then-block, else-block → 3 definitions)
3. Compute dominance frontiers of each defining block
4. Place phis at the frontiers (merge-block should get one)
5. Rename: walk the dominator tree with your stack, give each definition a fresh name
6. Write out the final SSA form with phi nodes

Do this without looking at ssa.rs. Then open it and compare your reasoning to the code.

### Exercise 9: GVN By Hand

Trace GVN on this MIR:

```
block0:
  r1 = add r0, 1
  r2 = mul r1, 2
  br block1

block1:
  r3 = add r0, 1      # same as r1!
  r4 = mul r3, 2       # same as r2 (after r3 is replaced)
  r5 = add r3, r4
  ret r5
```

Steps:
1. Start at block0 (dominates block1). Process r1: key = (add, [r0, 1]), not in map, insert (add, [r0, 1]) → r1
2. Process r2: key = (mul, [r1, 2]), insert → r2
3. Move to block1 (dominated by block0, so block0's values are visible)
4. Process r3: key = (add, [r0, 1]) — already in map! Replace r3's instruction with `r3 = copy r1`
5. Process r4: key = (mul, [r3, 2]) — BUT r3's args are now [r0, 1] equivalent... wait, GVN hashes on the operands as-is. Since r3 is still r3 (not yet replaced in the key), the key is (mul, [r3, 2]) which is different from (mul, [r1, 2]).

**This is the subtlety.** GVN as implemented doesn't do copy propagation inline — it replaces the instruction with a Copy but doesn't rewrite subsequent uses until copy prop runs. That's why the passes iterate to a fixed point. After copy prop replaces r3 with r1 everywhere, a second GVN pass would catch r4 too.

Trace this interplay between GVN and copy prop across two iterations.

### Exercise 10: LICM By Hand

Trace LICM on this loop:

```
block0 (entry):
  r1 = copy 10
  br block1

block1 (loop header):
  r2 = phi [r1, block0] [r4, block2]
  r3 = add r1, 5          # loop invariant! r1 and 5 are both loop-external
  r4 = add r2, r3
  r5 = lt r4, 100
  br_if r5, block2, block3

block2 (loop body):
  br block1

block3 (exit):
  ret r4
```

Steps:
1. Find the loop: back edge is block2 → block1 (block1 dominates block2). Body = {block1, block2}.
2. Find definitions: where is each register defined? r1 in block0 (outside loop), r2 in block1 (inside), r3 in block1 (inside), r4 in block1 (inside), r5 in block1 (inside).
3. Find invariants — iterate to fixed point:
   - r1: defined outside → invariant
   - 5: constant → invariant
   - r3 = add r1, 5: both operands invariant → r3 is invariant!
   - r2: phi → NOT invariant (changes each iteration)
   - r4 = add r2, r3: r2 is not invariant → r4 is not invariant
   - r5 = lt r4, 100: r4 not invariant → r5 not invariant
4. Hoist r3 to preheader:

```
block_preheader:
  r3 = add r1, 5
  br block1

block1:
  r2 = phi [r1, block0] [r4, block2]
  r4 = add r2, r3
  ...
```

Now extend this: what if `r3 = div r1, r2` instead? r2 is a phi (not invariant), so it wouldn't be invariant. But what if `r3 = div r1, 0`? It's invariant (both operands are loop-external) but **hoisting a division by zero is unsafe** if the loop body might not execute. This is the safety problem LICM needs to handle.

### Exercise 11: Ramsey Structuring By Hand

Take this CFG and structure it with Ramsey's algorithm:

```
block0 (entry):
  br_if cond, block1, block2

block1:
  br block3

block2:
  br block3

block3:
  br_if cond2, block4, block5

block4:
  br block3          # back edge! block3 dominates block4

block5:
  ret
```

Steps:
1. Compute dominator tree:
   - block0 dominates everything
   - block1: idom = block0
   - block2: idom = block0
   - block3: idom = block0
   - block4: idom = block3
   - block5: idom = block3

2. Call ramsey_structuring(block0):
   - Region dominated by block0 = all blocks
   - Successors in region: block1, block2
   - No back edges from block1 or block2 to block0
   - 2 outs → If-else
   - Then: ramsey_structuring(block1)
     - Region = {block1}. Outs in region = none (block3 not dominated by block1). Leaf.
   - Else: ramsey_structuring(block2)
     - Region = {block2}. Same thing. Leaf.
   - Result: If { cond: block0, then: Block(block1), else: Block(block2) }

Wait — what about block3, block4, block5? They're dominated by block0, not by block1 or block2. So they need to come after the if-else as a sequence.

Actually, let me re-examine. block3's only predecessors are block1, block2, and block4. block0 dominates block3 (all paths from entry go through block0). So block3 is in block0's dominated region.

The algorithm at block0 sees successors block1, block2 in its region. It builds If-else for those. But block3, block4, block5 are also in the region — they're reached through block1/block2.

This is where the algorithm recurses: ramsey_structuring(block1) sees block3 as a successor in block1's dominated region? No — block3 is NOT dominated by block1. So block1 has no outs in its region. It's a leaf.

The key insight: Ramsey's algorithm structures the *dominator subtree*, not the full CFG. Nodes not in a block's dominated region are handled by the parent's continuation.

**This is a good exercise to test your understanding of the algorithm's recursion structure. If you get confused, draw the dominator tree and trace the algorithm call by call.**

### Exercise 12: Full Pipeline Trace

Take the simplest possible program:

```
fn add(a: f64, b: f64) -> f64 {
  return a + b
}
```

Trace it through EVERY stage:
1. Tokens from the lexer
2. AST from the parser
3. HIR after type checking
4. MIR after HIR lowering (what blocks? what instructions?)
5. SSA form (do any phis get inserted? No — single block, single definition of each register)
6. After optimization (const prop does nothing, GVN does nothing, DCE does nothing — it's already minimal)
7. After SSA deconstruction (no phis to deconstruct)
8. After register compaction
9. Ramsey structuring (single block → leaf)
10. WAT output

This exercise verifies you understand the *entire* pipeline, not just individual passes.

### Exercise 13: Break Each Pass

For each optimization pass, construct a minimal input where the pass actually does something non-trivial:

1. **Const prop:** `r1 = copy 5; r2 = add r1, 3` → r1 gets replaced with 5 in the add
2. **GVN:** Two blocks that compute the same expression
3. **LICM:** A loop with an invariant computation
4. **DCE:** A function with unused variables
5. **Copy prop:** A chain of copies `r2 = copy r1; r3 = copy r2; ret r3`
6. **Tail call:** A recursive factorial
7. **Dead block elimination:** An if-else where the condition is constant (after const prop)
8. **Peephole:** A local.set immediately followed by local.get

For each, write the MIR before and after the pass runs. This cements what each pass actually changes.

### Exercise 14: Whiteboard Simulation

Pretend you're at an interview whiteboard. For each of these prompts, explain and draw:

1. "Draw a CFG with a loop. Show me the dominator tree. Where do phis go?"
2. "Walk me through how you'd eliminate this phi node" (draw a phi with three incoming edges, one of which creates a cycle)
3. "This function has a hot loop. What optimizations apply?" (draw a loop with an invariant computation and a dead variable)
4. "How does your compiler go from this MIR to WebAssembly?" (draw Ramsey structuring on a simple if-else + loop)

Practice drawing these on paper or a whiteboard, talking out loud as you draw.

### Exercise 15: The "Why" Chain

For each design decision, ask yourself "why?" three times:

**Example: "Why do you use a HashMap for the block arena?"**
- Why? Because blocks can be allocated and deleted out of order during optimization.
- Why does that matter? A Vec would require dense indices; deleting block 3 out of 10 would leave a gap or require shifting.
- Why not use a different approach? You could use a SlotMap or generational arena for better cache locality, but HashMap is simpler and correct.

Do this for:
1. Why a virtual entry block?
2. Why iterate passes to a fixed point?
3. Why post-order emission in SSA deconstruction?
4. Why does GVN walk the dominator tree instead of the CFG?
5. Why does LICM need a preheader instead of just moving instructions before the loop header?
6. Why does DCE work backwards from outputs instead of forwards from inputs?
7. Why does copy prop need a second pass for phi nodes?

---

## Interview Tips

### How to Present Your Work

Start with the one-sentence pitch. Then talk about the pipeline. If they ask about a specific pass:

1. Explain what problem it solves
2. Explain the key data structure
3. Mention the SSA-specific insight
4. Point to an interesting corner case (swap problem, lost-copy, critical edges, etc.)

### Prepare for Different Angles

- **Theory:** Why dominance frontier for phis? Why fixed-point for loop detection? Why Ramsey's algorithm?
- **Implementation:** How did you debug the swap problem? What was the hardest pass to get right?
- **Scaling:** How would you optimize this for 100K+ block functions?
- **Testing:** How did you validate SSA construction? How do you know deconstruction is correct?

### Be Honest About Limitations

- You don't implement full SCCP (just copy constant prop)
- You don't optimize for irreducible CFGs (rely on frontend)
- You don't handle potentially faulting instructions in LICM
- Dominator computation is O(N²), not optimal
- No PGO, no inlining, no strength reduction

Interviewers respect honesty and understanding of trade-offs.

### Ask Good Questions Back

- "In your codebase, do you optimize for code size, compile time, or runtime speed?"
- "How do you handle irreducible control flow?"
- "Do you do speculative optimization or care only about safe rewrites?"
- "What's your approach to testing correctness of IR transformations?"

---

## Final Notes

The Iris compiler is a solid demonstration of compiler fundamentals: CFG analysis, SSA construction and deconstruction, optimization passes, and structured code generation. The code is clean and well-commented. The biggest complexity is SSA deconstruction (parallel copies, swap problem), which is a real challenge even in production compilers.

Study the code, trace through examples by hand, and be ready to explain not just *what* each pass does but *why* it's correct. SSA form is the key insight that enables many optimizations.

Good luck with your interviews.
