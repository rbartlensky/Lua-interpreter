pub struct Instr {
    opcode: Opcode,
    args: Vec<usize>,
}

impl Instr {
    pub fn new(opcode: usize, args: Vec<usize>) -> Instr {
        Instr {
            opcode,
            args,
        }
    }
}

pub struct BasicBlock {
    instrs: Vec<Instr>,
}

impl BasicBlock {
    pub fn with_label(label: String) -> BasicBlock {
        BasicBlock {
            previous: vec![],
            next: vec![],
            label,
            instrs: vec![],
        }
    }
}

pub struct Function {
    params: Vec<usize>,
    basic_blocks: Vec<BasicBlock>,
}

pub struct Module {
    globals: Vec<usize>
    functions: Vec<Function>,
}
