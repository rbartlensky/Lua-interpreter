#[macro_use]
extern crate gc_derive;
#[macro_use]
extern crate gc;
extern crate luacompiler;
#[macro_use]
#[cfg(test)]
extern crate assert_float_eq;

mod errors;
mod instructions;
mod lua_values;

use errors::LuaError;
use instructions::{arithmetic_operators::*, loads::*};
use lua_values::LuaVal;
use luacompiler::bytecode::{instructions::opcode, LuaBytecode};

/// The instruction handler for each opcode.
const OPCODE_HANDLER: &'static [fn(&mut Vm, u32) -> Result<(), LuaError>] =
    &[mov, ldi, ldf, lds, add, sub, mul, div, modulus, fdiv, exp];

/// Represents a `LuaBytecode` interpreter.
pub struct Vm {
    pub bytecode: LuaBytecode,
    pub registers: Vec<LuaVal>,
}

impl Vm {
    /// Create a new interpreter for the given bytecode.
    pub fn new(bytecode: LuaBytecode) -> Vm {
        let regs = bytecode.reg_count();
        let mut registers: Vec<LuaVal> = Vec::with_capacity(regs as usize);
        for _ in 0..regs {
            registers.push(LuaVal::new());
        }
        Vm {
            bytecode,
            registers,
        }
    }

    /// Evaluate the program.
    pub fn eval(&mut self) {
        let mut pc = 0;
        let len = self.bytecode.instrs_len();
        while pc < len {
            let instr = self.bytecode.get_instr(pc);
            (OPCODE_HANDLER[opcode(instr) as usize])(self, instr).unwrap();
            pc += 1;
        }
    }
}
