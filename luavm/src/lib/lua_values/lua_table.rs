use gc::GcCell;
use std::collections::HashMap;
use LuaVal;

/// Represents a table in Lua.
#[derive(Trace, Finalize)]
pub struct LuaTable {
    v: GcCell<HashMap<String, LuaVal>>,
}

impl LuaTable {
    /// Creates a table with the given keys, and values.
    pub fn new(hm: HashMap<String, LuaVal>) -> LuaTable {
        LuaTable { v: GcCell::new(hm) }
    }

    /// Sets the given attribute to `val`.
    pub fn set(&self, attr: &str, val: LuaVal) {
        self.v
            .borrow_mut()
            .entry(attr.to_string())
            .or_insert(LuaVal::new())
            .set(val);
    }

    /// Gets a reference to given attribute.
    pub fn get_attr(&self, attr: &str) -> LuaVal {
        match self.v.borrow().get(attr) {
            Some(val) => val.clone(),
            None => LuaVal::new(),
        }
    }
}
