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

impl Arg {
    pub fn get_reg(&self) -> usize {
        if let Arg::Reg(reg) = self {
            *reg
        } else {
            panic!("Arg was not a Reg; received {:?}", self)
        }
    }

    pub fn get_some(&self) -> usize {
        if let Arg::Some(some) = self {
            *some
        } else {
            panic!("Arg was not a Some; received {:?}", self)
        }
    }

    pub fn get_str(&self) -> String {
        if let Arg::Str(s) = self {
            s.clone()
        } else {
            panic!("Arg was not a Str; received {:?}", self)
        }
    }

    pub fn get_func(&self) -> usize {
        if let Arg::Func(f) = self {
            *f
        } else {
            panic!("Arg was not a Func; received {:?}", self)
        }
    }
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
