use crate::{
    errors::LuaError,
    lua_values::{gc_val::GcVal, LuaVal},
    stdlib::StdBcFunc,
    Vm,
};
use gc::GcCell;
use luacompiler::bytecode::BcFunc;
use std::cell::Cell;

/// Represents a closure in Lua.
#[derive(Trace, Finalize)]
pub struct UserBcFunc {
    index: usize,
    reg_count: usize,
    param_count: usize,
    #[unsafe_ignore_trace]
    ret_vals: Cell<usize>,
    upvals: GcCell<Vec<LuaVal>>,
}

impl UserBcFunc {
    pub fn new(index: usize, reg_count: usize, param_count: usize) -> UserBcFunc {
        UserBcFunc {
            index,
            reg_count,
            param_count,
            ret_vals: Cell::new(0),
            upvals: GcCell::new(vec![]),
        }
    }

    pub fn with_upvals(
        index: usize,
        reg_count: usize,
        param_count: usize,
        upvals: Vec<LuaVal>,
    ) -> UserBcFunc {
        UserBcFunc {
            index,
            reg_count,
            param_count,
            ret_vals: Cell::new(0),
            upvals: GcCell::new(upvals),
        }
    }
}

impl GcVal for UserBcFunc {
    fn is_closure(&self) -> bool {
        true
    }

    fn index(&self) -> Result<usize, LuaError> {
        Ok(self.index)
    }

    fn reg_count(&self) -> Result<usize, LuaError> {
        Ok(self.reg_count)
    }

    fn param_count(&self) -> Result<usize, LuaError> {
        Ok(self.param_count)
    }

    fn call(&self, vm: &mut Vm) -> Result<(), LuaError> {
        vm.eval()
    }

    fn ret_vals(&self) -> Result<usize, LuaError> {
        Ok(self.ret_vals.get())
    }

    fn set_ret_vals(&self, vals: usize) -> Result<(), LuaError> {
        self.ret_vals.set(vals);
        Ok(())
    }

    fn inc_ret_vals(&self, amount: usize) -> Result<(), LuaError> {
        self.ret_vals.set(self.ret_vals.get() + amount);
        Ok(())
    }

    fn set_upvals(&self, upvals: Vec<LuaVal>) -> Result<(), LuaError> {
        *self.upvals.borrow_mut() = upvals;
        Ok(())
    }

    fn get_upval(&self, i: usize) -> Result<LuaVal, LuaError> {
        self.upvals
            .borrow()
            .get(i)
            .map(|v| v.clone())
            .ok_or(LuaError::Error(format!(
                "Upvalue with index {} doesn't exist!",
                i
            )))
    }

    fn set_upval(&self, i: usize, val: LuaVal) -> Result<(), LuaError> {
        self.upvals.borrow_mut()[i] = val;
        Ok(())
    }
}

#[derive(Trace, Finalize)]
pub struct BuiltinBcFunc {
    #[unsafe_ignore_trace]
    handler: fn(&mut Vm) -> Result<(), LuaError>,
    #[unsafe_ignore_trace]
    ret_vals: Cell<usize>,
}

impl GcVal for BuiltinBcFunc {
    fn is_closure(&self) -> bool {
        true
    }

    fn index(&self) -> Result<usize, LuaError> {
        Ok(0)
    }

    fn reg_count(&self) -> Result<usize, LuaError> {
        Ok(0)
    }

    fn param_count(&self) -> Result<usize, LuaError> {
        Ok(0)
    }

    fn call(&self, vm: &mut Vm) -> Result<(), LuaError> {
        (self.handler)(vm)
    }

    fn ret_vals(&self) -> Result<usize, LuaError> {
        Ok(self.ret_vals.get())
    }

    fn set_ret_vals(&self, vals: usize) -> Result<(), LuaError> {
        self.ret_vals.set(vals);
        Ok(())
    }

    fn inc_ret_vals(&self, amount: usize) -> Result<(), LuaError> {
        self.ret_vals.set(self.ret_vals.get() + amount);
        Ok(())
    }

    fn set_upvals(&self, _upvals: Vec<LuaVal>) -> Result<(), LuaError> {
        Err(LuaError::Error(
            "SetUpvals doesn't work on BuiltinBcFuncs.".to_string(),
        ))
    }

    fn get_upval(&self, _: usize) -> Result<LuaVal, LuaError> {
        Err(LuaError::Error(
            "GetUpVal doesn't work on BuiltinBcFuncs.".to_string(),
        ))
    }

    fn set_upval(&self, _: usize, _: LuaVal) -> Result<(), LuaError> {
        Err(LuaError::Error(
            "SetUpVal doesn't work on BuiltinBcFuncs.".to_string(),
        ))
    }
}

pub fn from_stdfunction(func: &StdBcFunc) -> Box<dyn GcVal> {
    Box::new(BuiltinBcFunc {
        handler: func.handler(),
        ret_vals: Cell::new(0),
    })
}

pub fn from_function(func: &BcFunc) -> Box<dyn GcVal> {
    let mut upvals = Vec::with_capacity(func.upvals_count());
    for _ in 0..func.upvals_count() {
        upvals.push(LuaVal::new());
    }
    Box::new(UserBcFunc {
        index: func.index(),
        reg_count: func.reg_count(),
        param_count: func.param_count(),
        ret_vals: Cell::new(0),
        upvals: GcCell::new(upvals),
    })
}
