# iris

This project has reached a good stopping point. It served as a learning exercise and showcase for implementing a compiler from scratch in Rust, covering:

- Lexing and parsing to AST
- HIR with type checking
- MIR in SSA form with optimization passes (constant propagation, copy propagation, GVN, LICM, DCE, tail call optimization)
- CFG analysis (dominators, dominator trees, dominator frontiers)
- WASM codegen with Ramsey-style CFG structuring

It's not production-ready, but it demonstrates the core concepts of a modern optimizing compiler pipeline. The codebase is reasonably clean and tested where it matters.
