use std::cmp::{Ordering, PartialOrd};
use std::fmt::{Display, Formatter, Result};

/// This enum represents all possible Lua values.
#[derive(Clone, Debug)]
pub enum Value {
    Nil,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    Str(String),
}

impl Value {
    /// Check whether the Value can be converted into a Float.
    pub fn is_float(&self) -> bool {
        match self {
            Value::Float(_) | Value::Str(_) => true,
            _ => false
        }
    }

    /// Convert the Value to an f64. This conversion returns Some when the value is
    /// either an Integer, a Float or a Str.
    /// In Lua, string are always converted to floats when they are used in
    /// arithmetic expressions.
    pub fn to_float(&self) -> Option<f64> {
        match self {
            Value::Integer(value) => Some(*value as f64),
            Value::Float(value) => Some(*value),
            Value::Str(value) => value.parse().ok(),
            _ => None
        }
    }

    /// Convert the Value to an i64. This conversion returns Some when the value is
    /// an Integer.
    pub fn to_int(&self) -> Option<i64> {
        match self {
            Value::Integer(value) => Some(*value),
            _ => None
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Nil, Value::Nil) => true,
            (Value::Boolean(l), Value::Boolean(r)) => l == r,
            (Value::Integer(l), Value::Integer(r)) => l == r,
            (Value::Float(l), Value::Float(r)) =>
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
            Value::Integer(int) => write!(f, "{}", int),
            Value::Float(float) => write!(f, "{}", float),
            Value::Str(ref content) => write!(f, "\"{}\"", content),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Val {
    LuaValue(Value),
    Register(usize)
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
    Mod(usize, Val, Val),
    FDiv(usize, Val, Val),
    Exp(usize, Val, Val)
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
            Instr::FDiv(reg, ref lhs, ref rhs) =>
                write!(f, "(fdiv ${} {} {})", reg, lhs, rhs),
            Instr::Exp(reg, ref lhs, ref rhs) =>
                write!(f, "(exp ${} {} {})", reg, lhs, rhs)
        }
    }
}
