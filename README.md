# Iris

Optimizing compiler in Rust with a multi-stage IR pipeline (HIR -> MIR -> LIR), CFG Analysis, SSA construction, Const prop, DCE, Register Allocation, etc.

## Architecture

```
Source -> Frontend -> HIR -> MIR -> LIR -> Target
                      |      |
                      |      +-- Analysis & Optimization
                      |          - CFG Construction
                      |          - Dominator Trees
                      |          - Dominance Frontiers
                      |          - SSA Conversion
                      |          - Constant Propagation (SCCP)
                      |          - Constant Folding
                      |          - Dead Code Elimination
                      |
                      +-- Passes
                          - Type Checking
                          - AST Simplification
                          - Lowering to MIR
```

## Key Features

### SSA Construction
- **SSA (Static Single Assigment)** Form conversion
- **Phi Node Insertion** via iterated dominance frontiers
- **Variable Renaming** using the standard dominator tree walk algorithm

### Control Flow Analysis
- **Predecessor/Successor** graph construction
- **Dominator Set** computation using iterative dataflow
- **Dominator Tree** construction (immediate dominators)
- **Dominance Frontier** calculation for phi placement

### Optimization Passes
- **Constant Propagation (SCCP)** - tracks constant values through SSA form
- **Constant Folding** - evaluates constant expressions at compile time
- **Copy Propagation** - eliminates redundant copies

### Visitor Pattern
Both HIR, MIR and LIR implement a visitor pattern for traversing and transforming the IR:

```rust
impl MirVisitor for MyPass {
    fn visit_instruction(&mut self, inst: &mut Instruction) -> Self::Output {
        // Transform or analyze instructions
    }
}
```

## Building

```bash
cargo build --release
```

## Usage

```bash
cargo run -- path/to/file.iris
```

## Example

```
fn fib_step(a: f64, b: f64, n: f64) -> f64 {
    var result: f64 = 0
    if (n < 1) {
        result = a
    } else {
        result = a + b
    }
    return result
}
```

Compiles to MIR with SSA form:
```
fn fib_step(3 params) -> F64:
block0:
    r20 = Copy F64 [0]
    r21 = Lt I1 [r16, 1]
    br_if r21, block1, block2
block1:
    r25 = Copy F64 [r14]
    br block3
block2:
    r22 = Add F64 [r14, r15]
    r23 = Copy F64 [r22]
    br block3
block3:
    r24 = Phi Void [[block2, r23], [block1, r25]]
    ret r24
```

## Project Structure

```
src/
|-- frontend/
|   |-- lexer.rs
|   +-- parser.rs
|
|-- hir/
|   |-- visitor.rs
|   +-- passes/
|       |-- typechecking.rs
|       |-- ast_simplification.rs
|       +-- lowering.rs            # HIR -> MIR
|
|-- mir/
|   |-- cfg.rs                     # CFG, dominators, dominance frontiers
|   |-- visitor.rs
|   +-- passes/
|       |-- ssa.rs                 # Phi insertion, variable renaming
|       +-- const_prop.rs          # Constant/copy propagation
|
+-- lir/
    +-- ...
```

