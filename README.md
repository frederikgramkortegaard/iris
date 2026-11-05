# Iris

![CI](https://github.com/frederikgramkortegaard/iris/actions/workflows/ci.yml/badge.svg)

Iris is a compiler for a custom language targeting WebAssembly, written from scratch in Rust with no external dependencies. The mid-end implements a full SSA-based optimization pipeline, and the back-end uses [Ramsey's (2022)](https://dl.acm.org/doi/10.1145/3547621) algorithm to recover structured control flow from the CFG. We chose this algorithm over heuristic restructuring as it is correct for any reducible CFG by construction, rather than for the set of shapes we happened to anticipate.

- SSA construction via iterated dominance frontiers and dominator tree renaming, with correct SSA deconstruction via parallel copies
- Optimization passes: constant propagation, CSE, LICM, copy propagation, DCE, function inlining, tail call optimization, dead block elimination, register compaction, peephole optimization
- SCEV (Scalar Evolution) / Induction Variable Analysis, Trip Count Computation
- [Ramsey (2022)](https://dl.acm.org/doi/10.1145/3547621) structure recovery for CFG -> structured WebAssembly
- Visitor pattern for traversal and transformation of both HIR and MIR

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

After CFG construction and SSA conversion, `s` and `i` each require a phi node at the loop header, with one incoming value from the entry block and one from the loop back edge:

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

The back-end then recovers structured control flow from the CFG. Here it is important to note that WebAssembly has no arbitrary jumps, a `br` instruction can only target a construct it is nested inside of, and the placement of `block` and `loop` constructs is therefore what encodes the edges of the CFG:

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

Constant propagation and DCE together eliminate every value which does not reach the return. In the following example, this reduces the function to a single addition.

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
  |     DFG (Data Flow Graph)
  |     Loop Detection
  |     Induction Variable Detection
  |     SCEV / Trip Count Computation
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

All passes operate on the MIR in SSA form. Each pass exposes new
opportunities for the next, and we therefore simply run them in an
iterative pipeline until a fixed point is reached.

- **Constant propagation**
- **CSE** -- dominator-scoped, keyed on register names, so not full value
  numbering. Calls and copies are left alone.
- **LICM** -- hoists loop-invariant instructions to a preheader, natural
  loops found via back edges.
- **Copy propagation**
- **DCE** -- backwards from live outputs.
- **Function inlining** -- bottom-up over the callgraph. Callees containing
  a loop are never inlined, as the loop already amortizes the call
  overhead, and inlining mandel's cell() into the pixel loop made it 12%
  slower.
- **Tail call optimization** -- rewrites tail recursion into branches, such
  that recursion depth is no longer bound by the stack.
- **Dead block elimination**
- **IV analysis / SCEV** -- induction variables from header phis, trip
  counts from exit conditions.

SSA is deconstructed back to conventional form with parallel copies
(lost-copy and swap cases).

## Optimization in numbers

Wasm instructions emitted per example, with and without the optimization
pipeline (`--no-opt`):

| example         | no-opt | optimized |
|-----------------|-------:|----------:|
| const_fold      |     17 |         3 |
| dce             |     27 |         3 |
| copy_prop       |     24 |         9 |
| gvn             |     60 |        31 |
| join_points     |    144 |       108 |
| triangular      |     41 |        32 |
| ssa_deconstruct |    285 |       221 |
| tailcall        |     50 |        64 |

It should be noted that tailcall grows, as tail call optimization trades a
few parameter copies for eliminating the calls entirely, a win which
instead shows up at run time (see below).

## Benchmarks

`bench/run.sh` times the compute-heavy programs in `bench/` compiled with
and without the optimization pipeline, using hyperfine with 2 warmups and
10 runs. Each program runs under two engine configurations: wasmtime with
its defaults, and `wasmtime -O opt-level=0`, which disables cranelift's own
optimizer. We added the second configuration as we noticed that the JIT
otherwise re-derives most of what the mid-end has already done, making it
impossible to tell which improvements are actually ours. Both variants are
checked for identical results before anything is timed.

| benchmark                  | engine        | no-opt   | opt      |
|----------------------------|---------------|---------:|---------:|
| fib(40), call-heavy        | jit           |   541 ms |   527 ms |
| fib(40)                    | jit opt off   |   526 ms |   534 ms |
| mandel(1200^2), float-heavy| jit           |   393 ms |   390 ms |
| mandel(1200^2)             | jit opt off   |   390 ms |   389 ms |
| nested(3e8 iters), licm    | jit           |   407 ms |   407 ms |
| nested(3e8 iters)          | jit opt off   |   408 ms |   398 ms |
| sum_tail(5e6), deep recursion | jit        |  trap: stack exhausted | 12.5e12 (correct) |

With cranelift's optimizer enabled the two columns are within noise of each
other on every benchmark. This is the expected outcome for a mid-end
running in front of a modern JIT, and we consider the honest wins to be
elsewhere: the instruction counts in the previous section, and the fact
that `sum_tail(5000000)` exhausts the call stack without tail call
optimization and simply runs with it. With the optimizer disabled, LICM
accounts for a measurable difference on the workload built for it
(408 ms to 398 ms on nested).

It should also be noted that the benchmarks changed one of our passes.
The inlining pass initially inlined every non-recursive call, and mandel
became 12% slower as a result, since inlining cell() into the pixel loop
increased register pressure where it matters most. The call overhead it
removed was already amortized by the loop inside cell() itself. As a
consequence of this, callees containing a loop are never inlined.

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

# Compile with optimizations disabled (for comparing output)
cargo run -- examples/triangular.iris --no-opt
```

### Running with wasmtime

```bash
brew install wasmtime

cargo run -- examples/sum.iris -o sum.wat
wasmtime run --invoke sum_range sum.wat 1.0 10.0
# => 55

cargo run -- examples/factorial.iris -o factorial.wat
wasmtime run --invoke factorial factorial.wat 5.0
# => 120
```

## Testing

```bash
cargo test
```

Unit tests cover the lexer, parser and typechecker. The end-to-end suite
(`tests/e2e.rs`) compiles every file in `examples/` with the real binary,
runs the resulting wasm with wasmtime, and checks each exported function
against the `# EXPECTED:` annotations in the source itself:

```
# EXPECTED: factorial(5) == 120
# EXPECTED: simple() traps
```

Every example is further compiled twice and the two outputs compared, such
that the compiler is known to stay deterministic. The suite runs as part of
`cargo test` when wasmtime is on PATH, and in CI, where the generated
`.wat` for all examples is uploaded as an artifact on every run.

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
|   |-- visitor.rs
|   |-- analysis/
|   |   |-- cfg.rs                   # CFG, dominators, dominance frontiers
|   |   |-- dfg.rs                   # Data flow graph, def-use chains
|   |   |-- loops.rs                 # Loop detection, natural loops
|   |   |-- callgraph.rs             # Callgraph construction
|   |   |-- recursion.rs             # Recursive-function detection
|   |   +-- iv.rs                    # Induction variable detection
|   +-- passes/
|       |-- inlining.rs              # Function inlining
|       |-- ssa.rs                   # Phi insertion, variable renaming
|       |-- deconstruct.rs           # SSA deconstruction
|       |-- const_prop.rs            # Constant propagation
|       |-- cse.rs                   # Common subexpression elimination
|       |-- copy_prop.rs             # Copy propagation
|       |-- dce.rs                   # Dead code elimination
|       |-- dbe.rs                   # Dead block elimination
|       |-- loops.rs                 # LICM (loop invariant code motion)
|       |-- scev.rs                  # SCEV, trip count computation
|       |-- tailcall.rs              # Tail call optimization
|       +-- reg_compact.rs           # Register compaction
|
+-- codegen/
    +-- wasm/
        |-- types.rs                 # WAT IR types
        |-- lower.rs                 # Structure recovery + MIR -> WAT IR
        |-- peephole.rs              # WAT peephole optimization
        +-- emit.rs                  # WAT IR -> text output
```
