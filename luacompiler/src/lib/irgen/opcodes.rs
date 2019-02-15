use bytecode::instructions::Opcode;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum IROpcode {
    Opcode(Opcode),
    Branch,
    Phi,
}

impl IROpcode {
    pub fn opcode(&self) -> Opcode {
        if let IROpcode::Opcode(o) = self {
            *o
        } else {
            panic!("IROpcode is not an Opcode!")
        }
    }
}

impl From<Opcode> for IROpcode {
    fn from(o: Opcode) -> Self {
        IROpcode::Opcode(o)
    }
}
