use std::fmt;

pub enum Instr {
    Mov(String, String),
    Add(String, String, String),
    Sub(String, String, String),
    Mul(String, String, String),
    Div(String, String, String),
    Mod(String, String, String),
}

impl fmt::Display for Instr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Instr::Mov(ref store, ref val) => write!(f, "(mov {} {})", store, val),
            Instr::Add(ref store, ref lhs, ref rhs) => write!(f, "(add {} {} {})", store, lhs, rhs),
            Instr::Sub(ref store, ref lhs, ref rhs) => write!(f, "(sub {} {} {})", store, lhs, rhs),
            Instr::Mul(ref store, ref lhs, ref rhs) => write!(f, "(mul {} {} {})", store, lhs, rhs),
            Instr::Div(ref store, ref lhs, ref rhs) => write!(f, "(div {} {} {})", store, lhs, rhs),
            Instr::Mod(ref store, ref lhs, ref rhs) => write!(f, "(mod {} {} {})", store, lhs, rhs),
        }
    }
}
