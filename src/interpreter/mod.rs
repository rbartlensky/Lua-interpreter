mod register;

use bytecode::LuaBytecode;
use bytecode::instructions::Value;
use bytecode::instructions::Val;
use bytecode::instructions::Instr::*;
use self::register::Reg;

/// Represents a `LuaBytecode` interpreter.
pub struct Interpreter {
    bytecode: LuaBytecode,
    registers: Vec<Reg>
}

impl Interpreter {
    /// Create a new interpreter for the given bytecode.
    pub fn new(bytecode: LuaBytecode) -> Interpreter {
        let regs = bytecode.reg_count();
        let mut registers = Vec::with_capacity(regs);
        for _ in 0..regs {
            registers.push(Reg::new());
        }
        Interpreter{ bytecode, registers }
    }

    /// Evaluate the program.
    pub fn eval(&mut self) {
        let mut pc = 0;
        let len = self.bytecode.instrs_len();
        while pc < len {
            let instr = self.bytecode.get_instr(pc);
            match *instr {
                Mov(reg, ref value) => {
                    let val = self.get_value(value).clone();
                    self.registers[reg].set_value(val);
                },
                Add(reg, ref lhs, ref rhs) => {
                    let res = self.eval_arithmetic_op(&lhs, &rhs, lua_add);
                    self.registers[reg].set_value(res);
                },
                Sub(reg, ref lhs, ref rhs) => {
                    let res = self.eval_arithmetic_op(&lhs, &rhs, lua_sub);
                    self.registers[reg].set_value(res);
                },
                Mul(reg, ref lhs, ref rhs) => {
                    let res = self.eval_arithmetic_op(&lhs, &rhs, lua_mul);
                    self.registers[reg].set_value(res);
                },
                Div(reg, ref lhs, ref rhs) => {
                    let res = self.eval_arithmetic_op(&lhs, &rhs, lua_div);
                    self.registers[reg].set_value(res);
                },
                Mod(reg, ref lhs, ref rhs) => {
                    let res = self.eval_arithmetic_op(&lhs, &rhs, lua_mod);
                    self.registers[reg].set_value(res);
                }
            }
            pc += 1;
        }
    }

    /// Apply <op> to the given `Value`s.
    fn eval_arithmetic_op(&self, lhs: &Val, rhs: &Val,
                          op: fn(f64, f64) -> Value) -> Value {
        let lhs = self.get_value(lhs);
        let rhs = self.get_value(rhs);
        match (lhs, rhs) {
            (Value::Number(l), Value::Number(r)) => op(*l, *r),
            (_, _) =>
                panic!("Unable to perform arithmetic on {} and {}", lhs, rhs)
        }
    }

    fn get_value<'a>(&'a self, val: &'a Val) -> &'a Value {
        match val {
            Val::LuaValue(ref value) => &value,
            Val::Register(reg) => self.registers[*reg].get_value()
        }
    }
}

// Functions that are used in the interpreter in order to minimise duplication
fn lua_add(lhs: f64, rhs: f64) -> Value { Value::Number(lhs + rhs) }
fn lua_sub(lhs: f64, rhs: f64) -> Value { Value::Number(lhs - rhs) }
fn lua_mul(lhs: f64, rhs: f64) -> Value { Value::Number(lhs * rhs) }
fn lua_div(lhs: f64, rhs: f64) -> Value { Value::Number(lhs / rhs) }
fn lua_mod(lhs: f64, rhs: f64) -> Value { Value::Number(lhs % rhs) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpreter_works_correctly() {
        let mut regs = vec![];
        for i in 0..6 {
            regs.push(i);
        }
        let instrs = vec![
            Mov(regs[0], Val::LuaValue(Value::Number(0.0))),
            Add(regs[1], Val::LuaValue(Value::Number(1.0)),
                Val::LuaValue(Value::Number(1.0))),
            Sub(regs[2], Val::LuaValue(Value::Number(1.0)),
                Val::LuaValue(Value::Number(1.0))),
            Mul(regs[3], Val::LuaValue(Value::Number(1.0)),
                Val::LuaValue(Value::Number(2.0))),
            Div(regs[4], Val::LuaValue(Value::Number(2.0)),
                Val::LuaValue(Value::Number(2.0))),
            Mod(regs[5], Val::LuaValue(Value::Number(3.0)),
                Val::LuaValue(Value::Number(2.0))),
        ];
        let bytecode = LuaBytecode::new(instrs, regs.len());
        let expected = vec![
            Value::Number(0.0),
            Value::Number(2.0),
            Value::Number(0.0),
            Value::Number(2.0),
            Value::Number(1.0),
            Value::Number(1.0),
        ];

        let mut interpreter = Interpreter::new(bytecode);
        interpreter.eval();
        for (i, r) in regs.iter().enumerate() {
            assert_eq!(*interpreter.registers[*r].get_value(), expected[i]);
        }
    }
}
