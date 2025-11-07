pub mod passes;
pub mod visitor;

#[derive(Debug)]
pub enum Opcode {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Copy,

    Call,

    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirType {
    F8,
    F16,
    F32,
    F64,

    I1,
    I8,
    I16,
    I32,
    I64,

    Void,
}

pub type Reg = usize;

/// Operand can be either a register or an immediate value
#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    Reg(Reg),
    ImmI64(i64),
    ImmF64(f64),
    ImmBool(bool),
    Label(String),
}

impl Operand {
    /// Create a register operand
    pub fn reg(r: Reg) -> Self {
        Operand::Reg(r)
    }

    /// Create an integer immediate operand
    pub fn imm_i64(val: i64) -> Self {
        Operand::ImmI64(val)
    }

    /// Create a float immediate operand
    pub fn imm_f64(val: f64) -> Self {
        Operand::ImmF64(val)
    }

    /// Create a boolean immediate operand
    pub fn imm_bool(val: bool) -> Self {
        Operand::ImmBool(val)
    }
}

/// Type-safe block identifier (index into BlockArena)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(usize);

impl BlockId {
    pub fn new(id: usize) -> Self {
        BlockId(id)
    }

    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Debug)]
pub struct Instruction {
    pub dest: Reg,
    pub op: Opcode,
    pub typ: MirType,
    pub args: Vec<Operand>,
}

#[derive(Debug)]
pub struct BasicBlock {
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
}

#[derive(Debug)]
pub enum Terminator {
    Br {
        target: BlockId,
    },
    BrIf {
        cond: Operand,
        then_bb: BlockId,
        else_bb: BlockId,
    },
    Ret {
        value: Option<Operand>,
    },
    Unreachable,
}

/// Arena for allocating basic blocks
#[derive(Debug)]
pub struct BlockArena {
    blocks: Vec<BasicBlock>,
}

impl BlockArena {
    pub fn new() -> Self {
        BlockArena { blocks: Vec::new() }
    }

    /// Allocate a new block and return its ID
    pub fn alloc(&mut self, block: BasicBlock) -> BlockId {
        let id = BlockId(self.blocks.len());
        self.blocks.push(block);
        id
    }

    /// Get a reference to a block by ID
    pub fn get(&self, id: BlockId) -> &BasicBlock {
        &self.blocks[id.0]
    }

    /// Get a mutable reference to a block by ID
    pub fn get_mut(&mut self, id: BlockId) -> &mut BasicBlock {
        &mut self.blocks[id.0]
    }

    /// Iterate over all blocks with their IDs
    pub fn iter(&self) -> impl Iterator<Item = (BlockId, &BasicBlock)> {
        self.blocks
            .iter()
            .enumerate()
            .map(|(i, block)| (BlockId(i), block))
    }

    /// Get the number of blocks
    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    /// Check if arena is empty
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }
}

#[derive(Debug)]
pub struct MirFunction {
    pub name: String,
    pub params: Vec<(Reg, MirType)>,
    pub return_type: MirType,
    pub arena: BlockArena,
    pub entry: BlockId,
}

impl MirFunction {
    /// Create a new function with an entry block
    pub fn new(name: String, params: Vec<(Reg, MirType)>, return_type: MirType) -> Self {
        let mut arena = BlockArena::new();

        // Create entry block
        let entry = arena.alloc(BasicBlock {
            instructions: Vec::new(),
            terminator: Terminator::Unreachable,
        });

        MirFunction {
            name,
            params,
            return_type,
            arena,
            entry,
        }
    }

    /// Get a reference to a block
    pub fn block(&self, id: BlockId) -> &BasicBlock {
        self.arena.get(id)
    }

    /// Get a mutable reference to a block
    pub fn block_mut(&mut self, id: BlockId) -> &mut BasicBlock {
        self.arena.get_mut(id)
    }
}

pub struct MirProgram {
    pub functions: Vec<MirFunction>,
}

// Example usage:
//
// let mut func = MirFunction::new("test".to_string());
//
// // Create a new block
// let block_id = func.arena.alloc(BasicBlock {
//     instructions: vec![
//         Instruction {
//             dest: "x".to_string(),
//             op: Opcode::Add,
//             typ: MirType::F64,
//             args: ["a".to_string(), "b".to_string()],
//         }
//     ],
//     terminator: Terminator::Ret { value: Some("x".to_string()) },
// });
//
// // Set entry to branch to our new block
// func.block_mut(func.entry).terminator = Terminator::Br { target: block_id };
//
// // Access blocks
// let block = func.block(block_id);
// println!("Block has {} instructions", block.instructions.len());
//
// // Iterate over all blocks
// for (id, block) in func.arena.iter() {
//     println!("Block {:?} has {} instructions", id, block.instructions.len());
// }
