pub mod analysis;
pub mod passes;
pub mod visitor;
use std::collections::{BTreeMap, BTreeSet};

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

impl Operand {
    pub fn is_constant(&self) -> bool {
        !matches!(
            self,
            Operand::Reg(_) | Operand::Pair { .. } | Operand::Label(_)
        )
    }

    /// Call f on every register in this operand (recurses through Pair)
    pub fn for_each_reg(&self, f: &mut impl FnMut(Reg)) {
        match self {
            Operand::Reg(r) => f(*r),
            Operand::Pair(_, inner) => inner.for_each_reg(f),
            _ => {}
        }
    }

    /// Rewrite every register in this operand through f (recurses through Pair)
    pub fn map_regs(&mut self, f: &mut impl FnMut(Reg) -> Reg) {
        match self {
            Operand::Reg(r) => *r = f(*r),
            Operand::Pair(_, inner) => inner.map_regs(f),
            _ => {}
        }
    }
}
/// Type-safe block identifier (index into BlockArena)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
    pub phi_nodes: Vec<Instruction>,
    pub note: Option<String>,
}

#[derive(Debug, Clone)]
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

impl Terminator {
    /// Blocks this terminator can branch to
    pub fn successors(&self) -> Vec<BlockId> {
        match self {
            Terminator::Br { target } => vec![*target],
            Terminator::BrIf {
                then_bb, else_bb, ..
            } => vec![*then_bb, *else_bb],
            Terminator::Ret { .. } | Terminator::Unreachable => vec![],
        }
    }

    /// Operands this terminator reads (BrIf cond, Ret value)
    pub fn operands(&self) -> impl Iterator<Item = &Operand> {
        match self {
            Terminator::BrIf { cond, .. } => Some(cond),
            Terminator::Ret { value } => value.as_ref(),
            _ => None,
        }
        .into_iter()
    }

    pub fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        match self {
            Terminator::BrIf { cond, .. } => Some(cond),
            Terminator::Ret { value } => value.as_mut(),
            _ => None,
        }
        .into_iter()
    }

    /// Rewrite each branch target through f (each target exactly once)
    pub fn map_targets(&mut self, mut f: impl FnMut(BlockId) -> BlockId) {
        match self {
            Terminator::Br { target } => *target = f(*target),
            Terminator::BrIf {
                then_bb, else_bb, ..
            } => {
                *then_bb = f(*then_bb);
                *else_bb = f(*else_bb);
            }
            _ => {}
        }
    }

    /// Replace every branch target equal to `from` with `to`
    pub fn retarget(&mut self, from: BlockId, to: BlockId) {
        self.map_targets(|t| if t == from { to } else { t });
    }
}

/// Arena for allocating basic blocks.
/// Backed by a BTreeMap so iteration is always in block id order --
/// pass output must not depend on hash ordering.
#[derive(Debug)]
pub struct BlockArena {
    blocks: BTreeMap<usize, BasicBlock>,
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
            blocks: BTreeMap::new(),
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

    /// Uses up the next free BlockId without automatically allocating a new bock
    pub fn reserve(&mut self) -> BlockId {
        self.next_id += 1;
        BlockId(self.next_id - 1)
    }

    /// Get a reference to a block by ID
    pub fn get(&self, id: BlockId) -> &BasicBlock {
        self.blocks.get(&id.0).expect("Invalid BlockId")
    }

    /// Get a mutable reference to a block by ID
    pub fn get_mut(&mut self, id: BlockId) -> &mut BasicBlock {
        self.blocks.get_mut(&id.0).expect("Invalid BlockId")
    }

    /// Iterate over all blocks with their IDs, in block id order
    pub fn iter(&self) -> impl Iterator<Item = (BlockId, &BasicBlock)> {
        self.blocks.iter().map(|(&id, block)| (BlockId(id), block))
    }

    /// Iterate mutably over all blocks with their IDs, in block id order
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (BlockId, &mut BasicBlock)> {
        self.blocks
            .iter_mut()
            .map(|(&id, block)| (BlockId(id), block))
    }

    /// Remove a block by ID
    pub fn remove(&mut self, id: BlockId) {
        self.blocks.remove(&id.0);
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

/// Type-safe function identifier (index into FunctionArena)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FuncId(usize);

impl FuncId {
    pub fn new(id: usize) -> Self {
        FuncId(id)
    }

    pub fn index(self) -> usize {
        self.0
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
    pub definitions: BTreeMap<Reg, BTreeSet<BlockId>>,
    pub next_free_reg: Reg,
    pub loops: Option<Vec<analysis::loops::Loop>>,
}

impl Function {
    /// Bump `next_free_reg`
    pub fn reserve_next_free_reg(&mut self) -> Reg {
        self.next_free_reg += 1;
        self.next_free_reg - 1
    }

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
            definitions: BTreeMap::new(),
            next_free_reg: 0,
            loops: None,
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

/// Arena for allocating functions.
/// Same deal as BlockArena: the map key is the id, so removing a function
/// can never invalidate another one's id, and BTreeMap iteration is always
/// in id order.
#[derive(Debug, Default)]
pub struct FunctionArena {
    functions: BTreeMap<usize, Function>,
    next_id: usize,
}

impl FunctionArena {
    pub fn new() -> Self {
        FunctionArena {
            functions: BTreeMap::new(),
            next_id: 0,
        }
    }

    /// Allocate a new function and return its ID
    pub fn alloc(&mut self, function: Function) -> FuncId {
        let id = FuncId(self.next_id);
        self.next_id += 1;
        self.functions.insert(id.0, function);
        id
    }

    /// Get a reference to a function by ID
    pub fn get(&self, id: FuncId) -> &Function {
        self.functions.get(&id.0).expect("Invalid FuncId")
    }

    /// Get a mutable reference to a function by ID
    pub fn get_mut(&mut self, id: FuncId) -> &mut Function {
        self.functions.get_mut(&id.0).expect("Invalid FuncId")
    }

    /// Iterate over all functions with their IDs, in id order
    pub fn iter(&self) -> impl Iterator<Item = (FuncId, &Function)> {
        self.functions.iter().map(|(&id, f)| (FuncId(id), f))
    }

    /// Iterate mutably over all functions with their IDs, in id order
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (FuncId, &mut Function)> {
        self.functions.iter_mut().map(|(&id, f)| (FuncId(id), f))
    }

    /// Remove a function by ID
    pub fn remove(&mut self, id: FuncId) {
        self.functions.remove(&id.0);
    }

    /// Get the number of functions
    pub fn len(&self) -> usize {
        self.functions.len()
    }

    /// Check if arena is empty
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }
}

pub struct Program {
    pub functions: FunctionArena,
}
