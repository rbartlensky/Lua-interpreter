use crate::{stdlib::StdFunction, Vm};
use gc::{Finalize, Gc, GcCell, Trace};
use luacompiler::bytecode::Function;

impl Finalize for Box<LuaClosure> {}
unsafe impl Trace for Box<LuaClosure> {
    custom_trace!(_this, {});
}

/// Represents a closure in Lua.
#[derive(Trace, Finalize)]
pub struct UserFunction {
    index: usize,
    reg_count: usize,
    param_count: usize,
    args_count: GcCell<usize>,
    args_start: GcCell<usize>,
}

impl UserFunction {
    pub fn new(index: usize, reg_count: usize, param_count: usize) -> UserFunction {
        UserFunction {
            index,
            reg_count,
            param_count,
            args_count: GcCell::new(0),
            args_start: GcCell::new(0),
        }
    }
}

impl LuaClosure for UserFunction {
    fn index(&self) -> usize {
        self.index
    }

    fn args_count(&self) -> usize {
        self.args_count.borrow().clone()
    }

    fn set_args_count(&self, count: usize) {
        *self.args_count.borrow_mut() = count;
    }

    fn args_start(&self) -> usize {
        self.args_start.borrow().clone()
    }

    fn set_args_start(&self, count: usize) {
        *self.args_start.borrow_mut() = count;
    }

    fn reg_count(&self) -> usize {
        self.reg_count
    }

    fn param_count(&self) -> usize {
        self.param_count
    }

    fn call(&self, vm: &mut Vm) {
        vm.eval();
    }
}

#[derive(Trace, Finalize)]
pub struct BuiltinFunction {
    #[unsafe_ignore_trace]
    handler: fn(&mut Vm),
    param_count: usize,
    args_count: GcCell<usize>,
    args_start: GcCell<usize>,
}

impl LuaClosure for BuiltinFunction {
    fn index(&self) -> usize {
        0
    }

    fn args_count(&self) -> usize {
        self.args_count.borrow().clone()
    }

    fn set_args_count(&self, count: usize) {
        *self.args_count.borrow_mut() = count;
    }

    fn args_start(&self) -> usize {
        self.args_start.borrow().clone()
    }

    fn set_args_start(&self, count: usize) {
        *self.args_start.borrow_mut() = count;
    }

    fn reg_count(&self) -> usize {
        // builtin functions might use the _ENV register, which is register 0, so their
        // reg_count is 1
        1
    }

    fn param_count(&self) -> usize {
        self.param_count
    }

    fn call(&self, vm: &mut Vm) {
        (self.handler)(vm);
    }
}

pub fn from_stdfunction(func: &StdFunction) -> Gc<Box<LuaClosure>> {
    Gc::new(Box::new(BuiltinFunction {
        handler: func.handler(),
        param_count: func.param_count(),
        args_count: GcCell::new(0),
        args_start: GcCell::new(0),
    }))
}

pub fn from_function(func: &Function) -> Gc<Box<LuaClosure>> {
    Gc::new(Box::new(UserFunction {
        index: func.index(),
        reg_count: func.reg_count(),
        param_count: func.param_count(),
        args_count: GcCell::new(0),
        args_start: GcCell::new(0),
    }))
}

pub trait LuaClosure: Trace + Finalize {
    fn index(&self) -> usize;
    fn args_count(&self) -> usize;
    fn set_args_count(&self, count: usize);
    fn args_start(&self) -> usize;
    fn set_args_start(&self, count: usize);
    fn reg_count(&self) -> usize;
    fn param_count(&self) -> usize;
    fn call(&self, vm: &mut Vm);
}
