use crate::{errors::LuaError, lua_values::LuaVal, stdlib::StdFunction, Vm};
use gc::{Finalize, Gc, Trace};
use luacompiler::bytecode::Function;
use std::cell::Cell;

impl Finalize for Box<LuaClosure> {}
unsafe impl Trace for Box<LuaClosure> {
    unsafe fn trace(&self) {
        (**self).trace();
    }

    unsafe fn root(&self) {
        (**self).root();
    }

    unsafe fn unroot(&self) {
        (**self).unroot();
    }

    fn finalize_glue(&self) {
        (**self).finalize();
        (**self).finalize_glue();
    }
}

/// Represents a closure in Lua.
#[derive(Trace, Finalize)]
pub struct UserFunction {
    index: usize,
    reg_count: usize,
    param_count: usize,
    #[unsafe_ignore_trace]
    ret_vals: Cell<usize>,
    upvals: Vec<Gc<LuaVal>>,
}

impl UserFunction {
    pub fn new(
        index: usize,
        reg_count: usize,
        param_count: usize,
        upvals: Vec<Gc<LuaVal>>,
    ) -> UserFunction {
        UserFunction {
            index,
            reg_count,
            param_count,
            ret_vals: Cell::new(0),
            upvals,
        }
    }
}

impl LuaClosure for UserFunction {
    fn index(&self) -> usize {
        self.index
    }

    fn reg_count(&self) -> usize {
        self.reg_count
    }

    fn param_count(&self) -> usize {
        self.param_count
    }

    fn call(&self, vm: &mut Vm) -> Result<(), LuaError> {
        vm.eval()
    }

    fn ret_vals(&self) -> usize {
        self.ret_vals.get()
    }

    fn set_ret_vals(&self, vals: usize) {
        self.ret_vals.set(vals);
    }

    fn get_upval(&self, i: usize) -> Result<&Gc<LuaVal>, LuaError> {
        self.upvals.get(i).ok_or(LuaError::Error(format!(
            "Upvalue with index {} doesn't exist!",
            i
        )))
    }

    fn set_upval(&self, _: usize, _: LuaVal) -> Result<(), LuaError> {
        Err(LuaError::Error(
            "SetUpVal doesn't work on BuiltinFunctions.".to_string(),
        ))
    }
}

#[derive(Trace, Finalize)]
pub struct BuiltinFunction {
    #[unsafe_ignore_trace]
    handler: fn(&mut Vm) -> Result<(), LuaError>,
    #[unsafe_ignore_trace]
    ret_vals: Cell<usize>,
}

impl LuaClosure for BuiltinFunction {
    fn index(&self) -> usize {
        0
    }

    fn reg_count(&self) -> usize {
        0
    }

    fn param_count(&self) -> usize {
        0
    }

    fn call(&self, vm: &mut Vm) -> Result<(), LuaError> {
        (self.handler)(vm)
    }

    fn ret_vals(&self) -> usize {
        self.ret_vals.get()
    }

    fn set_ret_vals(&self, vals: usize) {
        self.ret_vals.set(vals);
    }

    fn get_upval(&self, _: usize) -> Result<&Gc<LuaVal>, LuaError> {
        Err(LuaError::Error(
            "GetUpVal doesn't work on BuiltinFunctions.".to_string(),
        ))
    }

    fn set_upval(&self, _: usize, _: LuaVal) -> Result<(), LuaError> {
        Err(LuaError::Error(
            "SetUpVal doesn't work on BuiltinFunctions.".to_string(),
        ))
    }
}

pub fn from_stdfunction(func: &StdFunction) -> Gc<Box<LuaClosure>> {
    Gc::new(Box::new(BuiltinFunction {
        handler: func.handler(),
        ret_vals: Cell::new(0),
    }))
}

pub fn from_function(func: &Function) -> Gc<Box<LuaClosure>> {
    Gc::new(Box::new(UserFunction {
        index: func.index(),
        reg_count: func.reg_count(),
        param_count: func.param_count(),
        ret_vals: Cell::new(0),
        upvals: vec![],
    }))
}

pub trait LuaClosure: Trace + Finalize {
    fn index(&self) -> usize;
    fn reg_count(&self) -> usize;
    fn param_count(&self) -> usize;
    fn call(&self, vm: &mut Vm) -> Result<(), LuaError>;
    fn ret_vals(&self) -> usize;
    fn set_ret_vals(&self, vals: usize);
    fn get_upval(&self, i: usize) -> Result<&Gc<LuaVal>, LuaError>;
    fn set_upval(&self, i: usize, value: LuaVal) -> Result<(), LuaError>;
}
