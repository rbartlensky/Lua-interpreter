#[macro_use]
extern crate gc_derive;
extern crate gc;
extern crate luacompiler;
#[macro_use]
#[cfg(test)]
extern crate assert_float_eq;
extern crate ieee754;

mod errors;
mod instructions;
mod lua_std;
mod lua_values;
mod stdlib;

use errors::LuaError;
use instructions::{
    arithmetic_operators::*, control::*, functions::*, loads::*, relational_operators::*,
    tables::*, upvals::*,
};
use lua_std::{io::get_io_module, string::get_string_module};
use lua_values::{lua_closure::UserBcFunc, lua_table::UserTable, LuaVal};
use luacompiler::bytecode::{instructions::*, LuaBytecode};
use std::collections::HashMap;
use stdlib::STDLIB_FUNCS;

/// The maximum number of registers of the VM.
const REG_NUM: usize = 256;

/// The instruction handler for each opcode.
const OPCODE_HANDLER: &'static [fn(&mut Vm, u32) -> Result<(), LuaError>] = &[
    mov,
    ldi,
    ldf,
    lds,
    add,
    sub,
    mul,
    div,
    modulus,
    fdiv,
    exp,
    get_attr,
    set_attr,
    closure,
    call,
    push,
    vararg,
    eq,
    movr,
    ret,
    set_top,
    get_up_attr,
    set_up_attr,
    jmp,
    jmp_ne,
    lt,
    gt,
    le,
    ge,
    ne,
    jmp_eq,
    get_upval,
    set_upval,
    ldn,
    ldt,
    umn,
    ldb,
];

pub struct StackFrame {
    pub closure: LuaVal,
    pub start: usize,
}

/// Represents a `LuaBytecode` interpreter.
pub struct Vm {
    pub bytecode: LuaBytecode,
    pub registers: Vec<LuaVal>,
    pub stack: Vec<LuaVal>,
    pub top: usize,
    pub stack_frames: Vec<StackFrame>,
    pub curr_frame: usize,
    /// All attributes of _ENV that are also part of the string constant table are stored
    /// in a vector. Let's consider an example: "x" is mapped to index 2 in the constant
    /// table. This means that _ENV["x"] = <val> will modify env_attrs[2]. If however
    /// "x" was not in the constant table, then the lookup of the attribute would be
    /// done via the `get_attr` method of the `LuaTable` struct.
    pub env: LuaVal,
    pub pc: usize,
}

impl Vm {
    /// Create a new interpreter for the given bytecode.
    pub fn new(bytecode: LuaBytecode, script_args: Vec<&str>) -> Vm {
        let mut registers: Vec<LuaVal> = Vec::new();
        registers.resize(REG_NUM, LuaVal::new());
        let mut env = LuaVal::from(UserTable::new(HashMap::new()));
        Vm::init_stdlib_and_args(&script_args, &mut env);
        let closure = {
            let index = bytecode.get_main_function();
            let main = bytecode.get_function(index);
            let func = UserBcFunc::with_upvals(index, 0, main.param_count(), vec![env.clone()]);
            LuaVal::from(func)
        };
        let mut stack_frames = Vec::with_capacity(255);
        stack_frames.push(StackFrame { closure, start: 0 });
        Vm {
            bytecode,
            registers,
            stack: vec![],
            top: 0,
            stack_frames,
            curr_frame: 0,
            env,
            pc: 0,
        }
    }

    fn init_stdlib_and_args(script_args: &Vec<&str>, env: &mut LuaVal) {
        let args = LuaVal::from(UserTable::new(HashMap::new()));
        for (i, sarg) in script_args.iter().enumerate() {
            args.set_attr(LuaVal::from(i as i64), LuaVal::from(sarg.to_string()))
                .unwrap();
        }
        env.set_attr(LuaVal::from("arg".to_string()), args).unwrap();
        for func in STDLIB_FUNCS {
            env.set_attr(LuaVal::from(func.name().to_string()), LuaVal::from(func))
                .unwrap();
        }
        let io = get_io_module();
        env.set_attr(LuaVal::from(io.0.as_str().to_string()), io.1)
            .unwrap();
        let string = get_string_module();
        env.set_attr(LuaVal::from(string.0.as_str().to_string()), string.1)
            .unwrap();
    }

    pub fn closure(&self) -> &LuaVal {
        &self.stack_frames[self.curr_frame].closure
    }

    /// Evaluate the program.
    pub fn eval(&mut self) -> Result<(), LuaError> {
        self.pc = 0;
        let index = self.closure().index()?;
        let len = self.bytecode.get_function(index).instrs_len();
        while self.pc < len {
            let instr = self.bytecode.get_function(index).get_instr(self.pc);
            (OPCODE_HANDLER[opcode(instr) as usize])(self, instr)?;
            self.pc += 1;
        }
        Ok(())
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
    use luacompiler::{bytecodegen::compile_to_bytecode, irgen::compile_to_ir, LuaParseTree};

    fn get_vm_for(p: String) -> Vm {
        let pt = LuaParseTree::from_str(p).unwrap();
        let ir = compile_to_ir(&pt);
        let bc = compile_to_bytecode(ir);
        Vm::new(bc, vec![])
    }

    #[test]
    fn env_set_and_get() {
        let mut vm = get_vm_for(
            "x = 3
             y = x + 1"
                .to_string(),
        );
        vm.eval().unwrap();
        // vm.registers[0] has a reference to the _ENV variable
        // this is true because the compiler always loads the environment into register 0
        assert_eq!(
            vm.env.get_attr(&LuaVal::from(String::from("x"))).unwrap(),
            LuaVal::from(3)
        );
        assert_eq!(
            vm.env.get_attr(&LuaVal::from(String::from("y"))).unwrap(),
            LuaVal::from(4)
        );
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
        vm.eval().unwrap();
        // env is correctly updated
        assert_eq!(
            vm.env.get_attr(&LuaVal::from(String::from("x"))).unwrap(),
            LuaVal::from(3)
        );
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
        vm.eval().unwrap();
        // env is correctly updated
        assert_eq!(
            vm.env.get_attr(&LuaVal::from(String::from("x"))).unwrap(),
            LuaVal::from(3)
        );
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
        vm.eval().unwrap();
        // env is correctly updated
        assert_eq!(
            vm.env.get_attr(&LuaVal::from(String::from("x"))).unwrap(),
            LuaVal::from(3)
        );
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
        vm.eval().unwrap();
        // env is correctly updated
        assert_eq!(
            vm.env.get_attr(&LuaVal::from(String::from("x"))).unwrap(),
            LuaVal::new()
        );
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
        vm.eval().unwrap();
        // env is correctly updated
        let expected_vals = vec![
            LuaVal::from(1),
            LuaVal::from(2),
            LuaVal::from(3),
            LuaVal::new(),
        ];
        let strs = vec!["x", "y", "z", "w"];
        for i in 1..(strs.len() + 1) {
            assert_eq!(
                vm.env
                    .get_attr(&LuaVal::from(String::from(strs[i - 1])))
                    .unwrap(),
                expected_vals[i - 1]
            );
        }
    }

    #[test]
    fn function_call_with_rets() {
        let mut vm = get_vm_for(
            "function f(a, b)
                 return a + 1, b + 1
             end
             x, y = f(1, 2)
             z, w = f(10, 4), 5"
                .to_string(),
        );
        vm.eval().unwrap();
        assert_eq!(vm.top, 0);
        let expected_vals = vec![
            LuaVal::from(2),
            LuaVal::from(3),
            LuaVal::from(11),
            LuaVal::from(5),
        ];
        let strs = vec!["x", "y", "z", "w"];
        for i in 1..(strs.len() + 1) {
            assert_eq!(
                vm.env
                    .get_attr(&LuaVal::from(String::from(strs[i - 1])))
                    .unwrap(),
                expected_vals[i - 1]
            );
        }
    }

    #[test]
    fn function_call_returning_function_result() {
        let mut vm = get_vm_for(
            "function g()
                 return 1, 2, 3
             end
             function f(a)
                 return a + 1, g()
             end
             x, y, z, w = f(0)
             w1, w2 = f(10), 10"
                .to_string(),
        );
        vm.eval().unwrap();
        assert!(vm.top == 0);
        let expected_vals = vec![
            LuaVal::from(1),
            LuaVal::from(1),
            LuaVal::from(2),
            LuaVal::from(3),
            LuaVal::from(11),
            LuaVal::from(10),
        ];
        let strs = vec!["x", "y", "z", "w", "w1", "w2"];
        for i in 2..(strs.len() + 2) {
            assert_eq!(
                vm.env
                    .get_attr(&LuaVal::from(String::from(strs[i - 2])))
                    .unwrap(),
                expected_vals[i - 2]
            );
        }
    }

    #[test]
    fn function_call_varargs() {
        let mut vm = get_vm_for(
            "function g(...)
                 return 1, ...
             end
             function f(...)
                 return ..., 3
             end
             x, y, z, w = g(0, 1, 2)
             w1, w2 = f(11, 12, 13), 10"
                .to_string(),
        );
        vm.eval().unwrap();
        assert!(vm.top == 0);
        let expected_vals = vec![
            LuaVal::from(1),
            LuaVal::from(0),
            LuaVal::from(1),
            LuaVal::from(2),
            LuaVal::from(11),
            LuaVal::from(10),
        ];
        let strs = vec!["x", "y", "z", "w", "w1", "w2"];
        for i in 2..(strs.len() + 2) {
            assert_eq!(
                vm.env
                    .get_attr(&LuaVal::from(String::from(strs[i - 2])))
                    .unwrap(),
                expected_vals[i - 2]
            );
        }
    }

    #[test]
    fn function_call_misc() {
        let mut vm = get_vm_for(
            "function g(...)
                 return f(10, ...)
             end
             function f(...)
                 return 20, ...
             end
             function h(a, b, c, d)
                 return d, c, b, a
             end
             x, y, z, w, w1 = h(g(2, 3))"
                .to_string(),
        );
        vm.eval().unwrap();
        assert!(vm.top == 0);
        let expected_vals = vec![
            LuaVal::from(3),
            LuaVal::from(2),
            LuaVal::from(10),
            LuaVal::from(20),
            LuaVal::new(),
        ];
        let strs = vec!["x", "y", "z", "w", "w1"];
        for i in 3..(strs.len() + 3) {
            assert_eq!(
                vm.env
                    .get_attr(&LuaVal::from(String::from(strs[i - 3])))
                    .unwrap(),
                expected_vals[i - 3]
            );
        }
    }
}
