use gc::{Finalize, GcCell, Trace};
use std::collections::HashMap;
use LuaVal;

impl Finalize for Box<LuaTable> {}
unsafe impl Trace for Box<LuaTable> {
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

pub trait LuaTable: Trace + Finalize {
    fn get_attr(&self, attr: &LuaVal) -> LuaVal;
    fn set_attr(&self, attr: LuaVal, val: LuaVal);
}

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

impl LuaTable for UserTable {
    fn set_attr(&self, attr: LuaVal, val: LuaVal) {
        self.v.borrow_mut().insert(attr, val);
    }

    fn get_attr(&self, attr: &LuaVal) -> LuaVal {
        match self.v.borrow().get(attr) {
            Some(val) => val.clone(),
            None => LuaVal::new(),
        }
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

impl LuaTable for CachingTable {
    fn set_attr(&self, attr: LuaVal, val: LuaVal) {
        match attr.get_constant_index() {
            Some(i) => {
                self.str_attrs.borrow_mut()[i] = val;
            }
            None => {
                self.attrs.borrow_mut().insert(attr, val);
            }
        }
    }

    fn get_attr(&self, attr: &LuaVal) -> LuaVal {
        match attr.get_constant_index() {
            Some(i) => self.str_attrs.borrow()[i].clone(),
            None => match self.attrs.borrow().get(attr) {
                Some(val) => val.clone(),
                None => LuaVal::new(),
            },
        }
    }
}
