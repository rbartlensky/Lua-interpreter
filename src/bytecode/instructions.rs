use std::cmp::{Ord, Ordering, PartialOrd};
use std::fmt::{Display, Formatter, Result};
use bytecode::LuaBytecode;

/// This enum represents all possible Lua values.
#[derive(Clone, Debug)]
pub enum Value {
    Nil,
    Boolean(bool),
    Number(f64),
    Str(String),
}

impl PartialEq for Value {
    fn eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Nil, Value::Nil) => true,
            (Value::Boolean(l), Value::Boolean(r)) => l == r,
            (Value::Number(l), Value::Number(r)) =>
                l.partial_cmp(r).unwrap() == Ordering::Equal,
            (Value::Str(l), Value::Str(r)) => l == r,
            (_, _) => false
        }
    }
}

impl Eq for Value {}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            Value::Nil => write!(f, "Nil"),
            Value::Boolean(b) => write!(f, "{}", b.to_string()),
            Value::Number(float) => write!(f, "{}", float),
            Value::Str(ref content) => write!(f, "\"{}\"", content),
        }
    }
}

#[derive(Clone)]
pub struct Reg {
    id: usize,
    value: Value
}

impl Reg {
    pub fn new(id: usize) -> Reg {
        Reg { id, value: Value::Nil }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn get_value(&self) -> &Value {
        &self.value
    }

    pub fn set_value(&mut self, value: Value) {
        self.value = value;
    }
}

impl Display for Reg {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{} = {}", self.id, self.value)
    }
}

impl PartialEq for Reg {
    fn eq(&self, other: &Reg) -> bool {
        self.id == other.id
    }
}

impl Eq for Reg {}

impl PartialOrd for Reg {
    fn partial_cmp(&self, other: &Reg) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Reg {
    fn cmp(&self, other: &Reg) -> Ordering {
        self.id.cmp(&other.id)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Val {
    LuaValue(Value),
    Register(usize)
}

impl Val {
    pub fn get_value<'a>(&'a self, bytecode: &'a LuaBytecode) -> &'a Value {
        match *self {
            Val::LuaValue(ref value) => &value,
            Val::Register(reg) => bytecode.get_value(reg)
        }
    }
}

impl Display for Val {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            Val::LuaValue(ref value) => write!(f, "{}", value),
            Val::Register(reg) => write!(f, "${}", reg)
        }
    }
}

/// The instructions supported by the LuaBytecode
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Instr {
    Mov(usize, Val),
    Add(usize, Val, Val),
    Sub(usize, Val, Val),
    Mul(usize, Val, Val),
    Div(usize, Val, Val),
    Mod(usize, Val, Val)
}

impl Display for Instr {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            Instr::Mov(reg, ref val) => write!(f, "(mov ${} {})", reg, val),
            Instr::Add(reg, ref lhs, ref rhs) =>
                write!(f, "(add ${} {} {})", reg, lhs, rhs),
            Instr::Sub(reg, ref lhs, ref rhs) =>
                write!(f, "(sub ${} {} {})", reg, lhs, rhs),
            Instr::Mul(reg, ref lhs, ref rhs) =>
                write!(f, "(mul ${} {} {})", reg, lhs, rhs),
            Instr::Div(reg, ref lhs, ref rhs) =>
                write!(f, "(div ${} {} {})", reg, lhs, rhs),
            Instr::Mod(reg, ref lhs, ref rhs) =>
                write!(f, "(mod ${} {} {})", reg, lhs, rhs),
        }
    }
}
