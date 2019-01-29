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
mod stdlib;

use errors::LuaError;
use gc::Gc;
use instructions::{
    arithmetic_operators::*, functions::*, loads::*, relational_operators::*, tables::*,
};
use lua_values::{lua_closure::LuaClosure, lua_table::LuaTable, LuaVal};
use luacompiler::bytecode::{instructions::opcode, LuaBytecode};
use std::collections::HashMap;
use stdlib::STDLIB_FUNCS;

/// The maximum number of registers of the VM.
const REG_NUM: usize = 256;

/// The instruction handler for each opcode.
const OPCODE_HANDLER: &'static [fn(&mut Vm, u32) -> Result<(), LuaError>] = &[
    mov, ldi, ldf, lds, add, sub, mul, div, modulus, fdiv, exp, get_attr, set_attr, closure, call,
    push, vararg, eq,
];

/// Represents a `LuaBytecode` interpreter.
pub struct Vm {
    pub bytecode: LuaBytecode,
    pub registers: Vec<LuaVal>,
    pub stack: Vec<LuaVal>,
    pub top: usize,
    /// All attributes of _ENV that are also part of the string constant table are stored
    /// in a vector. Let's consider an example: "x" is mapped to index 2 in the constant
    /// table. This means that _ENV["x"] = <val> will modify env_attrs[2]. If however
    /// "x" was not in the constant table, then the lookup of the attribute would be
    /// done via the `get_attr` method of the `LuaTable` struct.
    pub env_attrs: Vec<LuaVal>,
    pub closure: Gc<Box<LuaClosure>>,
}

impl Vm {
    /// Create a new interpreter for the given bytecode.
    pub fn new(bytecode: LuaBytecode) -> Vm {
        let mut registers: Vec<LuaVal> = Vec::with_capacity(REG_NUM);
        registers.push(LuaVal::from(LuaTable::new(HashMap::new())));
        for _ in 1..REG_NUM {
            registers.push(LuaVal::new());
        }
        let mut env_attrs = Vec::new();
        env_attrs.resize(bytecode.get_strings_len(), LuaVal::new());
        Vm::init_stdlib(&bytecode, &mut registers[0], &mut env_attrs);
        let closure = LuaVal::from(bytecode.get_function(bytecode.get_main_function()));
        Vm {
            bytecode,
            registers,
            stack: vec![],
            top: 0,
            env_attrs,
            closure: closure.get_closure().unwrap(),
        }
    }

    fn init_stdlib(bc: &LuaBytecode, env: &mut LuaVal, env_attrs: &mut Vec<LuaVal>) {
        let mut strings = bc.strings().iter().enumerate();
        for func in STDLIB_FUNCS {
            if let Some(res) = strings.find(|s| s.1 == func.name()) {
                env_attrs[res.0] = LuaVal::from(func);
            } else {
                env.set_attr(LuaVal::from(func.name().to_string()), LuaVal::from(func))
                    .unwrap();
            }
        }
    }

    /// Evaluate the program.
    pub fn eval(&mut self) {
        let mut pc = 0;
        let len = self
            .bytecode
            .get_function(self.closure.index())
            .instrs_len();
        while pc < len {
            let instr = self
                .bytecode
                .get_function(self.closure.index())
                .get_instr(pc);
            (OPCODE_HANDLER[opcode(instr) as usize])(self, instr).unwrap();
            pc += 1;
        }
    }

    pub fn push(&mut self, val: LuaVal) {
        if self.top < self.stack.len() {
            self.stack[self.top] = val;
        } else {
            self.stack.push(val);
        }
        self.top += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luacompiler::irgen::register_map::ENV_REG;
    use luacompiler::{bytecodegen::compile_to_bytecode, irgen::compile_to_ir, LuaParseTree};

    fn get_vm_for(p: String) -> Vm {
        let pt = LuaParseTree::from_str(p).unwrap();
        let ir = compile_to_ir(&pt);
        let bc = compile_to_bytecode(ir);
        Vm::new(bc)
    }

    #[test]
    fn env_set_and_get() {
        let mut vm = get_vm_for(
            "x = 3
             y = x + 1"
                .to_string(),
        );
        vm.eval();
        let index_of_x = 0;
        // vm.registers[0] has a reference to the _ENV variable
        // this is true because the compiler always loads the environment into register 0
        assert_eq!(
            vm.registers[ENV_REG]
                .get_attr(&LuaVal::from((String::from("x"), index_of_x)))
                .unwrap(),
            LuaVal::new()
        );
        assert_eq!(vm.env_attrs[index_of_x], LuaVal::from(3));
        let index_of_y = 1;
        assert_eq!(
            vm.registers[ENV_REG]
                .get_attr(&LuaVal::from((String::from("y"), index_of_y)))
                .unwrap(),
            LuaVal::new()
        );
        assert_eq!(vm.env_attrs[index_of_y], LuaVal::from(4));
    }

    #[test]
    fn function_call() {
        let mut vm = get_vm_for(
            "function f()
                 x = 3
                 local x = 4
             end
             f()"
            .to_string(),
        );
        vm.eval();
        let index_of_x = 0;
        // env is correctly updated
        assert_eq!(vm.env_attrs[index_of_x], LuaVal::from(3));
    }

    #[test]
    fn function_call_with_args() {
        let mut vm = get_vm_for(
            "function f(a)
                 x = a
                 local x = 4
             end
             f(3)"
                .to_string(),
        );
        vm.eval();
        let index_of_x = 0;
        // env is correctly updated
        assert_eq!(vm.env_attrs[index_of_x], LuaVal::from(3));
    }

    #[test]
    fn function_call_with_extra_args() {
        let mut vm = get_vm_for(
            "function f(a)
                 x = a
                 local x = 4
             end
             f(3, 4, 5)"
                .to_string(),
        );
        vm.eval();
        let index_of_x = 0;
        // env is correctly updated
        assert_eq!(vm.env_attrs[index_of_x], LuaVal::from(3));
    }

    #[test]
    fn function_call_with_no_args() {
        let mut vm = get_vm_for(
            "function f(a)
                 x = a
                 local x = 4
             end
             f()"
            .to_string(),
        );
        vm.eval();
        let index_of_x = 0;
        // env is correctly updated
        assert_eq!(vm.env_attrs[index_of_x], LuaVal::new());
    }

    #[test]
    fn function_call_with_varargs() {
        let mut vm = get_vm_for(
            "function f(a, ...)
                 x = a
                 y, z, w = ...
             end
             f(1, 2, 3)"
                .to_string(),
        );
        vm.eval();
        let index_of_x = 0;
        let index_of_y = 1;
        let index_of_z = 2;
        let index_of_w = 3;
        // env is correctly updated
        assert_eq!(vm.env_attrs[index_of_x], LuaVal::from(1));
        assert_eq!(vm.env_attrs[index_of_y], LuaVal::from(2));
        assert_eq!(vm.env_attrs[index_of_z], LuaVal::from(3));
        assert_eq!(vm.env_attrs[index_of_w], LuaVal::new());
    }
}
