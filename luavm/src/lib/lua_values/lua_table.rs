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
