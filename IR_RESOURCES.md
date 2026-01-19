# Learning About IR Design

## Books (Best Starting Point)

**"Engineering a Compiler" (2nd ed) by Cooper & Torczon**
- Chapter 5: Intermediate Representations
- Chapter 8-10: Optimization techniques
- Best book for understanding IR abstraction levels and design choices
- Explains three-address code, SSA, and CFG construction clearly

**"Modern Compiler Implementation" by Appel**
- Chapter 7: Intermediate Representation Trees
- Chapter 8: Basic Blocks and Traces
- More academic but excellent for understanding IR tradeoffs

**"SSA Book" (free online)**
- https://pfalcon.github.io/ssabook/latest/
- Comprehensive resource on SSA form
- Explains phi nodes, dominance, optimizations

## Online Resources

**LLVM Documentation**
- https://llvm.org/docs/LangRef.html (LLVM IR reference)
- https://llvm.org/docs/tutorial/ (Kaleidoscope tutorial - builds a compiler with LLVM IR)
- Shows a real-world low-level IR design

**Cranelift IR**
- https://github.com/bytecodealliance/wasmtime/tree/main/cranelift
- https://cranelift.readthedocs.io/en/latest/ir.html
- Simpler than LLVM, good for studying
- Used by Wasmtime

**Blog Posts**
- "A Tourist's Guide to the LLVM Source Code" - https://blog.regehr.org/archives/1453
- Chris Lattner's blog posts on LLVM design
- Russ Cox's articles on Go compiler IR

## What is Three-Address Code?

**Simple explanation:**
Each instruction has at most 3 operands:
```
x = y op z
```

**Examples:**
```
// High-level: a = b + c * d
t1 = c * d
t2 = b + t1
a = t2

// With types:
%1 = mul i32 %c, %d
%2 = add i32 %b, %1
store i32 %2, ptr %a
```

**Why three-address?**
- Maps well to most CPU instruction sets
- Easy to optimize
- Simple to convert to SSA form
- Clear data flow

## IR Abstraction Levels

**High-level IR** (close to source)
- Still has language constructs (loops, if-else)
- Good for: early optimizations, error reporting
- Example: Go's AST-based IR

**Mid-level IR** (abstract machine)
- Basic blocks, branches, no structured control flow
- Three-address code or similar
- Good for: most optimizations, SSA form
- Example: LLVM IR, Cranelift IR

**Low-level IR** (close to machine)
- Virtual registers, specific instructions
- Good for: register allocation, instruction selection
- Example: Machine IR in LLVM

## Recommended Learning Path

1. **Start with Cooper & Torczon Chapter 5**
   - Understand IR design tradeoffs
   - Learn about different IR forms

2. **Study LLVM IR basics**
   - Read the language reference
   - Look at simple compiled examples: `clang -S -emit-llvm file.c`

3. **Build a simple three-address IR**
   - Start with basic arithmetic
   - Add branches and basic blocks
   - Don't worry about SSA initially

4. **Study SSA form**
   - Read SSA book chapters 1-3
   - Understand phi nodes
   - Learn dominance and dominance frontier

5. **Look at real compilers**
   - LLVM opt passes: https://llvm.org/docs/Passes.html
   - Cranelift optimizations
   - Read the source code

## Specific Papers (Optional but Valuable)

**"Efficiently Computing Static Single Assignment Form and the Control Dependence Graph"**
- Cytron et al., 1991
- The original SSA paper
- Available: https://www.cs.utexas.edu/~pingali/CS380C/2010/papers/ssaCytron.pdf

**"Simple and Efficient Construction of Static Single Assignment Form"**
- Braun et al., 2013
- Simpler algorithm for SSA construction
- Available: https://pp.info.uni-karlsruhe.de/uploads/publikationen/braun13cc.pdf

## Practical Exercise

After reading, try:
1. Design a simple three-address IR for Iris
2. Lower one function from AST to IR
3. Print it out and verify correctness
4. Add basic block boundaries
5. Build a CFG from it

Want me to help you start designing the IR once you've read some of these?
