use bytecode::LuaBytecode;
use bytecode::instructions::Value;
use bytecode::instructions::Val;
use bytecode::instructions::Instr::*;

/// Represents a `LuaBytecode` interpreter.
pub struct Interpreter {
    bytecode: LuaBytecode
}

impl Interpreter {
    /// Create a new interpreter for the given bytecode.
    pub fn new(bytecode: LuaBytecode) -> Interpreter {
        Interpreter{ bytecode }
    }

    /// Evaluate the program.
    pub fn eval(&mut self) {
        let mut pc = 0;
        let len = self.bytecode.instrs_len();
        while pc < len {
            let instr = self.bytecode.get_instr(pc);
            match instr {
                Mov(reg, ref value) => {
                    let val = value.get_value(&self.bytecode).clone();
                    self.bytecode.set_value(reg, val);
                },
                Add(reg, ref lhs, ref rhs) => {
                    let res = self.eval_arithmetic_op(&lhs, &rhs, lua_add);
                    self.bytecode.set_value(reg, res);
                },
                Sub(reg, ref lhs, ref rhs) => {
                    let res = self.eval_arithmetic_op(&lhs, &rhs, lua_sub);
                    self.bytecode.set_value(reg, res);
                },
                Mul(reg, ref lhs, ref rhs) => {
                    let res = self.eval_arithmetic_op(&lhs, &rhs, lua_mul);
                    self.bytecode.set_value(reg, res);
                },
                Div(reg, ref lhs, ref rhs) => {
                    let res = self.eval_arithmetic_op(&lhs, &rhs, lua_div);
                    self.bytecode.set_value(reg, res);
                },
                Mod(reg, ref lhs, ref rhs) => {
                    let res = self.eval_arithmetic_op(&lhs, &rhs, lua_mod);
                    self.bytecode.set_value(reg, res);
                },
            }
            pc += 1;
        }
    }

    /// Apply <op> to the given `Value`s.
    fn eval_arithmetic_op(&self, lhs: &Val, rhs: &Val,
                          op: fn(f64, f64) -> Value) -> Value {
        let lhs = lhs.get_value(&self.bytecode);
        let rhs = rhs.get_value(&self.bytecode);
        match (lhs, rhs) {
            (Value::Number(l), Value::Number(r)) => op(*l, *r),
            (_, _) =>
                panic!("Unable to perform arithmetic on {} and {}", lhs, rhs)
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
    use bytecode::instructions::Reg;

    #[test]
    fn interpreter_works_correctly() {
        let mut regs = vec![];
        for i in 0..6 {
            regs.push(Reg::new(i));
        }
        let instrs = vec![
            Mov(regs[0].id(), Val::LuaValue(Value::Number(0.0))),
            Add(regs[1].id(), Val::LuaValue(Value::Number(1.0)),
                Val::LuaValue(Value::Number(1.0))),
            Sub(regs[2].id(), Val::LuaValue(Value::Number(1.0)),
                Val::LuaValue(Value::Number(1.0))),
            Mul(regs[3].id(), Val::LuaValue(Value::Number(1.0)),
                Val::LuaValue(Value::Number(2.0))),
            Div(regs[4].id(), Val::LuaValue(Value::Number(2.0)),
                Val::LuaValue(Value::Number(2.0))),
            Mod(regs[5].id(), Val::LuaValue(Value::Number(3.0)),
                Val::LuaValue(Value::Number(2.0))),
        ];
        let bytecode = LuaBytecode::new(instrs, regs.clone());
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
        for (i, ref r) in regs.iter().enumerate() {
            assert_eq!(*interpreter.bytecode.get_value(r.id()), expected[i]);
        }
    }
}
