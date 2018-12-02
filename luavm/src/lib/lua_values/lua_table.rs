use gc::GcCell;
use std::collections::HashMap;
use LuaVal;

/// Represents a table in Lua.
#[derive(Trace, Finalize)]
pub struct LuaTable {
    v: GcCell<HashMap<LuaVal, LuaVal>>,
}

impl LuaTable {
    /// Creates a table with the given keys, and values.
    pub fn new(hm: HashMap<LuaVal, LuaVal>) -> LuaTable {
        LuaTable { v: GcCell::new(hm) }
    }

    /// Sets the given attribute to `val`.
    pub fn set_attr(&self, attr: LuaVal, val: LuaVal) {
        self.v.borrow_mut().insert(attr, val);
    }

    /// Gets a reference to the given attribute.
    pub fn get_attr(&self, attr: &LuaVal) -> LuaVal {
        match self.v.borrow().get(attr) {
            Some(val) => val.clone(),
            None => LuaVal::new(),
        }
    }
}
