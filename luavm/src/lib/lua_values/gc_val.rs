use errors::LuaError;
use gc::{Finalize, Trace};
use lua_values::LuaVal;
use Vm;

impl Finalize for Box<GcVal> {}
unsafe impl Trace for Box<GcVal> {
    #[inline]
    unsafe fn trace(&self) {
        (**self).trace();
    }

    #[inline]
    unsafe fn root(&self) {
        (**self).root();
    }

    #[inline]
    unsafe fn unroot(&self) {
        (**self).unroot();
    }

    #[inline]
    fn finalize_glue(&self) {
        (**self).finalize();
        (**self).finalize_glue();
    }
}

pub trait GcVal: Trace + Finalize {
    fn is_table(&self) -> bool {
        false
    }

    fn is_closure(&self) -> bool {
        false
    }

    #[inline]
    fn get_attr(&self, _attr: &LuaVal) -> Result<LuaVal, LuaError> {
        Err(LuaError::GetAttrErr)
    }

    #[inline]
    fn set_attr(&self, _attr: LuaVal, _val: LuaVal) -> Result<(), LuaError> {
        Err(LuaError::SetAttrErr)
    }

    #[inline]
    fn index(&self) -> Result<usize, LuaError> {
        Err(LuaError::NotAClosure)
    }

    #[inline]
    fn reg_count(&self) -> Result<usize, LuaError> {
        Err(LuaError::NotAClosure)
    }

    #[inline]
    fn param_count(&self) -> Result<usize, LuaError> {
        Err(LuaError::NotAClosure)
    }

    #[inline]
    fn call(&self, _vm: &mut Vm) -> Result<(), LuaError> {
        Err(LuaError::NotAClosure)
    }

    #[inline]
    fn ret_vals(&self) -> Result<usize, LuaError> {
        Err(LuaError::NotAClosure)
    }

    #[inline]
    fn set_ret_vals(&self, _vals: usize) -> Result<(), LuaError> {
        Err(LuaError::NotAClosure)
    }

    #[inline]
    fn inc_ret_vals(&self, _amount: usize) -> Result<(), LuaError> {
        Err(LuaError::NotAClosure)
    }

    #[inline]
    fn get_upval(&self, _i: usize) -> Result<LuaVal, LuaError> {
        Err(LuaError::NotAClosure)
    }

    #[inline]
    fn set_upval(&self, _i: usize, _value: LuaVal) -> Result<(), LuaError> {
        Err(LuaError::NotAClosure)
    }
}
