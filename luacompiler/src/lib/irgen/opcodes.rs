use bytecode::instructions::Opcode;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum IROpcode {
    Opcode(Opcode),
    Branch,
    Phi,
}

impl From<Opcode> for IROpcode {
    fn from(o: Opcode) -> Self {
        IROpcode::Opcode(o)
    }
}
