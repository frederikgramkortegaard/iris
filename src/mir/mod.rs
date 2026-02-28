pub mod cfg;
pub mod passes;
pub mod visitor;
use std::collections::{HashMap, HashSet};

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
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

    Phi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
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
    Pair(BlockId, Box<Operand>), // Used for Phi nodes
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

#[derive(Debug, Clone)]
pub struct Instruction {
    pub dest: Reg,
    pub op: Opcode,
    pub typ: Type,
    pub args: Vec<Operand>,
}

#[derive(Debug)]
pub struct BasicBlock {
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
    pub phi_nodes: Vec<Instruction>,
    pub note: Option<String>,
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
    blocks: HashMap<usize, BasicBlock>,
    next_id: usize,
}

impl Default for BlockArena {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockArena {
    pub fn new() -> Self {
        BlockArena {
            blocks: HashMap::new(),
            next_id: 0,
        }
    }

    /// Allocate a new block and return its ID
    pub fn alloc(&mut self, block: BasicBlock) -> BlockId {
        let id = BlockId(self.next_id);
        self.next_id += 1;
        self.blocks.insert(id.0, block);
        id
    }

    /// Get a reference to a block by ID
    pub fn get(&self, id: BlockId) -> &BasicBlock {
        self.blocks.get(&id.0).expect("Invalid BlockId")
    }

    /// Get a mutable reference to a block by ID
    pub fn get_mut(&mut self, id: BlockId) -> &mut BasicBlock {
        self.blocks.get_mut(&id.0).expect("Invalid BlockId")
    }

    /// Iterate over all blocks with their IDs (unordered)
    pub fn iter(&self) -> impl Iterator<Item = (BlockId, &BasicBlock)> {
        self.blocks.iter().map(|(&id, block)| (BlockId(id), block))
    }

    /// Iterate mutably over all blocks with their IDs (unordered)
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (BlockId, &mut BasicBlock)> {
        self.blocks
            .iter_mut()
            .map(|(&id, block)| (BlockId(id), block))
    }

    /// Iterate over all blocks sorted by BlockId (for deterministic output)
    pub fn iter_sorted(&self) -> impl Iterator<Item = (BlockId, &BasicBlock)> {
        let mut entries: Vec<_> = self.blocks.iter().collect();
        entries.sort_by_key(|(id, _)| *id);
        entries.into_iter().map(|(&id, block)| (BlockId(id), block))
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
pub struct Function {
    pub name: String,
    pub params: Vec<(Reg, Type)>,
    pub return_type: Type,
    pub arena: BlockArena,
    pub entry: BlockId,
    pub virtual_entry: BlockId,
    pub definitions: HashMap<Reg, HashSet<BlockId>>,
    pub next_free_reg: Reg,
}

impl Function {
    /// Create a new function with an entry block and virtual entry
    pub fn new(name: String, params: Vec<(Reg, Type)>, return_type: Type) -> Self {
        let mut arena = BlockArena::new();

        // Create real entry block (where code goes)
        let entry = arena.alloc(BasicBlock {
            instructions: Vec::new(),
            terminator: Terminator::Unreachable,
            phi_nodes: Vec::new(),
            note: Some("entry".to_string()),
        });

        // Create virtual entry block (for CFG purposes, branches to real entry)
        let virtual_entry = arena.alloc(BasicBlock {
            instructions: Vec::new(),
            terminator: Terminator::Br { target: entry },
            phi_nodes: Vec::new(),
            note: Some("virtual_entry".to_string()),
        });

        Function {
            name,
            params,
            return_type,
            arena,
            entry,
            virtual_entry,
            definitions: HashMap::new(),
            next_free_reg: 0,
        }
    }

    /// Look up the type of a register by scanning params and all instruction destinations.
    /// Returns None if the register is not found.
    pub fn reg_type(&self, reg: Reg) -> Option<Type> {
        // Check function parameters first
        if let Some((_, typ)) = self.params.iter().find(|(r, _)| *r == reg) {
            return Some(*typ);
        }
        // Scan all instructions in all blocks
        for (_, block) in self.arena.iter() {
            for inst in &block.instructions {
                if inst.dest == reg {
                    return Some(inst.typ);
                }
            }
        }
        None
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

pub struct Program {
    pub functions: Vec<Function>,
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
//             typ: Type::F64,
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
//
