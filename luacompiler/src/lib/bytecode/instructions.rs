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
pub const fn make_instr(opcode: Opcode, arg1: u8, arg2: u8, arg3: u8) -> u32 {
    opcode as u32 + ((arg1 as u32) << 8) + ((arg2 as u32) << 16) + ((arg3 as u32) << 24)
}

pub fn format_instr(instr: u32) -> String {
    let i = match opcode(instr) {
        0 => "Mov",
        1 => "Ldi",
        2 => "Ldf",
        3 => "Lds",
        4 => "Add",
        5 => "Sub",
        6 => "Mul",
        7 => "Div",
        8 => "Mod",
        9 => "FDiv",
        10 => "Exp",
        11 => "GetAttr",
        12 => "SetAttr",
        13 => "Closure",
        14 => "Call",
        15 => "Push",
        16 => "VarArg",
        17 => "Eq",
        18 => "MovR",
        19 => "Ret",
        20 => "SetTop",
        _ => unreachable!("No such opcode: {}", opcode(instr)),
    };
    format!(
        "{} {} {} {}",
        i,
        first_arg(instr),
        second_arg(instr),
        third_arg(instr)
    )
}

/// Represents a high level instruction whose operands have a size of usize.
/// This is used by the frontend to create a high-level IR, which later gets translated
/// into smaller instructions that fit in 32 bits.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
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
/// Arg(i) represents the i-th argument; Reg(i) == The Arg(i)-th register
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Opcode {
    MOV = 0,      // R(1) = R(2)
    LDI = 1,      // R(1) = I(1); load integer from the constant table
    LDF = 2,      // R(1) = F(1); load float from the constant table
    LDS = 3,      // R(1) = S(1); load string from the constant table
    ADD = 4,      // R(1) = R(2) + R(3)
    SUB = 5,      // R(1) = R(2) - R(3)
    MUL = 6,      // R(1) = R(2) * R(3)
    DIV = 7,      // R(1) = R(2) / R(3)
    MOD = 8,      // R(1) = R(2) % R(3)
    FDIV = 9,     // R(1) = R(2) // R(3)
    EXP = 10,     // R(1) = R(2) ^ R(3)
    GetAttr = 11, // R(1) = R(2)[R(3)]
    SetAttr = 12, // R(1)[R(2)] = R(3)
    CLOSURE = 13, // R(1) = Closure(R(2))
    CALL = 14,    // call R(1) with Arg(2) arguments
    // Push R(1) to the stack; If Arg(3) is set to some value, then it is added
    // to the number of return values of a function
    PUSH = 15,
    // Copy Arg(2) varargs into registers starting from R(1);
    // If Arg(3) is set to 1, then all varargs are pushed to the stack
    // If Arg(3) is set to 2, then the vm will do the same thing as before,
    // but also increase the count of the return values of a function
    VarArg = 16,
    EQ = 17, // R(1) == R(2)
    // Copy return value RV(2) into R(1); Arg(3) = 1 or 2 => same reasoning as above
    MOVR = 18,
    RET = 19,    // return to the parent frame
    SetTop = 20, // set R(1)'s `args_start` to the top of the stack
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
