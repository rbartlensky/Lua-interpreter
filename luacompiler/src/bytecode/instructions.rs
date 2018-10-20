const MASK: u32 = 0x000F;

/// Get the opcode of an instruction
pub fn opcode(instr: u32) -> u8 {
    (instr & MASK) as u8
}

/// Get the first argument of an instruction.
pub fn first_arg(instr: u32) -> u8 {
    ((instr >> 4) & MASK) as u8
}

/// Get the second argument of an instruction.
pub fn second_arg(instr: u32) -> u8 {
    ((instr >> 8) & MASK) as u8
}

/// Get the third argument of an instruction.
pub fn third_arg(instr: u32) -> u8 {
    ((instr >> 12) & MASK) as u8
}

/// Create an instruction with the given opcode and arguments.
pub fn make_instr(opcode: Opcode, arg1: u8, arg2: u8, arg3: u8) -> u32 {
    opcode as u32 + ((arg1 as u32) << 4) + ((arg2 as u32) << 8) + ((arg3 as u32) << 12)
}

/// Represents the supported operations of the bytecode.
/// Each operation can have at most 3 arguments.
/// There are 256 available registers, and load operations (LDI, LDF, LDS) can only
/// refer to at most 256 constants.
pub enum Opcode {
    MOV =   0, // R(1) = R(2)
    LDI =   1, // R(1) = I(1); load integer from the constant table
    LDF =   2, // R(1) = F(1); load float from the constant table
    LDS =   3, // R(1) = S(1); load string from the constant table
    ADD =   4, // R(1) = R(2) + R(3)
    SUB =   5, // R(1) = R(2) - R(3)
    MUL =   6, // R(1) = R(2) * R(3)
    DIV =   7, // R(1) = R(2) / R(3)
    MOD =   8, // R(1) = R(2) % R(3)
    FDIV =  9, // R(1) = R(2) // R(3)
    EXP =  10  // R(1) = R(2) ^ R(3)
}
