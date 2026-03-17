# Iris

Optimizing compiler targeting WebAssembly, written in Rust. Features a multi-stage IR pipeline, SSA-based optimizations, and control flow structure recovery.

## Architecture

```
Source (.iris)
  |
  +-- Lexer -> Parser
  |
  v
 HIR (High-level IR)
  |-- Type Checking
  |-- AST Simplification
  |
  v
 MIR (Mid-level IR)
  |-- CFG Construction
  |-- Dominator Trees & Frontiers
  |-- SSA Construction (phi insertion + renaming)
  |-- Optimization (iterated):
  |     Constant Propagation (SCCP)
  |     Loop Invariant Code Motion
  |     Global Value Numbering
  |     Copy Propagation
  |     Dead Code Elimination
  |-- Tail Call Optimization
  |-- SSA Deconstruction
  |-- Register Compaction
  |-- Dead Block Elimination
  |
  v
 WAT (WebAssembly Text)
  |-- Ramsey Structure Recovery (CFG -> structured control flow)
  |-- MIR -> WAT IR Lowering
  |-- Peephole Optimization
  |-- Text Emission
  |
  v
 .wat output
```

## Key Features

### SSA Construction
- **Phi Node Insertion** via iterated dominance frontiers
- **Variable Renaming** using the dominator tree walk algorithm
- **SSA Deconstruction** back to conventional form with parallel copies

### Control Flow Analysis
- **Predecessor/Successor** graph construction
- **Dominator Set** computation using iterative dataflow
- **Dominator Tree** construction (immediate dominators)
- **Dominance Frontier** calculation for phi placement

### Optimization Passes
- **Constant Propagation (SCCP)** - tracks constant values through SSA form
- **Loop Invariant Code Motion** - hoists loop-invariant computations
- **Global Value Numbering (GVN)** - eliminates redundant computations
- **Copy Propagation** - eliminates redundant copies
- **Dead Code Elimination (DCE)** - removes unused instructions
- **Dead Block Elimination** - removes unreachable blocks
- **Tail Call Optimization** - rewrites tail-recursive calls into loops

### WebAssembly Backend
- **Ramsey Structure Recovery** - recovers structured control flow (if/else, loops) from the CFG using the dominator tree
- **WAT IR** - typed intermediate representation for WebAssembly instructions
- **Peephole Optimization** - eliminates redundant local.set/local.get pairs
- **Register Compaction** - remaps sparse register numbers to contiguous locals

### Visitor Pattern
Both HIR and MIR implement a visitor pattern for traversing and transforming the IR:

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
# Compile to WAT (prints to stdout)
cargo run -- examples/factorial.iris

# Compile to WAT file
cargo run -- examples/factorial.iris -o output.wat

# Specify target (default: wasm)
cargo run -- examples/factorial.iris -t wasm -o output.wat
```

### Running with wasmtime

```bash
# Install wasmtime
brew install wasmtime

# Compile and run
cargo run -- examples/sum.iris -o sum.wat
wasmtime run --invoke sum_range sum.wat 1.0 10.0
# => 55

wasmtime run --invoke factorial examples/factorial.wat 5.0
# => 120
```

## Example

```
fn factorial(n: f64) -> f64 {
    if (n <= 1) {
        return 1
    } else {
        return n * factorial(n - 1)
    }
}
```

Optimized SSA form:
```
fn factorial(1 params: [r0]) -> F64:
block0:
    r5 = Le I1 [r0, 1]
    br_if r5, block2, block3
block2:
    ret 1
block3:
    r6 = Sub F64 [r0, 1]
    r7 = Call F64 [@factorial, r6]
    r8 = Mul F64 [r0, r7]
    ret r8
```

Compiled WAT output:
```wat
(module
  (func $factorial (export "factorial")
    (param $r0 f64)
    (result f64)
    (local $r1 i32)
    (local $r2 f64)
    (local $r3 f64)
    (local $r4 f64)
    local.get $r0
    f64.const 1
    f64.le
    if
      f64.const 1
      return
    else
      local.get $r0
      f64.const 1
      f64.sub
      call $factorial
      local.set $r3
      local.get $r0
      local.get $r3
      f64.mul
      return
    end
    unreachable
  )
)
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
|       |-- simplify.rs
|       +-- lowering.rs              # HIR -> MIR
|
|-- mir/
|   |-- cfg.rs                       # CFG, dominators, dominance frontiers
|   |-- visitor.rs
|   +-- passes/
|       |-- ssa.rs                   # Phi insertion, variable renaming
|       |-- deconstruct.rs           # SSA deconstruction
|       |-- const_prop.rs            # Constant propagation (SCCP)
|       |-- gvn.rs                   # Global value numbering
|       |-- copy_prop.rs             # Copy propagation
|       |-- dce.rs                   # Dead code elimination
|       |-- dbe.rs                   # Dead block elimination
|       |-- loops.rs                 # Loop analysis & LICM
|       |-- tailcall.rs              # Tail call optimization
|       +-- reg_compact.rs           # Register compaction
|
+-- codegen/
    +-- wasm/
        |-- ramsey.rs                # Structure recovery from CFG
        |-- types.rs                 # WAT IR types
        |-- lower.rs                 # MIR -> WAT IR
        |-- peephole.rs              # WAT peephole optimization
        +-- emit.rs                  # WAT IR -> text output
```
