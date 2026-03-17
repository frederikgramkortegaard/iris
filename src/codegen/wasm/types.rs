#[derive(Debug)]
pub struct WatModule {
    pub functions: Vec<WatFunction>,
}

#[derive(Debug)]
pub struct WatFunction {
    pub name: String,
    pub params: Vec<(String, WatType)>,
    pub result: Option<WatType>,
    pub locals: Vec<(String, WatType)>,
    pub body: Vec<WatInstruction>,
}

#[derive(Debug, Clone, Copy)]
pub enum WatType {
    I32,
    I64,
    F32,
    F64,
}

#[derive(Debug, PartialEq)]
pub enum WatInstruction {
    // Constants
    F64Const(f64),
    F32Const(f32),
    I32Const(i32),

    // f64 arithmetic
    F64Add,
    F64Sub,
    F64Mul,
    F64Div,

    // f32 arithmetic
    F32Add,
    F32Sub,
    F32Mul,
    F32Div,

    // f64 comparisons (produce i32)
    F64Eq,
    F64Ne,
    F64Lt,
    F64Le,
    F64Gt,
    F64Ge,

    // f32 comparisons (produce i32)
    F32Eq,
    F32Ne,
    F32Lt,
    F32Le,
    F32Gt,
    F32Ge,

    // i32 arithmetic
    I32Add,
    I32Sub,
    I32Mul,
    I32DivS,

    // i32 comparisons
    I32Eq,
    I32Ne,
    I32LtS,
    I32LeS,
    I32GtS,
    I32GeS,
    I32Eqz,

    // Variables
    LocalGet(u32),
    LocalSet(u32),

    // Structured control flow
    Block {
        label: Option<String>,
        body: Vec<WatInstruction>,
    },
    Loop {
        label: Option<String>,
        body: Vec<WatInstruction>,
    },
    If {
        then_body: Vec<WatInstruction>,
        else_body: Vec<WatInstruction>,
    },
    Br(u32),
    BrIf(u32),
    Return,
    Unreachable,
    Call(String),
}
