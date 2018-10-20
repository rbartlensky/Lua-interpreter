/// Super trait which helps with cloning `Box<LuaValue>`s.
pub trait LuaValueClone {
    fn clone_box(&self) -> Box<LuaValue>;
}

/// Represents a value in Lua.
pub trait LuaValue: LuaValueClone {
    fn to_int(&self) -> Option<i64> {
        None
    }

    fn is_float(&self) -> bool {
        false
    }

    fn to_float(&self) -> Option<f64> {
        None
    }
}

impl<T> LuaValueClone for T
where
    T: 'static + LuaValue + Clone,
{
    fn clone_box(&self) -> Box<LuaValue> {
        Box::new(self.clone())
    }
}

impl Clone for Box<LuaValue> {
    fn clone(&self) -> Box<LuaValue> {
        self.clone_box()
    }
}

#[derive(Clone)]
pub struct LuaNil {}

impl LuaValue for LuaNil {}

#[derive(Clone)]
pub struct LuaInt {
    v: i64,
}

impl LuaInt {
    pub fn new(v: i64) -> LuaInt {
        LuaInt { v }
    }
}

impl LuaValue for LuaInt {
    fn to_int(&self) -> Option<i64> {
        Some(self.v)
    }

    fn to_float(&self) -> Option<f64> {
        Some(self.v as f64)
    }
}

#[derive(Clone)]
pub struct LuaFloat {
    v: f64,
}

impl LuaFloat {
    pub fn new(v: f64) -> LuaFloat {
        LuaFloat { v }
    }
}

impl LuaValue for LuaFloat {
    fn is_float(&self) -> bool {
        true
    }

    fn to_float(&self) -> Option<f64> {
        Some(self.v)
    }
}

#[derive(Clone)]
pub struct LuaString {
    v: String,
}

impl LuaString {
    pub fn new(v: String) -> LuaString {
        LuaString { v }
    }
}

impl LuaValue for LuaString {
    fn is_float(&self) -> bool {
        true
    }

    fn to_float(&self) -> Option<f64> {
        self.v.parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lua_nil() {
        let ln = LuaNil {};
        assert!(!ln.is_float());
        assert!(ln.to_float().is_none());
        assert!(ln.to_int().is_none());
    }

    #[test]
    fn lua_int() {
        let li = LuaInt::new(2);
        assert!(!li.is_float());
        assert!(li.to_float().is_some());
        assert_eq!(li.to_int(), Some(2));
    }

    #[test]
    fn lua_float() {
        let lf = LuaFloat::new(0.0);
        assert!(lf.is_float());
        assert!(lf.to_float().is_some());
        assert!(lf.to_int().is_none());
    }

    #[test]
    fn lua_string() {
        let ls = LuaString::new("Foo".to_string());
        assert!(ls.is_float());
        assert!(ls.to_float().is_none());
        assert!(ls.to_int().is_none());
        let ls = LuaString::new("0.0".to_string());
        assert!(ls.to_float().is_some());
    }
}
