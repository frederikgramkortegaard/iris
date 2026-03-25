# Iris

A compiler for a custom language targeting WebAssembly, built from scratch in Rust with no external dependencies. It implements a full SSA-based optimization pipeline and uses [Ramsey's (2022)](https://dl.acm.org/doi/10.1145/3547621) algorithm to recover structured control flow from reducible CFGs, producing WebAssembly-compatible constructs without heuristic restructuring.

- SSA construction via iterated dominance frontiers and dominator tree renaming, with correct SSA deconstruction via parallel copies
- Optimization passes: constant propagation, GVN, LICM, copy propagation, DCE, tail call optimization, dead block elimination, register compaction, peephole optimization
- [Ramsey (2022)](https://dl.acm.org/doi/10.1145/3547621) structure recovery for CFG -> structured WebAssembly
- Visitor pattern for traversal and transformation of both HIR and MIR

I'm writing up the internals in more depth at [frederikgramkortegaard.github.io/articles](https://frederikgramkortegaard.github.io/articles/).

## Example

```
fn triangular(n: f64) -> f64 {
  var s = 0
  var i = 1
  while (i <= n) {
    s = s + i
    i = i + 1
  }
  return s
}
```

After CFG construction and SSA conversion, `s` and `i` each require a phi node at the loop header - one incoming value from the entry block and one from the loop back edge:

```
fn triangular(1 params: [r0]) -> F64:
block0:
    r1 = 0
    r2 = 1
    br block1

block1:                                      ; loop header
    r3 = phi [r1, block0] [r4, block2]       ; s
    r5 = phi [r2, block0] [r6, block2]       ; i
    r7 = Le I1 [r5, r0]                      ; i <= n
    br_if r7, block2, block3

block2:                                      ; loop body
    r4 = Add F64 [r3, r5]                    ; s = s + i
    r6 = Add F64 [r5, 1]                     ; i = i + 1
    br block1

block3:
    ret r3
```

Ramsey's structure recovery converts the reducible CFG into structured control flow (`if/else`, `loop` blocks) that maps cleanly to WebAssembly:

```wat
(module
  (func $triangular (export "triangular")
    (param $r0 f64)
    (result f64)
    (local $r1 f64)
    (local $r2 f64)
    (local $r3 i32)
    (local $r4 f64)
    (local $r5 f64)
    f64.const 0
    local.set $r1
    f64.const 1
    local.set $r2
    block
      loop
        local.get $r2
        local.get $r0
        f64.le
        i32.eqz
        br_if 1
        local.get $r1
        local.get $r2
        f64.add
        local.set $r4
        local.get $r2
        f64.const 1
        f64.add
        local.set $r5
        local.get $r4
        local.set $r1
        local.get $r5
        local.set $r2
        br 0
      end
    end
    local.get $r1
    return
  )
)
```

## Optimization effect

DCE combined with constant propagation eliminates all values not reaching the return. After constant propagation and dead code removal, the function reduces to a single addition.

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

```wat
(func $main (export "main")
  (result f64)
  f64.const 5
  f64.const 10
  f64.add
  return
)
```

## Pipeline

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
  |     Constant Propagation
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

## Optimization passes

All passes operate on the MIR in SSA form and run to a fixed point in an iterative pipeline, since each pass exposes new opportunities for the next.

**Constant Propagation** - tracks constant values through copy instructions in the SSA def-use graph, replacing register uses with their constant values where known.

**GVN** (Global Value Numbering) - assigns value numbers across basic block boundaries. Instructions computing the same value get replaced with the dominating definition.

**LICM** (Loop Invariant Code Motion) - identifies instructions whose operands are all loop-external and hoists them to the loop preheader, using natural loop detection via back edges identified from the dominator tree.

**Copy Propagation** - walks def-use chains and replaces copies with their source operand, cleaning up chains that accumulate from earlier passes.

**DCE** (Dead Code Elimination) - works backwards from live outputs. In SSA form each value has exactly one definition, so use chains are explicit and dead instructions are unambiguous.

**Tail Call Optimization** - detects tail-recursive calls in the MIR and rewrites them into direct branches before SSA deconstruction, eliminating stack growth from recursive calls.

**Dead Block Elimination** - removes basic blocks that become unreachable once the CFG is finalized.

SSA is deconstructed back to conventional form using parallel copies to correctly handle lost-copy and swap cases at phi node boundaries.

## Building

```bash
cargo build --release
```

## Usage

```bash
# Compile to WAT (prints to stdout)
cargo run -- examples/triangular.iris

# Compile to WAT file
cargo run -- examples/triangular.iris -o output.wat
```

### Running with wasmtime

```bash
brew install wasmtime

cargo run -- examples/sum.iris -o sum.wat
wasmtime run --invoke sum_range sum.wat 1.0 10.0
# => 55

wasmtime run --invoke factorial examples/factorial.wat 5.0
# => 120
```

## Project structure

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
|       |-- const_prop.rs            # Constant propagation
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
