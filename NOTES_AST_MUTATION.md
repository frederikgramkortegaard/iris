# AST Mutation in Passes

## Can You Mutate the AST?

Yes! Since visitor methods take `&mut` references, you can mutate nodes in place, including replacing entire nodes.

## Example: Constant Folding

```rust
pub struct ConstantFoldingPass {
    diagnostics: DiagnosticCollector,
}

impl Visitor for ConstantFoldingPass {
    type Output = ();

    fn visit_expression(&mut self, expression: &mut Expression) -> Self::Output {
        // First, recursively fold children
        self.walk_expression(expression);

        // Then try to fold this expression
        match expression {
            Expression::BinaryOp { left, op, right } => {
                // Check if both operands are now constants
                if let (Expression::Number(a), Expression::Number(b)) = (&**left, &**right) {
                    let result = match op.tag {
                        TokenType::Plus => a + b,
                        TokenType::Minus => a - b,
                        TokenType::Star => a * b,
                        TokenType::Slash => {
                            if *b == 0.0 {
                                // Can't fold division by zero
                                return;
                            }
                            a / b
                        }
                        _ => return, // Not a constant-foldable operation
                    };

                    // Replace the entire BinaryOp with the computed Number
                    *expression = Expression::Number(result);
                }
            }

            Expression::UnaryOp { left, op } => {
                if let Expression::Number(n) = &**left {
                    let result = match op.tag {
                        TokenType::Minus => -n,
                        TokenType::Plus => *n,
                        _ => return,
                    };
                    *expression = Expression::Number(result);
                }
            }

            _ => {}
        }
    }
}
```

## How It Works

**Before folding:**
```
BinaryOp {
    left: Number(2),
    op: Plus,
    right: BinaryOp {
        left: Number(3),
        right: Number(4),
        op: Star
    }
}
```

**After folding:**
```
Number(14)  // 2 + (3 * 4)
```

## The Key Pattern

1. **Recursively visit children first** (bottom-up)
   ```rust
   self.walk_expression(expression);
   ```

2. **Then try to fold this node**
   ```rust
   match expression {
       // Check if it's foldable
       // If yes: *expression = simplified_version
   }
   ```

3. **Bottom-up is important!**
   - Fold `3 * 4` into `12` first
   - Then fold `2 + 12` into `14`

## Why This Pattern?

**You can't replace in place while matching:**
```rust
// This doesn't work:
match expression {
    Expression::BinaryOp { left, op, right } => {
        // Now expression is borrowed as BinaryOp
        // Can't reassign *expression here because fields are borrowed!
    }
}
```

**Solution: Match, compute, then replace:**
```rust
match expression {
    Expression::BinaryOp { left, op, right } => {
        // Extract values without keeping borrows
        if let (Expression::Number(a), Expression::Number(b)) = (&**left, &**right) {
            let result = compute(*a, *b, op);
            // Now the match is done, we can replace
            *expression = Expression::Number(result);
        }
    }
}
```

## When to Mutate AST vs Use IR

**Mutate AST for:**
- Simple optimizations (constant folding, dead code elimination)
- High-level transformations
- Before lowering to IR

**Use IR for:**
- Complex optimizations (CSE, LICM, GVN)
- Anything involving SSA form
- Data-flow analysis
- Most "real" optimizations

**Why IR is better for most opts:**
- SSA form makes data flow explicit
- CFG makes control flow explicit
- Three-address code is simpler to pattern match
- Easier to analyze and transform

## Example Pass Order

```
AST
  → Typechecking (read-only)
  → Simple constant folding (mutate AST)
  → Dead code elimination (mutate AST)
  ↓
Lower to IR
  ↓
IR (SSA, CFG)
  → Constant propagation (mutate IR)
  → DCE, CSE, LICM, etc (mutate IR)
  ↓
Code generation
```

## Should Typechecker Mutate?

**NO!** Keep it pure analysis:
- Easier to debug
- Easier to test
- Easier to understand
- Separation of concerns

If you want to fold constants, make it a separate pass that runs after typechecking.
