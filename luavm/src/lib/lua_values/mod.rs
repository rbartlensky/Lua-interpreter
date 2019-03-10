pub mod gc_val;
pub mod lua_closure;
mod lua_obj;
pub mod lua_table;
mod tagging;

use self::gc_val::GcVal;
use self::{lua_closure::*, lua_obj::*, tagging::*};
use crate::stdlib::StdFunction;
use errors::LuaError;
use gc::finalizer_safe;
use gc::{gc::GcBox, Finalize, Trace};
use ieee754::Ieee754;
use luacompiler::bytecode::Function;
use std::cell::Cell;
use std::{
    cmp::Ordering,
    fmt,
    fmt::{Display, Formatter},
    hash::{Hash, Hasher},
    mem::transmute,
};
use Vm;

/// Represents a value in Lua.
#[derive(Debug)]
pub struct LuaVal {
    val: Cell<usize>,
}

impl Finalize for LuaVal {}
unsafe impl Trace for LuaVal {
    #[inline]
    unsafe fn trace(&self) {
        match self.kind() {
            LuaValKind::Gc | LuaValKind::GcRoot => self.inner().trace_inner(),
            _ => {}
        }
    }

    #[inline]
    unsafe fn root(&self) {
        match self.kind() {
            LuaValKind::Gc | LuaValKind::GcRoot => {
                assert!(!self.rooted(), "Can't double-root a Gc<T>");
                self.inner().root_inner();
                self.set_root();
            }
            _ => {}
        }
    }

    #[inline]
    unsafe fn unroot(&self) {
        match self.kind() {
            LuaValKind::Gc | LuaValKind::GcRoot => {
                assert!(self.rooted(), "Can't double-unroot a Gc<T>");
                self.inner().unroot_inner();
                self.clear_root();
            }
            _ => {}
        }
    }

    #[inline]
    fn finalize_glue(&self) {
        Finalize::finalize(self);
    }
}

impl LuaVal {
    /// Create an empty LuaVal which is equivalent to Nil.
    pub fn new() -> LuaVal {
        LuaVal { val: Cell::new(0) }
    }

    /// Returns the type of the value store in the pointer.
    fn kind(&self) -> LuaValKind {
        LuaValKind::from(self.val.get())
    }

    /// Interprets the value as a pointer to a LuaObj, and returns a pointer to it.
    fn as_boxed(&self) -> *mut Box<LuaObj> {
        (LuaValKind::BOXED ^ self.val.get()) as *mut Box<LuaObj>
    }

    pub fn is_int(&self) -> bool {
        match self.kind() {
            LuaValKind::INT => true,
            LuaValKind::BOXED => unsafe { (*self.as_boxed()).is_int() },
            _ => false,
        }
    }

    pub fn is_float(&self) -> bool {
        match self.kind() {
            LuaValKind::FLOAT => true,
            LuaValKind::BOXED => unsafe { (*self.as_boxed()).is_float() },
            _ => false,
        }
    }

    pub fn is_number(&self) -> bool {
        match self.kind() {
            LuaValKind::INT | LuaValKind::FLOAT => true,
            LuaValKind::BOXED => unsafe { (*self.as_boxed()).is_number() },
            _ => false,
        }
    }

    pub fn is_string(&self) -> bool {
        match self.kind() {
            LuaValKind::BOXED => unsafe { (*self.as_boxed()).is_string() },
            _ => false,
        }
    }

    /// Gets the index of the underlying string in the constant table.
    pub fn get_constant_index(&self) -> Option<usize> {
        match self.kind() {
            LuaValKind::BOXED => unsafe { (*self.as_boxed()).get_constant_index() },
            _ => None,
        }
    }

    /// Returns true if the underlying type is either a float or a string.
    /// In Lua, if either of these two types are used in an arithmetic
    /// expression, then both arguments are converted to floats.
    fn is_aop_float(&self) -> bool {
        match self.kind() {
            LuaValKind::FLOAT => true,
            LuaValKind::BOXED => unsafe { (*self.as_boxed()).is_aop_float() },
            _ => false,
        }
    }

    /// Attempts to convert this value to a float.
    pub fn to_float(&self) -> Result<f64, LuaError> {
        unsafe {
            match self.kind() {
                // https://www.lua.org/manual/5.3/manual.html#3.4.3
                // The behaviour of `as f64` is the same as the conversion
                // from int to float described in the manual.
                LuaValKind::INT => Ok(((self.val.get() >> tagging::TAG_SHIFT) as i64) as f64),
                LuaValKind::FLOAT => {
                    Ok(transmute::<usize, f64>(LuaValKind::FLOAT ^ self.val.get()))
                }
                LuaValKind::BOXED => (*self.as_boxed()).to_float(),
                _ => Err(LuaError::FloatConversionErr),
            }
        }
    }

    /// Attempts to convert this value to an integer.
    pub fn to_int(&self) -> Result<i64, LuaError> {
        unsafe {
            match self.kind() {
                LuaValKind::INT => Ok((self.val.get() >> tagging::TAG_SHIFT) as i64),
                LuaValKind::BOXED => (*self.as_boxed()).to_int(),
                _ => Err(LuaError::IntConversionErr),
            }
        }
    }

    /// Attempts to convert this value to a string.
    pub fn to_string(&self) -> Result<String, LuaError> {
        match self.kind() {
            LuaValKind::BOOL => Ok(((self.val.get() >> tagging::TAG_SHIFT) != 0).to_string()),
            LuaValKind::INT => Ok(((self.val.get() >> tagging::TAG_SHIFT) as i64).to_string()),
            LuaValKind::FLOAT => {
                Ok(
                    (unsafe { transmute::<usize, f64>(LuaValKind::FLOAT ^ self.val.get()) })
                        .to_string(),
                )
            }
            LuaValKind::BOXED => unsafe { (*self.as_boxed()).to_string() },
            _ => Err(LuaError::StringConversionErr),
        }
    }

    pub fn to_bool(&self) -> bool {
        match self.kind() {
            LuaValKind::NIL => false,
            LuaValKind::BOOL => (self.val.get() >> tagging::TAG_SHIFT) != 0,
            _ => true,
        }
    }

    fn get_string_ref(&self) -> Option<&str> {
        match self.kind() {
            LuaValKind::BOXED => unsafe { (*self.as_boxed()).get_string_ref() },
            _ => None,
        }
    }

    /// Sets the given attribute to a given value.
    pub fn set_attr(&self, attr: LuaVal, val: LuaVal) -> Result<(), LuaError> {
        if let LuaValKind::Gc | LuaValKind::GcRoot = self.kind() {
            unsafe { (*gc_ptr(self.val.get())).value().set_attr(attr, val) }
        } else {
            Err(LuaError::SetAttrErr)
        }
    }

    /// Gets the value of the given attribute.
    pub fn get_attr(&self, attr: &LuaVal) -> Result<LuaVal, LuaError> {
        if let LuaValKind::Gc | LuaValKind::GcRoot = self.kind() {
            unsafe { (*gc_ptr(self.val.get())).value().get_attr(attr) }
        } else {
            Err(LuaError::GetAttrErr)
        }
    }

    pub fn add(&self, other: &LuaVal) -> Result<LuaVal, LuaError> {
        Ok(if self.is_aop_float() || other.is_aop_float() {
            LuaVal::from(self.to_float()? + other.to_float()?)
        } else {
            LuaVal::from(self.to_int()? + other.to_int()?)
        })
    }

    pub fn sub(&self, other: &LuaVal) -> Result<LuaVal, LuaError> {
        Ok(if self.is_aop_float() || other.is_aop_float() {
            LuaVal::from(self.to_float()? - other.to_float()?)
        } else {
            LuaVal::from(self.to_int()? - other.to_int()?)
        })
    }

    pub fn mul(&self, other: &LuaVal) -> Result<LuaVal, LuaError> {
        Ok(if self.is_aop_float() || other.is_aop_float() {
            LuaVal::from(self.to_float()? * other.to_float()?)
        } else {
            LuaVal::from(self.to_int()? * other.to_int()?)
        })
    }

    pub fn div(&self, other: &LuaVal) -> Result<LuaVal, LuaError> {
        Ok(if self.is_aop_float() || other.is_aop_float() {
            LuaVal::from(self.to_float()? / other.to_float()?)
        } else {
            LuaVal::from(self.to_int()? / other.to_int()?)
        })
    }

    pub fn modulus(&self, other: &LuaVal) -> Result<LuaVal, LuaError> {
        Ok(if self.is_aop_float() || other.is_aop_float() {
            LuaVal::from(self.to_float()? % other.to_float()?)
        } else {
            LuaVal::from(self.to_int()? % other.to_int()?)
        })
    }

    pub fn fdiv(&self, other: &LuaVal) -> Result<LuaVal, LuaError> {
        Ok(if self.is_aop_float() || other.is_aop_float() {
            LuaVal::from((self.to_float()? / other.to_float()?).floor())
        } else {
            LuaVal::from(self.to_int()? / other.to_int()?)
        })
    }

    pub fn exp(&self, other: &LuaVal) -> Result<LuaVal, LuaError> {
        Ok(LuaVal::from(self.to_float()?.powf(other.to_float()?)))
    }

    pub fn negate_number(&self) -> Result<LuaVal, LuaError> {
        if self.is_int() {
            Ok(LuaVal::from(-self.to_int()?))
        } else if self.is_float() {
            Ok(LuaVal::from(-self.to_float()?))
        } else {
            Err(LuaError::Error("Cannot negate non-numbers!".to_string()))
        }
    }

    pub fn index(&self) -> Result<usize, LuaError> {
        if self.kind() == LuaValKind::Gc || self.kind() == LuaValKind::GcRoot {
            unsafe { (*gc_ptr(self.val.get())).value().index() }
        } else {
            Err(LuaError::NotAClosure)
        }
    }

    pub fn reg_count(&self) -> Result<usize, LuaError> {
        if self.kind() == LuaValKind::Gc || self.kind() == LuaValKind::GcRoot {
            unsafe { (*gc_ptr(self.val.get())).value().reg_count() }
        } else {
            Err(LuaError::NotAClosure)
        }
    }

    pub fn param_count(&self) -> Result<usize, LuaError> {
        if self.kind() == LuaValKind::Gc || self.kind() == LuaValKind::GcRoot {
            unsafe { (*gc_ptr(self.val.get())).value().param_count() }
        } else {
            Err(LuaError::NotAClosure)
        }
    }

    pub fn call(&self, vm: &mut Vm) -> Result<(), LuaError> {
        if self.kind() == LuaValKind::Gc || self.kind() == LuaValKind::GcRoot {
            unsafe { (*gc_ptr(self.val.get())).value().call(vm) }
        } else {
            Err(LuaError::NotAClosure)
        }
    }

    pub fn ret_vals(&self) -> Result<usize, LuaError> {
        if self.kind() == LuaValKind::Gc || self.kind() == LuaValKind::GcRoot {
            unsafe { (*gc_ptr(self.val.get())).value().ret_vals() }
        } else {
            Err(LuaError::NotAClosure)
        }
    }

    pub fn set_ret_vals(&self, vals: usize) -> Result<(), LuaError> {
        if self.kind() == LuaValKind::Gc || self.kind() == LuaValKind::GcRoot {
            unsafe { (*gc_ptr(self.val.get())).value().set_ret_vals(vals) }
        } else {
            Err(LuaError::NotAClosure)
        }
    }

    pub fn inc_ret_vals(&self, amount: usize) -> Result<(), LuaError> {
        if self.kind() == LuaValKind::Gc || self.kind() == LuaValKind::GcRoot {
            unsafe { (*gc_ptr(self.val.get())).value().inc_ret_vals(amount) }
        } else {
            Err(LuaError::NotAClosure)
        }
    }

    pub fn set_upvals(&self, upvals: Vec<LuaVal>) -> Result<(), LuaError> {
        if self.kind() == LuaValKind::Gc || self.kind() == LuaValKind::GcRoot {
            unsafe { (*gc_ptr(self.val.get())).value().set_upvals(upvals) }
        } else {
            Err(LuaError::NotAClosure)
        }
    }

    pub fn get_upval(&self, i: usize) -> Result<LuaVal, LuaError> {
        if self.kind() == LuaValKind::Gc || self.kind() == LuaValKind::GcRoot {
            unsafe { (*gc_ptr(self.val.get())).value().get_upval(i) }
        } else {
            Err(LuaError::NotAClosure)
        }
    }

    pub fn set_upval(&self, i: usize, value: LuaVal) -> Result<(), LuaError> {
        if self.kind() == LuaValKind::Gc || self.kind() == LuaValKind::GcRoot {
            unsafe { (*gc_ptr(self.val.get())).value().set_upval(i, value) }
        } else {
            Err(LuaError::NotAClosure)
        }
    }

    // FOR GC ONLY!
    #[inline]
    fn rooted(&self) -> bool {
        LuaValKind::GcRoot == self.kind()
    }

    #[inline]
    unsafe fn set_root(&self) {
        self.val.set(set_tag(self.val.get(), LuaValKind::GcRoot))
    }

    #[inline]
    unsafe fn clear_root(&self) {
        self.val.set(set_tag(self.val.get(), LuaValKind::Gc))
    }

    #[inline]
    fn inner(&self) -> &GcBox<Box<dyn GcVal>> {
        assert!(finalizer_safe());
        unsafe { &*gc_ptr(self.val.get()) }
    }
}

impl PartialEq for LuaVal {
    fn eq(&self, other: &LuaVal) -> bool {
        if self.is_number() && other.is_number() {
            return self.to_float().unwrap() == other.to_float().unwrap();
        } else if self.is_string() && other.is_string() {
            return self.get_string_ref().unwrap() == other.get_string_ref().unwrap();
        } else if self.kind() == other.kind() {
            if self.kind() == LuaValKind::NIL {
                return true;
            } else if self.kind() == LuaValKind::Gc || self.kind() == LuaValKind::GcRoot {
                return gc_ptr(self.val.get()) == gc_ptr(other.val.get());
            } else if self.kind() == LuaValKind::BOOL {
                return self.val == other.val;
            }
        }
        false
    }
}

impl Eq for LuaVal {}

impl Hash for LuaVal {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self.kind() {
            LuaValKind::BOXED => {
                let val = unsafe { &*self.as_boxed() };
                if val.is_string() {
                    val.get_string_ref().unwrap().hash(state)
                } else if val.is_aop_float() {
                    let f = val.to_float().unwrap();
                    // NaN or Infs cannot be hashed, and Lua doesn't handle them either
                    if f.is_nan() || f.is_infinite() {
                        panic!("Tried to hash NaN or Inf")
                    }
                    unsafe { transmute::<f64, u64>(f) }.hash(state)
                } else {
                    val.to_int().unwrap().hash(state)
                }
            }
            _ => self.val.get().hash(state),
        }
    }
}

impl From<i64> for LuaVal {
    /// Create an integer LuaVal.
    fn from(int: i64) -> Self {
        let uint = int as usize;
        // if any of the 3 high-order bits are set, then the int is boxed
        let val = if uint & (tagging::MASK.rotate_right(tagging::TAG_SHIFT as u32)) != 0 {
            LuaValKind::BOXED ^ to_boxed(Box::new(LuaInt { v: int }))
        } else {
            LuaValKind::INT ^ (uint << tagging::TAG_SHIFT)
        };
        LuaVal {
            val: Cell::new(val),
        }
    }
}

impl From<f64> for LuaVal {
    /// Create a float LuaVal.
    fn from(float: f64) -> Self {
        let uint = unsafe { transmute::<f64, usize>(float) };
        // in most cases floats have one of the first three high order bits set
        // but the three low order bits are less likely to be set, therefore the
        // low order bits are used for the tag
        let val = if uint & tagging::MASK != 0 {
            LuaValKind::BOXED ^ to_boxed(Box::new(LuaFloat { v: float }))
        } else {
            LuaValKind::FLOAT ^ uint
        };
        LuaVal {
            val: Cell::new(val),
        }
    }
}

impl From<String> for LuaVal {
    /// Create a float LuaVal.
    fn from(string: String) -> Self {
        LuaVal {
            val: Cell::new(
                LuaValKind::BOXED
                    ^ to_boxed(Box::new(LuaString {
                        v: string,
                        const_index: None,
                    })),
            ),
        }
    }
}

impl From<(String, usize)> for LuaVal {
    /// Create a float LuaVal.
    fn from(string: (String, usize)) -> Self {
        LuaVal {
            val: Cell::new(
                LuaValKind::BOXED
                    ^ to_boxed(Box::new(LuaString {
                        v: string.0,
                        const_index: Some(string.1),
                    })),
            ),
        }
    }
}

impl From<&StdFunction> for LuaVal {
    /// Create a gc-able closure LuaVal
    fn from(func: &StdFunction) -> Self {
        let lua_closure = from_stdfunction(func);
        unsafe {
            let ptr = GcBox::new(lua_closure);
            (*ptr.as_ptr()).value().unroot();
            let val = LuaVal {
                val: Cell::new(LuaValKind::GcRoot ^ ptr.as_ptr() as usize),
            };
            val
        }
    }
}

impl From<&Function> for LuaVal {
    /// Create a gc-able closure LuaVal
    fn from(func: &Function) -> Self {
        let lua_closure = from_function(func);
        unsafe {
            let ptr = GcBox::new(lua_closure);
            (*ptr.as_ptr()).value().unroot();
            let val = LuaVal {
                val: Cell::new(LuaValKind::GcRoot ^ ptr.as_ptr() as usize),
            };
            val
        }
    }
}

impl From<bool> for LuaVal {
    /// Create an integer LuaVal.
    fn from(b: bool) -> Self {
        LuaVal {
            val: Cell::new(LuaValKind::BOOL ^ ((b as usize) << tagging::TAG_SHIFT)),
        }
    }
}

impl<T: GcVal + 'static> From<T> for LuaVal {
    /// Create a gc-able LuaVal.
    fn from(table: T) -> Self {
        let lua_table: Box<dyn GcVal> = Box::new(table);
        unsafe {
            let ptr = GcBox::new(lua_table);
            (*ptr.as_ptr()).value().unroot();
            let val = LuaVal {
                val: Cell::new(LuaValKind::GcRoot ^ ptr.as_ptr() as usize),
            };
            val
        }
    }
}

impl Drop for LuaVal {
    fn drop(&mut self) {
        match self.kind() {
            LuaValKind::BOXED => unsafe {
                Box::from_raw(self.as_boxed());
            },
            LuaValKind::GcRoot => unsafe {
                self.inner().unroot_inner();
            },
            // NIL is a nullptr, so there is no need to free, and raw ints and floats
            // are not heap allocated.
            _ => (),
        }
    }
}

impl Clone for LuaVal {
    fn clone(&self) -> LuaVal {
        match self.kind() {
            LuaValKind::BOXED => LuaVal {
                val: Cell::new(unsafe {
                    LuaValKind::BOXED ^ to_boxed((*self.as_boxed()).clone_box())
                }),
            },
            LuaValKind::Gc | LuaValKind::GcRoot => unsafe {
                self.inner().root_inner();
                let val = LuaVal {
                    val: Cell::new(self.val.get()),
                };
                val.set_root();
                val
            },
            _ => LuaVal {
                val: Cell::new(self.val.get()),
            },
        }
    }
}

impl Display for LuaVal {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self.kind() {
            LuaValKind::NIL => write!(f, "nil"),
            LuaValKind::Gc | LuaValKind::GcRoot => {
                if unsafe { (*gc_ptr(self.val.get())).value() }.is_table() {
                    write!(f, "lua_table at {:x}", gc_ptr(self.val.get()) as usize)
                } else {
                    write!(f, "lua_closure at {:x}", gc_ptr(self.val.get()) as usize)
                }
            }
            _ => write!(f, "{}", self.to_string().unwrap()),
        }
    }
}

impl PartialOrd for LuaVal {
    fn partial_cmp(&self, other: &LuaVal) -> Option<Ordering> {
        if self.is_number() && other.is_number() {
            Some(
                self.to_float()
                    .unwrap()
                    .total_cmp(&other.to_float().unwrap()),
            )
        } else if self.is_string() && other.is_string() {
            Some(
                self.get_string_ref()
                    .unwrap()
                    .cmp(other.get_string_ref().unwrap()),
            )
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::lua_table::*;
    use super::*;
    use std::{collections::HashMap, vec::Vec};

    fn test_get_and_set_attr_errors(main: &mut LuaVal) {
        assert_eq!(
            main.get_attr(&LuaVal::from(String::from("foo")))
                .unwrap_err(),
            LuaError::GetAttrErr
        );
        assert_eq!(
            main.set_attr(LuaVal::from(String::from("foo")), LuaVal::new())
                .unwrap_err(),
            LuaError::SetAttrErr
        );
    }

    #[test]
    fn nil_type() {
        let mut main = LuaVal::new();
        assert_eq!(main.kind(), LuaValKind::NIL);
        assert_eq!(main.is_aop_float(), false);
        assert_eq!(main.to_int().unwrap_err(), LuaError::IntConversionErr);
        assert_eq!(main.to_float().unwrap_err(), LuaError::FloatConversionErr);
        test_get_and_set_attr_errors(&mut main);
        let main_clone = main.clone();
        assert_eq!(main_clone.val, main.val);
        assert_eq!(main_clone.kind(), main.kind());
    }

    #[test]
    fn int_type() {
        let mut main = LuaVal::from(1);
        assert_eq!(main.kind(), LuaValKind::INT);
        assert_eq!(main.is_aop_float(), false);
        assert_eq!(main.to_int().unwrap(), 1);
        assert_float_absolute_eq!(main.to_float().unwrap(), 1.0, 0.1);
        test_get_and_set_attr_errors(&mut main);
        let main_clone = main.clone();
        assert_eq!(main_clone.val, main.val);
        assert_eq!(main_clone.kind(), main.kind());
        assert_eq!(main.negate_number().unwrap().to_int().unwrap(), -1);
    }

    #[test]
    fn luaint_type() {
        let val = 2_i64.pow(62);
        let mut main = LuaVal::from(val);
        assert_eq!(main.kind(), LuaValKind::BOXED);
        assert_eq!(main.is_aop_float(), false);
        assert_eq!(main.to_int().unwrap(), val);
        assert_float_absolute_eq!(main.to_float().unwrap(), 2.0_f64.powf(62.0), 0.1);
        test_get_and_set_attr_errors(&mut main);
        let main_clone = main.clone();
        assert_ne!(main_clone.val, main.val);
        assert_eq!(main_clone.kind(), main.kind());
    }

    #[test]
    fn float_type() {
        let float_to_test = unsafe { transmute::<u64, f64>(2_u64.pow(61) - 1) };
        let mut main = LuaVal::from(float_to_test);
        assert_eq!(main.kind(), LuaValKind::BOXED);
        assert_eq!(main.is_aop_float(), true);
        assert_eq!(main.to_int().unwrap_err(), LuaError::IntConversionErr);
        assert_float_absolute_eq!(main.to_float().unwrap(), float_to_test, 0.00000000001);
        test_get_and_set_attr_errors(&mut main);
        let main_clone = main.clone();
        assert_ne!(main_clone.val, main.val);
        assert_eq!(main_clone.kind(), main.kind());
    }

    #[test]
    fn luafloat_type() {
        let mut main = LuaVal::from(1.0);
        assert_eq!(main.kind(), LuaValKind::FLOAT);
        assert_eq!(main.is_aop_float(), true);
        assert_eq!(main.to_int().unwrap_err(), LuaError::IntConversionErr);
        assert_float_absolute_eq!(main.to_float().unwrap(), 1.0, 0.1);
        test_get_and_set_attr_errors(&mut main);
        let main_clone = main.clone();
        assert_eq!(main_clone.val, main.val);
        assert_eq!(main_clone.kind(), main.kind());
    }

    #[test]
    fn luastring_type() {
        let mut main = LuaVal::from(String::from("1"));
        assert_eq!(main.kind(), LuaValKind::BOXED);
        assert_eq!(main.is_aop_float(), true);
        assert_eq!(main.to_int().unwrap(), 1);
        assert_float_absolute_eq!(main.to_float().unwrap(), 1.0, 0.1);
        test_get_and_set_attr_errors(&mut main);
        let main_clone = main.clone();
        assert_ne!(main_clone.val, main.val);
        assert_eq!(main_clone.kind(), main.kind());
    }

    #[test]
    fn table_type() {
        let mut hm = HashMap::new();
        hm.insert(LuaVal::from(String::from("bar")), LuaVal::from(2));
        let main = LuaVal::from(UserTable::new(hm));
        assert!(main.kind() == LuaValKind::Gc || main.kind() == LuaValKind::GcRoot);
        assert_eq!(main.is_aop_float(), false);
        assert_eq!(main.to_int().unwrap_err(), LuaError::IntConversionErr);
        assert_eq!(main.to_float().unwrap_err(), LuaError::FloatConversionErr);
        let main_clone = main.clone();
        assert_eq!(main_clone.kind(), main.kind());
        assert_eq!(
            main.get_attr(&LuaVal::from(String::from("foo")))
                .unwrap()
                .kind(),
            LuaValKind::NIL
        );
        let bar_get = main.get_attr(&LuaVal::from(String::from("bar"))).unwrap();
        assert_eq!(bar_get.kind(), LuaValKind::INT);
        assert_eq!(bar_get.to_int().unwrap(), 2);
        main.set_attr(LuaVal::from(String::from("bar")), LuaVal::from(2.0))
            .unwrap();
        let bar_get = main.get_attr(&LuaVal::from(String::from("bar"))).unwrap();
        assert_eq!(bar_get.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(bar_get.to_float().unwrap(), 2.0, 0.1);
    }

    #[test]
    fn closure_type() {
        let mut main = LuaVal::from(UserFunction::new(0, 0, 0));
        assert!(main.kind() == LuaValKind::Gc || main.kind() == LuaValKind::GcRoot);
        assert_eq!(main.is_aop_float(), false);
        assert_eq!(main.to_int().unwrap_err(), LuaError::IntConversionErr);
        assert_eq!(main.to_float().unwrap_err(), LuaError::FloatConversionErr);
        test_get_and_set_attr_errors(&mut main);
    }

    #[test]
    fn bool_type() {
        let mut main = LuaVal::from(false);
        assert_eq!(main.kind(), LuaValKind::BOOL);
        assert_eq!(main.is_aop_float(), false);
        assert_eq!(main.to_int().unwrap_err(), LuaError::IntConversionErr);
        assert_eq!(main.to_float().unwrap_err(), LuaError::FloatConversionErr);
        test_get_and_set_attr_errors(&mut main);
        let main_clone = main.clone();
        assert_eq!(main_clone.val, main.val);
        assert_eq!(main_clone.kind(), main.kind());
    }

    fn get_types() -> Vec<LuaVal> {
        vec![
            LuaVal::new(),
            LuaVal::from(1),
            LuaVal::from(3.0),
            LuaVal::from(UserTable::new(HashMap::new())),
            LuaVal::from(String::from("3.0")),
            LuaVal::from(UserFunction::new(0, 0, 0)),
            LuaVal::from(false),
        ]
    }

    #[test]
    fn add() {
        let types = get_types();
        for t in types.iter() {
            // cannot add nils, tables, closures, or bools
            for i in vec![0, 3, 5, 6] {
                assert!(types[i].add(t).is_err());
                assert!(t.add(&types[i]).is_err());
            }
        }
        // int + int
        let val = types[1].add(&types[1]).unwrap();
        assert_eq!(val.kind(), LuaValKind::INT);
        assert_eq!(val.to_int().unwrap(), 2);
        // int + float
        let val = types[1].add(&types[2]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 4.0, 0.1);
        // int + string
        let val = types[1].add(&types[4]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 4.0, 0.1);
        // float + int
        let val = types[2].add(&types[1]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 4.0, 0.1);
        // float + float
        let val = types[2].add(&types[2]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 6.0, 0.1);
        // float + string
        let val = types[2].add(&types[4]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 6.0, 0.1);
        // string + int
        let val = types[4].add(&types[1]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 4.0, 0.1);
        // string + float
        let val = types[4].add(&types[2]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 6.0, 0.1);
        // string + string
        let val = types[4].add(&types[4]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 6.0, 0.1);
    }

    #[test]
    fn sub() {
        let types = get_types();
        for t in types.iter() {
            // cannot sub nils, tables, closures, or bools
            for i in vec![0, 3, 5, 6] {
                assert!(types[i].add(t).is_err());
                assert!(t.add(&types[i]).is_err());
            }
        }
        // int - int
        let val = types[1].sub(&types[1]).unwrap();
        assert_eq!(val.kind(), LuaValKind::INT);
        assert_eq!(val.to_int().unwrap(), 0);
        // int - float
        let val = types[1].sub(&types[2]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), -2.0, 0.1);
        // int - string
        let val = types[1].sub(&types[4]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), -2.0, 0.1);
        // float - int
        let val = types[2].sub(&types[1]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 2.0, 0.1);
        // float - float
        let val = types[2].sub(&types[2]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 0.0, 0.1);
        // float - string
        let val = types[2].sub(&types[4]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 0.0, 0.1);
        // string - int
        let val = types[4].sub(&types[1]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 2.0, 0.1);
        // string - float
        let val = types[4].sub(&types[2]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 0.0, 0.1);
        // string - string
        let val = types[4].sub(&types[4]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 0.0, 0.1);
    }

    #[test]
    fn mul() {
        let types = get_types();
        for t in types.iter() {
            // cannot mul nils, tables, closures or bools
            for i in vec![0, 3, 5, 6] {
                assert!(types[i].add(t).is_err());
                assert!(t.add(&types[i]).is_err());
            }
        }
        // int * int
        let val = types[1].mul(&types[1]).unwrap();
        assert_eq!(val.kind(), LuaValKind::INT);
        assert_eq!(val.to_int().unwrap(), 1);
        // int * float
        let val = types[1].mul(&types[2]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 3.0, 0.1);
        // int * string
        let val = types[1].mul(&types[4]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 3.0, 0.1);
        // float * int
        let val = types[2].mul(&types[1]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 3.0, 0.1);
        // float * float
        let val = types[2].mul(&types[2]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 9.0, 0.1);
        // float * string
        let val = types[2].mul(&types[4]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 9.0, 0.1);
        // string * int
        let val = types[4].mul(&types[1]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 3.0, 0.1);
        // string * float
        let val = types[4].mul(&types[2]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 9.0, 0.1);
        // string * string
        let val = types[4].mul(&types[4]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 9.0, 0.1);
    }

    #[test]
    fn div() {
        let types = get_types();
        for t in types.iter() {
            // cannot div nils, tables, closures, or bools
            for i in vec![0, 3, 5, 6] {
                assert!(types[i].add(t).is_err());
                assert!(t.add(&types[i]).is_err());
            }
        }
        // int / int
        let val = types[1].div(&types[1]).unwrap();
        assert_eq!(val.kind(), LuaValKind::INT);
        assert_eq!(val.to_int().unwrap(), 1);
        // int / float
        let val = types[1].div(&types[2]).unwrap();
        assert_eq!(val.kind(), LuaValKind::BOXED);
        assert_float_absolute_eq!(val.to_float().unwrap(), 0.3, 0.1);
        // int / string
        let val = types[1].div(&types[4]).unwrap();
        assert_eq!(val.kind(), LuaValKind::BOXED);
        assert_float_absolute_eq!(val.to_float().unwrap(), 0.3, 0.1);
        // float / int
        let val = types[2].div(&types[1]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 3.0, 0.1);
        // float / float
        let val = types[2].div(&types[2]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 1.0, 0.1);
        // float / string
        let val = types[2].div(&types[4]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 1.0, 0.1);
        // string / int
        let val = types[4].div(&types[1]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 3.0, 0.1);
        // string / float
        let val = types[4].div(&types[2]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 1.0, 0.1);
        // string / string
        let val = types[4].div(&types[4]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 1.0, 0.1);
    }

    #[test]
    fn modulus() {
        let types = get_types();
        for t in types.iter() {
            // cannot mod nils, tables, closures, or bools
            for i in vec![0, 3, 5, 6] {
                assert!(types[i].add(t).is_err());
                assert!(t.add(&types[i]).is_err());
            }
        }
        // int % int
        let val = types[1].modulus(&types[1]).unwrap();
        assert_eq!(val.kind(), LuaValKind::INT);
        assert_eq!(val.to_int().unwrap(), 0);
        // int % float
        let val = types[1].modulus(&types[2]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 1.0, 0.1);
        // int % string
        let val = types[1].modulus(&types[4]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 1.0, 0.1);
        // float % int
        let val = types[2].modulus(&types[1]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 0.0, 0.1);
        // float % float
        let val = types[2].modulus(&types[2]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 0.0, 0.1);
        // float % string
        let val = types[2].modulus(&types[4]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 0.0, 0.1);
        // string % int
        let val = types[4].modulus(&types[1]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 0.0, 0.1);
        // string % float
        let val = types[4].modulus(&types[2]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 0.0, 0.1);
        // string % string
        let val = types[4].modulus(&types[4]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 0.0, 0.1);
    }

    #[test]
    fn fdiv() {
        let types = get_types();
        for t in types.iter() {
            // cannot fdiv nils, tables, closures, or bools
            for i in vec![0, 3, 5, 6] {
                assert!(types[i].add(t).is_err());
                assert!(t.add(&types[i]).is_err());
            }
        }
        // int // int
        let val = types[1].fdiv(&types[1]).unwrap();
        assert_eq!(val.kind(), LuaValKind::INT);
        assert_eq!(val.to_int().unwrap(), 1);
        // int // float
        let val = types[1].fdiv(&types[2]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 0.0, 0.1);
        // int // string
        let val = types[1].fdiv(&types[4]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 0.0, 0.1);
        // float // int
        let val = types[2].fdiv(&types[1]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 3.0, 0.1);
        // float // float
        let val = types[2].fdiv(&types[2]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 1.0, 0.1);
        // float // string
        let val = types[2].fdiv(&types[4]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 1.0, 0.1);
        // string // int
        let val = types[4].fdiv(&types[1]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 3.0, 0.1);
        // string // float
        let val = types[4].fdiv(&types[2]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 1.0, 0.1);
        // string // string
        let val = types[4].fdiv(&types[4]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 1.0, 0.1);
    }

    #[test]
    fn exp() {
        let types = get_types();
        for t in types.iter() {
            // cannot exp nils, tables, closures, or bools
            for i in vec![0, 3, 5, 6] {
                assert!(types[i].add(t).is_err());
                assert!(t.add(&types[i]).is_err());
            }
        }
        // int ^ int
        let val = types[1].exp(&types[1]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 1.0, 0.1);
        // int ^ float
        let val = types[1].exp(&types[2]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 1.0, 0.1);
        // int ^ string
        let val = types[1].exp(&types[4]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 1.0, 0.1);
        // float ^ int
        let val = types[2].exp(&types[1]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 3.0, 0.1);
        // float ^ float
        let val = types[2].exp(&types[2]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 27.0, 0.1);
        // float ^ string
        let val = types[2].exp(&types[4]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 27.0, 0.1);
        // string ^ int
        let val = types[4].exp(&types[1]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 3.0, 0.1);
        // string ^ float
        let val = types[4].exp(&types[2]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 27.0, 0.1);
        // string ^ string
        let val = types[4].exp(&types[4]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 27.0, 0.1);
    }

    #[test]
    fn table_mutability() {
        let mut hm1 = HashMap::new();
        hm1.insert(LuaVal::from(String::from("foo")), LuaVal::from(1));
        let mut hm2 = HashMap::new();
        hm2.insert(LuaVal::from(String::from("bar")), LuaVal::from(2.0));
        // table1 {foo: 1}
        let table1 = LuaVal::from(UserTable::new(hm1));
        hm2.insert(LuaVal::from(String::from("foo")), table1);
        // table2 { foo: { foo: 1 }, bar: 2.0 }
        let table2 = LuaVal::from(UserTable::new(hm2));
        // table3 { foo: { foo: 1 }, bar: 2.0 }, table 3 is a reference to the same dict
        let table3 = table2.clone();
        // table2 { foo: { foo: 1 }, bar: 2 }
        table2
            .set_attr(LuaVal::from(String::from("bar")), LuaVal::from(2))
            .unwrap();
        // check if table3 was updated as well
        assert_eq!(
            table3
                .get_attr(&LuaVal::from(String::from("bar")))
                .unwrap()
                .to_int()
                .unwrap(),
            2
        );
        // table2 { foo: { foo: 2 }, bar: 2 }
        table2
            .get_attr(&LuaVal::from(String::from("foo")))
            .unwrap()
            .set_attr(LuaVal::from(String::from("foo")), LuaVal::from(2))
            .unwrap();
        assert_eq!(
            table3
                .get_attr(&LuaVal::from(String::from("foo")))
                .unwrap()
                .get_attr(&LuaVal::from(String::from("foo")))
                .unwrap()
                .to_int()
                .unwrap(),
            2
        );
    }

    fn get_eq_types() -> Vec<LuaVal> {
        vec![
            LuaVal::from(1),
            LuaVal::from(1.0),
            LuaVal::from(String::from("1.0")),
            LuaVal::from(UserTable::new(HashMap::new())),
            LuaVal::from(UserFunction::new(0, 0, 0)),
        ]
    }

    #[test]
    fn eq_for_ints() {
        let types = get_eq_types();
        let int = LuaVal::from(1);
        assert_eq!(int, types[0]);
        assert_eq!(int, types[1]);
        assert_ne!(int, types[2]);
        assert_ne!(int, types[3]);
        assert_ne!(int, types[4]);
        let int = LuaVal::from(2);
        for i in 0..5 {
            assert_ne!(int, types[i]);
        }
    }

    #[test]
    fn eq_for_floats() {
        let types = get_eq_types();
        let float = LuaVal::from(1.0);
        assert_eq!(float, types[0]);
        assert_eq!(float, types[1]);
        assert_ne!(float, types[2]);
        assert_ne!(float, types[3]);
        assert_ne!(float, types[4]);
        let float = LuaVal::from(2.0);
        for i in 0..5 {
            assert_ne!(float, types[i]);
        }
    }

    #[test]
    fn eq_for_tables() {
        let types = get_eq_types();
        let table = LuaVal::from(UserTable::new(HashMap::new()));
        assert_eq!(table, table);
        for i in 0..5 {
            assert_ne!(table, types[i]);
        }
    }

    #[test]
    fn eq_for_strings() {
        let types = get_eq_types();
        let string = LuaVal::from(String::from("1.0"));
        assert_ne!(string, types[0]);
        assert_ne!(string, types[1]);
        assert_eq!(string, types[2]);
        assert_ne!(string, types[3]);
        assert_ne!(string, types[4]);
        let string = LuaVal::from(String::from("2.0"));
        for i in 0..5 {
            assert_ne!(string, types[i]);
        }
    }
}
