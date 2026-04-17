# Venom Optimization Passes

## Dependency Analysis

| Pass | What it does | Also known as |
|------|--------------|---------------|
| **DDA** (Data Dependency Analysis) | Tracks which instructions produce values used by others | Use-def chains, reaching definitions, def-use chains |
| **EDA** (Effect Dependency Analysis) | Tracks ordering constraints from side effects (memory, storage) | Alias analysis, memory dependence analysis, may-alias analysis |
| **DFG** (Data Flow Graph) | Graph of value flow between instructions | SSA graph, value dependence graph |
| **CFG** (Control Flow Graph) | Graph of basic blocks and jumps | Flow graph |
| **Liveness Analysis** | Tracks which values are still needed at each point | Live variable analysis, register pressure analysis |
| **Dominance Analysis** | Block A dominates B if all paths to B go through A | Dominator tree |

## Stack & Scheduling

| Pass | What it does | Also known as |
|------|--------------|---------------|
| **DFTPass** | Reorders instructions to minimize stack depth | Instruction scheduling, list scheduling, depth-first scheduling |
| **StackOrderAnalysis** | Figures out optimal stack layout at block boundaries | Register allocation (in register machines), stack layout |
| **Stack Reordering** | Emits SWAP/DUP to arrange operands for each instruction | Operand permutation, stack manipulation |
| **Stack Spilling** | Moves values to memory when stack > 16 deep | Register spilling, spill code generation |
| **SingleUseExpansion** | Inlines single-use values to avoid stack slots | Copy propagation, inline expansion |
| **Mem2Var** | Promotes memory locations to virtual registers | Mem2reg, SSA construction, register promotion |
| **PhiElimination** | Converts SSA φ-nodes to copies at block edges | SSA destruction, phi lowering, copy insertion |

## Data Flow & Constants

| Pass | What it does | Also known as |
|------|--------------|---------------|
| **SCCP** | Sparse conditional constant propagation | Constant propagation, constant folding, value numbering |
| **CSE** | Common subexpression elimination | Value numbering, redundancy elimination |
| **AlgebraicOptimization** | `x * 1 → x`, `x + 0 → x`, etc. | Strength reduction, identity elimination, peephole optimization |
| **AffineFolding** | Simplifies affine expressions | Linear expression simplification |

## Memory

| Pass | What it does | Also known as |
|------|--------------|---------------|
| **DeadStoreElimination** | Removes writes that are never read | DSE, useless store elimination |
| **LoadElimination** | Removes redundant loads | Load-store optimization, redundant load elimination, GVN |
| **MemMerging** | Combines adjacent memory operations | Memory coalescing, store merging |
| **MemoryCopyElision** | Eliminates unnecessary memory copies | Copy elision, copy propagation |

## Control Flow

| Pass | What it does | Also known as |
|------|--------------|---------------|
| **BranchOptimization** | Simplifies conditional jumps | Jump threading, branch folding |
| **SimplifyCFG** | Merges/removes unnecessary blocks | CFG simplification, block merging, jump threading |
| **TailMerge** | Deduplicates identical block endings | Tail duplication (inverse), cross-jumping |
| **AssertCombiner** | Combines multiple asserts | Guard combining |
| **AssertElimination** | Removes provably-true asserts | Redundant check elimination |

## Functions & Loops

| Pass | What it does | Also known as |
|------|--------------|---------------|
| **FunctionInliner** | Inlines small internal functions | Inlining, procedure integration |
| **LICM** | Loop invariant code motion | Code hoisting, loop-invariant hoisting |
| **LoopAnalysis** | Detects natural loops via back edges | Loop detection, loop nesting analysis |

## Effects System

| Effect | What it tracks | EVM opcodes |
|--------|----------------|-------------|
| **MEMORY** | Memory reads/writes | mload, mstore, mstore8, mcopy |
| **STORAGE** | Persistent storage | sload, sstore |
| **TRANSIENT** | Transient storage | tload, tstore |
| **BALANCE** | Balance queries | balance, selfbalance |
| **EXTCODE** | External code inspection | extcodesize, extcodecopy, extcodehash |
| **LOG** | Event logging | log0, log1, log2, log3, log4 |
| **MSIZE** | Memory size observation | msize |

## Not Yet Implemented

| Pass | What it would do | Also known as |
|------|------------------|---------------|
| **Scalar Evolution** | Analyze how loop variables change | Induction variable analysis, SCEV |
| **Loop Unrolling** | Duplicate loop body to reduce overhead | Loop expansion |
| **Code Sinking** | Move code down into branches where needed | Partial dead code elimination |
| **Expression Reassociation** | Reorder `(a + b) + c` for optimization | Reassociation, tree balancing |
