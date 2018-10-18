mod arithmetic_operators;
mod register;

use bytecode::LuaBytecode;
use bytecode::instructions::Value;
use bytecode::instructions::Val;
use bytecode::instructions::Instr::*;
use self::register::Reg;
use self::arithmetic_operators::*;

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
                    let res = add(self.get_value(lhs), self.get_value(rhs));
                    self.registers[reg].set_value(res);
                },
                Sub(reg, ref lhs, ref rhs) => {
                    let res = sub(self.get_value(lhs), self.get_value(rhs));
                    self.registers[reg].set_value(res);
                },
                Mul(reg, ref lhs, ref rhs) => {
                    let res = mul(self.get_value(lhs), self.get_value(rhs));
                    self.registers[reg].set_value(res);
                },
                Div(reg, ref lhs, ref rhs) => {
                    let res = div(self.get_value(lhs), self.get_value(rhs));
                    self.registers[reg].set_value(res);
                },
                Mod(reg, ref lhs, ref rhs) => {
                    let res = modulus(self.get_value(lhs), self.get_value(rhs));
                    self.registers[reg].set_value(res);
                },
                FDiv(reg, ref lhs, ref rhs) => {
                    let res = fdiv(self.get_value(lhs), self.get_value(rhs));
                    self.registers[reg].set_value(res);
                },
                Exp(reg, ref lhs, ref rhs) => {
                    let res = exp(self.get_value(lhs), self.get_value(rhs));
                    self.registers[reg].set_value(res);
                }
            }
            pc += 1;
        }
    }

    fn get_value<'a>(&'a self, val: &'a Val) -> &'a Value {
        match val {
            Val::LuaValue(ref value) => &value,
            Val::Register(reg) => self.registers[*reg].get_value()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpreter_works_correctly() {
        let mut regs = vec![];
        for i in 0..8 {
            regs.push(i);
        }
        let instrs = vec![
            Mov(regs[0], Val::LuaValue(Value::Float(0.0))),
            Add(regs[1], Val::LuaValue(Value::Float(1.0)),
                Val::LuaValue(Value::Float(1.0))),
            Sub(regs[2], Val::LuaValue(Value::Float(1.0)),
                Val::LuaValue(Value::Float(1.0))),
            Mul(regs[3], Val::LuaValue(Value::Float(1.0)),
                Val::LuaValue(Value::Float(2.0))),
            Div(regs[4], Val::LuaValue(Value::Float(2.0)),
                Val::LuaValue(Value::Float(2.0))),
            Mod(regs[5], Val::LuaValue(Value::Float(3.0)),
                Val::LuaValue(Value::Float(2.0))),
            FDiv(regs[6], Val::LuaValue(Value::Float(3.0)),
                 Val::LuaValue(Value::Float(2.0))),
            Exp(regs[7], Val::LuaValue(Value::Float(1.0)),
                 Val::LuaValue(Value::Float(2.0)))
        ];
        let bytecode = LuaBytecode::new(instrs, regs.len());
        let expected = vec![
            Value::Float(0.0),
            Value::Float(2.0),
            Value::Float(0.0),
            Value::Float(2.0),
            Value::Float(1.0),
            Value::Float(1.0),
            Value::Float(1.0),
            Value::Float(1.0)
        ];

        let mut interpreter = Interpreter::new(bytecode);
        interpreter.eval();
        for (i, r) in regs.iter().enumerate() {
            assert_eq!(*interpreter.registers[*r].get_value(), expected[i]);
        }
    }
}
