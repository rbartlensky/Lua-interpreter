pub mod instructions;

use std::vec::Vec;
use std::fmt;
use self::instructions::{Reg, Instr, Value};

/// A simpler representation of Lua
pub struct LuaBytecode {
    block: Vec<Instr>,
    registers: Vec<Reg>
}

impl LuaBytecode {
    pub fn new(instrs: Vec<Instr>, registers: Vec<Reg>) -> LuaBytecode {
        LuaBytecode {
            block: instrs,
            registers,
        }
    }

    /// Get the number of instructions that are part of this block.
    pub fn instrs_len(&self) -> usize {
        self.block.len()
    }

    /// Get the list of instructions that can be executed in order
    /// to perform some computation.
    pub fn get_instr(&self, index: usize) -> Instr {
        self.block[index].clone()
    }

    /// Set the register to the given value
    pub fn set_value(&mut self, id: usize, value: Value) {
        self.registers[id].set_value(value);
    }

    pub fn get_value(&self, id: usize) -> &Value {
        self.registers[id].get_value()
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

    #[test]
    fn bytecode_works_correctly() {
        let instrs = vec![
            Instr::Mov(0, instructions::Val::LuaValue(Value::Nil))
        ];
        let registers = vec![
            Reg::new(0),
            Reg::new(1)
        ];
        let mut bc = LuaBytecode::new(instrs, registers);
        // check if register values are correctly updated
        assert_eq!(*bc.get_value(0), Value::Nil);
        assert_eq!(*bc.get_value(1), Value::Nil);
        bc.set_value(0, Value::Boolean(false));
        assert_eq!(*bc.get_value(0), Value::Boolean(false));

        assert_eq!(bc.instrs_len(), 1);
        assert_eq!(bc.get_instr(0),
                   Instr::Mov(0, instructions::Val::LuaValue(Value::Nil)));
    }
}
