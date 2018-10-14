pub mod instructions;

use std::vec::Vec;
use std::fmt;
use self::instructions::{Instr};

/// A simpler representation of Lua
pub struct LuaBytecode {
    block: Vec<Instr>,
    reg_count: usize
}

impl LuaBytecode {
    pub fn new(instrs: Vec<Instr>, reg_count: usize) -> LuaBytecode {
        LuaBytecode {
            block: instrs,
            reg_count,
        }
    }

    /// Get the number of instructions that are part of this block.
    pub fn instrs_len(&self) -> usize {
        self.block.len()
    }

    /// Get the list of instructions that can be executed in order
    /// to perform some computation.
    pub fn get_instr(&self, index: usize) -> &Instr {
        &self.block[index]
    }

    /// Get the number of registers that this bytecode uses in order to encode
    /// instructions.
    pub fn reg_count(&self) -> usize {
        self.reg_count
    }
}

impl fmt::Display for LuaBytecode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut to_write = String::from("{\n");
        for instr in &self.block {
            to_write = format!("{}  {}\n", to_write, instr);
        }
        to_write += "}";
        write!(f, "{}", to_write)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::instructions::*;

    #[test]
    fn bytecode_works_correctly() {
        let instrs = vec![
            Instr::Mov(0, Val::LuaValue(Value::Nil))
        ];
        let mut bc = LuaBytecode::new(instrs, 3);
        assert_eq!(bc.reg_count(), 3);
        assert_eq!(bc.instrs_len(), 1);
        assert_eq!(*bc.get_instr(0),
                   Instr::Mov(0, instructions::Val::LuaValue(Value::Nil)));
    }
}
