use errors::LuaError;
use gc::GcCell;
use lua_values::gc_val::GcVal;
use std::collections::HashMap;
use LuaVal;

/// Represents a table in Lua.
#[derive(Trace, Finalize)]
pub struct UserTable {
    v: GcCell<HashMap<LuaVal, LuaVal>>,
}

impl UserTable {
    /// Creates a table with the given keys, and values.
    pub fn new(hm: HashMap<LuaVal, LuaVal>) -> UserTable {
        UserTable { v: GcCell::new(hm) }
    }
}

impl GcVal for UserTable {
    fn is_table(&self) -> bool {
        true
    }

    fn set_attr(&self, attr: LuaVal, val: LuaVal) -> Result<(), LuaError> {
        self.v.borrow_mut().insert(attr, val);
        Ok(())
    }

    fn get_attr(&self, attr: &LuaVal) -> Result<LuaVal, LuaError> {
        Ok(match self.v.borrow().get(attr) {
            Some(val) => val.clone(),
            None => LuaVal::new(),
        })
    }
}

#[derive(Trace, Finalize)]
pub struct CachingTable {
    str_attrs: GcCell<Vec<LuaVal>>,
    attrs: GcCell<HashMap<LuaVal, LuaVal>>,
}

impl CachingTable {
    pub fn new(hash_map: HashMap<LuaVal, LuaVal>, num: usize) -> CachingTable {
        let mut str_attrs = Vec::new();
        str_attrs.resize(num, LuaVal::new());
        CachingTable {
            str_attrs: GcCell::new(str_attrs),
            attrs: GcCell::new(hash_map),
        }
    }
}

impl GcVal for CachingTable {
    fn is_table(&self) -> bool {
        true
    }

    fn set_attr(&self, attr: LuaVal, val: LuaVal) -> Result<(), LuaError> {
        match attr.get_constant_index() {
            Some(i) => {
                self.str_attrs.borrow_mut()[i] = val;
            }
            None => {
                self.attrs.borrow_mut().insert(attr, val);
            }
        }
        Ok(())
    }

    fn get_attr(&self, attr: &LuaVal) -> Result<LuaVal, LuaError> {
        Ok(match attr.get_constant_index() {
            Some(i) => self.str_attrs.borrow()[i].clone(),
            None => match self.attrs.borrow().get(attr) {
                Some(val) => val.clone(),
                None => LuaVal::new(),
            },
        })
    }
}
