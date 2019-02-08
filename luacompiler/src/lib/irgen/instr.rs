use bytecode::instructions::Opcode;

#[derive(PartialEq, Debug)]
pub enum Arg {
    Nil,
    Int(i64),
    Float(f64),
    Str(String),
    Reg(usize),
    Func(usize),
    Some(usize),
}

#[derive(PartialEq, Debug)]
pub struct Instr {
    pub opcode: Opcode,
    pub args: Vec<Arg>,
}

impl Instr {
    pub fn new(opcode: Opcode, args: Vec<Arg>) -> Instr {
        Instr { opcode, args }
    }
}
