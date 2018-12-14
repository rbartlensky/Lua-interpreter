const MASK: u32 = 0x000000FF;

/// Get the opcode of an instruction
pub fn opcode(instr: u32) -> u8 {
    (instr & MASK) as u8
}

/// Get the first argument of an instruction.
pub fn first_arg(instr: u32) -> u8 {
    ((instr >> 8) & MASK) as u8
}

/// Get the second argument of an instruction.
pub fn second_arg(instr: u32) -> u8 {
    ((instr >> 16) & MASK) as u8
}

/// Get the third argument of an instruction.
pub fn third_arg(instr: u32) -> u8 {
    ((instr >> 24) & MASK) as u8
}

/// Create an instruction with the given opcode and arguments.
pub fn make_instr(opcode: Opcode, arg1: u8, arg2: u8, arg3: u8) -> u32 {
    opcode as u32 + ((arg1 as u32) << 8) + ((arg2 as u32) << 16) + ((arg3 as u32) << 24)
}

/// Represents a high level instruction whose operands have a size of usize.
/// This is used by the frontend to create an SSA IR, which later gets translated
/// into smaller instructions that fit in 32 bits.
#[derive(PartialEq, Eq, Debug)]
pub struct HLInstr(pub Opcode, pub usize, pub usize, pub usize);

impl HLInstr {
    pub fn as_32bit(&self) -> u32 {
        if self.1 > 255 || self.2 > 255 || self.3 > 255 {
            panic!("Value is truncated!");
        }
        make_instr(self.0, self.1 as u8, self.2 as u8, self.3 as u8)
    }
}

/// Represents the supported operations of the bytecode.
/// Each operation can have at most 3 arguments.
/// There are 256 available registers, and load operations (LDI, LDF, LDS) can only
/// refer to at most 256 constants.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Opcode {
    MOV = 0,  // R(1) = R(2)
    LDI = 1,  // R(1) = I(1); load integer from the constant table
    LDF = 2,  // R(1) = F(1); load float from the constant table
    LDS = 3,  // R(1) = S(1); load string from the constant table
    ADD = 4,  // R(1) = R(2) + R(3)
    SUB = 5,  // R(1) = R(2) - R(3)
    MUL = 6,  // R(1) = R(2) * R(3)
    DIV = 7,  // R(1) = R(2) / R(3)
    MOD = 8,  // R(1) = R(2) % R(3)
    FDIV = 9, // R(1) = R(2) // R(3)
    EXP = 10, // R(1) = R(2) ^ R(3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoding_and_decoding_works() {
        for i in 0..=255 {
            for j in 0..=255 {
                for k in 0..=255 {
                    let instr = make_instr(Opcode::MOV, i, j, k);
                    assert_eq!(opcode(instr), Opcode::MOV as u8);
                    assert_eq!(first_arg(instr), i);
                    assert_eq!(second_arg(instr), j);
                    assert_eq!(third_arg(instr), k);
                }
            }
        }
    }
}
