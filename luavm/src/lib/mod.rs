extern crate luacompiler;

mod instructions;
mod lua_value;

use instructions::{arithmetic_operators::*, loads::*};
use lua_value::{LuaNil, LuaValue};
use luacompiler::bytecode::{instructions::opcode, LuaBytecode};

/// The instruction handler for each opcode.
const OPCODE_HANDLER: &'static [fn(&mut Vm, u32)] =
    &[mov, ldi, ldf, lds, add, sub, mul, div, modulus, fdiv, exp];

/// Represents a `LuaBytecode` interpreter.
pub struct Vm {
    pub bytecode: LuaBytecode,
    pub registers: Vec<Box<LuaValue>>,
}

impl Vm {
    /// Create a new interpreter for the given bytecode.
    pub fn new(bytecode: LuaBytecode) -> Vm {
        let regs = bytecode.reg_count();
        let mut registers: Vec<Box<LuaValue>> = Vec::with_capacity(regs as usize);
        for _ in 0..regs {
            registers.push(Box::new(LuaNil {}));
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
            (OPCODE_HANDLER[opcode(instr) as usize])(self, instr);
            pc += 1;
        }
    }
}
