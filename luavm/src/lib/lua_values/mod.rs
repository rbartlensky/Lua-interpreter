mod lua_obj;
mod lua_table;
mod tagging;

use self::{lua_obj::*, lua_table::LuaTable, tagging::*};
use errors::LuaError;
use gc::{Finalize, Gc, Trace};
use std::mem::{swap, transmute};

/// Represents a value in Lua.
#[derive(Debug)]
pub struct LuaVal {
    val: usize,
}

impl Finalize for LuaVal {}
unsafe impl Trace for LuaVal {
    custom_trace!(this, {
        // only tables have a Gc inside, so there is no need to mark anything else
        if let LuaValKind::TABLE = this.kind() {
            mark(unsafe { &*table_ptr(this.val) });
        }
    });
}

impl LuaVal {
    /// Create an empty LuaVal which is equivalent to Nil.
    pub fn new() -> LuaVal {
        LuaVal { val: 0 }
    }

    /// Returns the type of the value store in the pointer.
    fn kind(&self) -> LuaValKind {
        LuaValKind::from(self.val)
    }

    /// Interprets the value as a pointer to a LuaObj, and returns a pointer to it.
    fn as_boxed(&self) -> *mut Box<LuaObj> {
        (LuaValKind::BOXED ^ self.val) as *mut Box<LuaObj>
    }

    /// Returns true if this value can be considered a float, or false otherwise.
    /// This method is only used in arithmetic operations.
    fn is_float(&self) -> bool {
        match self.kind() {
            LuaValKind::FLOAT => true,
            LuaValKind::BOXED => unsafe { (*self.as_boxed()).is_float() },
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
                LuaValKind::INT => Ok(((self.val >> tagging::TAG_SHIFT) as i64) as f64),
                LuaValKind::FLOAT => Ok(transmute::<usize, f64>(LuaValKind::FLOAT ^ self.val)),
                LuaValKind::BOXED => (*self.as_boxed()).to_float(),
                _ => Err(LuaError::FloatConversionErr),
            }
        }
    }

    /// Attempts to convert this value to an integer.
    pub fn to_int(&self) -> Result<i64, LuaError> {
        unsafe {
            match self.kind() {
                LuaValKind::INT => Ok((self.val >> tagging::TAG_SHIFT) as i64),
                LuaValKind::BOXED => (*self.as_boxed()).to_int(),
                _ => Err(LuaError::IntConversionErr),
            }
        }
    }

    /// Sets this to a new value.
    pub fn set(&mut self, mut val: LuaVal) {
        // exchange the pointers so that when `val` goes out of scope the old value
        // is released and not leaked
        swap(&mut self.val, &mut val.val);
    }

    /// Sets the given attribute to a given value.
    pub fn set_attr(&mut self, attr: &str, val: LuaVal) -> Result<(), LuaError> {
        if let LuaValKind::TABLE = self.kind() {
            Ok(unsafe {
                (*table_ptr(self.val)).set(attr, val);
            })
        } else {
            Err(LuaError::SetAttrErr)
        }
    }

    /// Gets the value of the given attribute.
    pub fn get_attr(&self, attr: &str) -> Result<LuaVal, LuaError> {
        if let LuaValKind::TABLE = self.kind() {
            Ok(unsafe { (*table_ptr(self.val)).get_attr(attr) })
        } else {
            Err(LuaError::GetAttrErr)
        }
    }

    pub fn add(&self, other: &LuaVal) -> Result<LuaVal, LuaError> {
        Ok(if self.is_float() || other.is_float() {
            LuaVal::from(self.to_float()? + other.to_float()?)
        } else {
            LuaVal::from(self.to_int()? + other.to_int()?)
        })
    }

    pub fn sub(&self, other: &LuaVal) -> Result<LuaVal, LuaError> {
        Ok(if self.is_float() || other.is_float() {
            LuaVal::from(self.to_float()? - other.to_float()?)
        } else {
            LuaVal::from(self.to_int()? - other.to_int()?)
        })
    }

    pub fn mul(&self, other: &LuaVal) -> Result<LuaVal, LuaError> {
        Ok(if self.is_float() || other.is_float() {
            LuaVal::from(self.to_float()? * other.to_float()?)
        } else {
            LuaVal::from(self.to_int()? * other.to_int()?)
        })
    }

    pub fn div(&self, other: &LuaVal) -> Result<LuaVal, LuaError> {
        Ok(if self.is_float() || other.is_float() {
            LuaVal::from(self.to_float()? / other.to_float()?)
        } else {
            LuaVal::from(self.to_int()? / other.to_int()?)
        })
    }

    pub fn modulus(&self, other: &LuaVal) -> Result<LuaVal, LuaError> {
        Ok(if self.is_float() || other.is_float() {
            LuaVal::from(self.to_float()? % other.to_float()?)
        } else {
            LuaVal::from(self.to_int()? % other.to_int()?)
        })
    }

    pub fn fdiv(&self, other: &LuaVal) -> Result<LuaVal, LuaError> {
        Ok(if self.is_float() || other.is_float() {
            LuaVal::from((self.to_float()? / other.to_float()?).floor())
        } else {
            LuaVal::from(self.to_int()? / other.to_int()?)
        })
    }

    pub fn exp(&self, other: &LuaVal) -> Result<LuaVal, LuaError> {
        Ok(LuaVal::from(self.to_float()?.powf(other.to_float()?)))
    }
}

impl From<i64> for LuaVal {
    /// Create an integer LuaVal.
    fn from(int: i64) -> Self {
        let uint = int as usize;
        // if any of the 3 high-order bits are set, then the int is boxed
        let val = if uint & !tagging::MASK != 0 {
            LuaValKind::BOXED ^ to_boxed(Box::new(LuaInt { v: int }))
        } else {
            LuaValKind::INT ^ (uint << tagging::TAG_SHIFT)
        };
        LuaVal { val }
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
        LuaVal { val }
    }
}

impl From<LuaTable> for LuaVal {
    /// Create a table LuaVal.
    fn from(table: LuaTable) -> Self {
        LuaVal {
            val: LuaValKind::TABLE ^ to_raw_ptr(Gc::new(table)),
        }
    }
}

impl Drop for LuaVal {
    fn drop(&mut self) {
        match self.kind() {
            // NIL is a nullptr, so there is no need to free, and raw ints and floats
            // are not heap allocated.
            LuaValKind::NIL | LuaValKind::INT | LuaValKind::FLOAT => (),
            LuaValKind::BOXED => unsafe {
                Box::from_raw(self.as_boxed());
            },
            LuaValKind::TABLE => unsafe {
                Box::from_raw(table_ptr(self.val));
            },
        }
    }
}

impl Clone for LuaVal {
    fn clone(&self) -> LuaVal {
        let val = match self.kind() {
            LuaValKind::NIL | LuaValKind::INT | LuaValKind::FLOAT => self.val,
            LuaValKind::BOXED => unsafe {
                LuaValKind::BOXED ^ to_boxed((*self.as_boxed()).clone_box())
            },
            LuaValKind::TABLE => unsafe {
                LuaValKind::TABLE ^ to_raw_ptr((*table_ptr(self.val)).clone())
            },
        };
        LuaVal { val }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashMap, vec::Vec};

    fn test_get_and_set_attr_errors(main: &mut LuaVal) {
        assert_eq!(main.get_attr("foo").unwrap_err(), LuaError::GetAttrErr);
        assert_eq!(
            main.set_attr("foo", LuaVal::new()).unwrap_err(),
            LuaError::SetAttrErr
        );
    }

    #[test]
    fn nil_type() {
        let mut main = LuaVal::new();
        assert_eq!(main.kind(), LuaValKind::NIL);
        assert_eq!(main.is_float(), false);
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
        assert_eq!(main.is_float(), false);
        assert_eq!(main.to_int().unwrap(), 1);
        assert_float_absolute_eq!(main.to_float().unwrap(), 1.0, 0.1);
        test_get_and_set_attr_errors(&mut main);
        let main_clone = main.clone();
        assert_eq!(main_clone.val, main.val);
        assert_eq!(main_clone.kind(), main.kind());
    }

    #[test]
    fn luaint_type() {
        let val = 2_i64.pow(62);
        let mut main = LuaVal::from(val);
        assert_eq!(main.kind(), LuaValKind::BOXED);
        assert_eq!(main.is_float(), false);
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
        assert_eq!(main.is_float(), true);
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
        assert_eq!(main.is_float(), true);
        assert_eq!(main.to_int().unwrap_err(), LuaError::IntConversionErr);
        assert_float_absolute_eq!(main.to_float().unwrap(), 1.0, 0.1);
        test_get_and_set_attr_errors(&mut main);
        let main_clone = main.clone();
        assert_eq!(main_clone.val, main.val);
        assert_eq!(main_clone.kind(), main.kind());
    }

    #[test]
    fn table_type() {
        let mut hm = HashMap::new();
        hm.insert(String::from("bar"), LuaVal::from(2));
        let mut main = LuaVal::from(LuaTable::new(hm));
        assert_eq!(main.kind(), LuaValKind::TABLE);
        assert_eq!(main.is_float(), false);
        assert_eq!(main.to_int().unwrap_err(), LuaError::IntConversionErr);
        assert_eq!(main.to_float().unwrap_err(), LuaError::FloatConversionErr);
        let main_clone = main.clone();
        assert_ne!(main_clone.val, main.val);
        assert_eq!(main_clone.kind(), main.kind());
        assert_eq!(main.get_attr("foo").unwrap().kind(), LuaValKind::NIL);
        let bar_get = main.get_attr("bar").unwrap();
        assert_eq!(bar_get.kind(), LuaValKind::INT);
        assert_eq!(bar_get.to_int().unwrap(), 2);
        main.set_attr("bar", LuaVal::from(2.0));
        let bar_get = main.get_attr("bar").unwrap();
        assert_eq!(bar_get.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(bar_get.to_float().unwrap(), 2.0, 0.1);
    }

    fn get_types() -> Vec<LuaVal> {
        vec![
            LuaVal::new(),
            LuaVal::from(1),
            LuaVal::from(3.0),
            LuaVal::from(LuaTable::new(HashMap::new())),
        ]
    }

    #[test]
    fn add() {
        let types = get_types();
        // cannot add nils or tables
        for t in types.iter() {
            assert!(types[0].add(t).is_err());
            assert!(types[3].add(t).is_err());
            assert!(t.add(&types[0]).is_err());
            assert!(t.add(&types[3]).is_err());
        }
        // int + int
        let val = types[1].add(&types[1]).unwrap();
        assert_eq!(val.kind(), LuaValKind::INT);
        assert_eq!(val.to_int().unwrap(), 2);
        // int + float
        let val = types[1].add(&types[2]).unwrap();
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
    }

    #[test]
    fn sub() {
        let types = get_types();
        // cannot sub nils or tables
        for t in types.iter() {
            assert!(types[0].sub(t).is_err());
            assert!(types[3].sub(t).is_err());
            assert!(t.sub(&types[0]).is_err());
            assert!(t.sub(&types[3]).is_err());
        }
        // int - int
        let val = types[1].sub(&types[1]).unwrap();
        assert_eq!(val.kind(), LuaValKind::INT);
        assert_eq!(val.to_int().unwrap(), 0);
        // int - float
        let val = types[1].sub(&types[2]).unwrap();
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
    }

    #[test]
    fn mul() {
        let types = get_types();
        // cannot mul nils or tables
        for t in types.iter() {
            assert!(types[0].mul(t).is_err());
            assert!(types[3].mul(t).is_err());
            assert!(t.mul(&types[0]).is_err());
            assert!(t.mul(&types[3]).is_err());
        }
        // int * int
        let val = types[1].mul(&types[1]).unwrap();
        assert_eq!(val.kind(), LuaValKind::INT);
        assert_eq!(val.to_int().unwrap(), 1);
        // int * float
        let val = types[1].mul(&types[2]).unwrap();
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
    }

    #[test]
    fn div() {
        let types = get_types();
        // cannot div nils or tables
        for t in types.iter() {
            assert!(types[0].div(t).is_err());
            assert!(types[3].div(t).is_err());
            assert!(t.div(&types[0]).is_err());
            assert!(t.div(&types[3]).is_err());
        }
        // int / int
        let val = types[1].div(&types[1]).unwrap();
        assert_eq!(val.kind(), LuaValKind::INT);
        assert_eq!(val.to_int().unwrap(), 1);
        // int / float
        let val = types[1].div(&types[2]).unwrap();
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
    }

    #[test]
    fn modulus() {
        let types = get_types();
        // cannot modulus nils or tables
        for t in types.iter() {
            assert!(types[0].modulus(t).is_err());
            assert!(types[3].modulus(t).is_err());
            assert!(t.modulus(&types[0]).is_err());
            assert!(t.modulus(&types[3]).is_err());
        }
        // int % int
        let val = types[1].modulus(&types[1]).unwrap();
        assert_eq!(val.kind(), LuaValKind::INT);
        assert_eq!(val.to_int().unwrap(), 0);
        // int % float
        let val = types[1].modulus(&types[2]).unwrap();
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
    }

    #[test]
    fn fdiv() {
        let types = get_types();
        // cannot fdiv nils or tables
        for t in types.iter() {
            assert!(types[0].fdiv(t).is_err());
            assert!(types[3].fdiv(t).is_err());
            assert!(t.fdiv(&types[0]).is_err());
            assert!(t.fdiv(&types[3]).is_err());
        }
        // int // int
        let val = types[1].fdiv(&types[1]).unwrap();
        assert_eq!(val.kind(), LuaValKind::INT);
        assert_eq!(val.to_int().unwrap(), 1);
        // int // float
        let val = types[1].fdiv(&types[2]).unwrap();
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
    }

    #[test]
    fn exp() {
        let types = get_types();
        // cannot exp nils or tables
        for t in types.iter() {
            assert!(types[0].exp(t).is_err());
            assert!(types[3].exp(t).is_err());
            assert!(t.exp(&types[0]).is_err());
            assert!(t.exp(&types[3]).is_err());
        }
        // int ^ int
        let val = types[1].exp(&types[1]).unwrap();
        assert_eq!(val.kind(), LuaValKind::FLOAT);
        assert_float_absolute_eq!(val.to_float().unwrap(), 1.0, 0.1);
        // int ^ float
        let val = types[1].exp(&types[2]).unwrap();
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
    }

    #[test]
    fn table_mutability() {
        let mut hm1 = HashMap::new();
        hm1.insert(String::from("foo"), LuaVal::from(1));
        let mut hm2 = HashMap::new();
        hm2.insert(String::from("bar"), LuaVal::from(2.0));
        // table1 {foo: 1}
        let table1 = LuaVal::from(LuaTable::new(hm1));
        hm2.insert(String::from("foo"), table1);
        // table2 { foo: { foo: 1 }, bar: 2.0 }
        let mut table2 = LuaVal::from(LuaTable::new(hm2));
        // table3 { foo: { foo: 1 }, bar: 2.0 }, table 3 is a reference to the same dict
        let table3 = table2.clone();
        // table2 { foo: { foo: 1 }, bar: 2 }
        table2.set_attr("bar", LuaVal::from(2)).unwrap();
        // check if table3 was updated as well
        assert_eq!(table3.get_attr("bar").unwrap().to_int().unwrap(), 2);
        // table2 { foo: { foo: 2 }, bar: 2 }
        table2
            .get_attr("foo")
            .unwrap()
            .set_attr("foo", LuaVal::from(2));
        assert_eq!(
            table3
                .get_attr("foo")
                .unwrap()
                .get_attr("foo")
                .unwrap()
                .to_int()
                .unwrap(),
            2
        );
    }

    #[test]
    fn basic_types_mutability() {
        // int cloning
        let mut int = LuaVal::from(1);
        let int2 = int.clone();
        int.set(LuaVal::from(2));
        assert_ne!(int.to_int(), int2.to_int());
        // float cloning
        let mut float = LuaVal::from(1.0);
        let float2 = float.clone();
        float.set(LuaVal::from(2.0));
        assert_ne!(float.to_float().unwrap(), float2.to_float().unwrap());
    }
}
