const MASK: u32 = 0x000000FF;
const EXTENDED_MASK: u32 = 0x0000FFFF;

/// Get the opcode of an instruction
#[inline]
pub const fn opcode(instr: u32) -> u8 {
    (instr & MASK) as u8
}

/// Get the first argument of an instruction.
#[inline]
pub const fn first_arg(instr: u32) -> u8 {
    ((instr >> 8) & MASK) as u8
}

#[inline]
pub fn set_first_arg(instr: &mut u32, v: u8) {
    *instr |= (v as u32) << 8;
}

/// Get the second argument of an instruction.
#[inline]
pub const fn second_arg(instr: u32) -> u8 {
    ((instr >> 16) & MASK) as u8
}

#[inline]
pub fn set_second_arg(instr: &mut u32, v: u8) {
    *instr |= (v as u32) << 16;
}

/// Get the third argument of an instruction.
#[inline]
pub const fn third_arg(instr: u32) -> u8 {
    ((instr >> 24) & MASK) as u8
}

/// Get the second argument of an instruction.
#[inline]
pub const fn extended_arg(instr: u32) -> i16 {
    ((instr >> 16) & EXTENDED_MASK) as i16
}

#[inline]
pub fn set_extended_arg(instr: &mut u32, v: i16) {
    *instr |= (v as u32) << 16;
}

/// Create an instruction with the given opcode and arguments.
#[inline]
pub const fn make_instr(opcode: Opcode, arg1: u8, arg2: u8, arg3: u8) -> u32 {
    opcode as u32 + ((arg1 as u32) << 8) + ((arg2 as u32) << 16) + ((arg3 as u32) << 24)
}

/// Create an instruction with the given opcode and arguments.
#[inline]
pub const fn make_extended_instr(opcode: Opcode, arg1: u8, arg2: i16) -> u32 {
    opcode as u32 + ((arg1 as u32) << 8) + ((arg2 as u32) << 16)
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
        21 => "GetUpAttr",
        22 => "SetUpAttr",
        23 => "Jmp",
        24 => "JmpNE",
        25 => "LT",
        26 => "GT",
        27 => "LE",
        28 => "GE",
        29 => "NE",
        30 => "JmpEQ",
        31 => "GetUpval",
        32 => "SetUpval",
        33 => "Ldn",
        34 => "Ldt",
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
    RET = 19,       // return to the parent frame
    SetTop = 20,    // set R(1)'s `args_start` to the top of the stack
    GetUpAttr = 21, // R(1) = Upvals[Arg(2)][Arg(3)]
    SetUpAttr = 22, // Upvals[Arg(1)][Arg(2)] = R(3)
    Jmp = 23,
    JmpNE = 24,
    LT = 25, // R(1) = R(2) < R(3)
    GT = 26, // R(1) = R(2) > R(3)
    LE = 27, // R(1) = R(2) <= R(3)
    GE = 28, // R(1) = R(2) >= R(3)
    NE = 29, // R(1) = R(2) != R(3)
    JmpEQ = 30,
    GetUpVal = 31, // R(1) = UpVals[Arg(2)]
    SetUpVal = 32, // UpVals[Arg(1)] = R(2)
    LDN = 33,      // R(1) = Nil
    LDT = 34,      // R(1) = {}
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
