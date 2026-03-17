pub mod emit;
pub mod lower;
pub mod peephole;
pub mod ramsey;
pub mod types;
use crate::mir::BlockId;

// Wasm structures, this will be the output of Ramsey's structuring algorithm, and will be used to generate WAT code.
#[derive(Debug)]
pub enum StructuredNode {
    Sequence(Vec<StructuredNode>),
    If {
        cond: BlockId,
        then_branch: Box<StructuredNode>,
        else_branch: Box<StructuredNode>,
    },
    Loop {
        header: BlockId,
        body: Box<StructuredNode>,
    },
    Block(BlockId), // simple straight-line block
}
