use errors::LuaError;

/// Represents a super type for all primitives that don't fit in 61 bits.
pub trait LuaObj {
    /// Clones the underlying type, and returns a box of it.
    fn clone_box(&self) -> Box<LuaObj>;
    fn is_int(&self) -> bool;
    fn is_float(&self) -> bool;
    /// Checks whther the underlying type is a float or an int.
    fn is_number(&self) -> bool;
    /// Returns true if the underlying type is either a float or a string.
    /// In Lua, if either of these two types are used in an arithmetic
    /// expression, then both arguments are converted to floats.
    fn is_aop_float(&self) -> bool;
    /// Checks whether the underlying type is a string or not.
    fn is_string(&self) -> bool;
    /// Converts the underlying type to an int.
    fn to_int(&self) -> Result<i64, LuaError>;
    /// Converts the underlying type to a float.
    fn to_float(&self) -> Result<f64, LuaError>;
    /// Converts the underlying type to a string.
    fn to_string(&self) -> Result<String, LuaError>;
    /// Gets a reference to the underlying string.
    fn get_string_ref(&self) -> Option<&str> {
        None
    }
    /// If the underlying type is a String, then this method returns the String's index
    /// in the constant table.
    fn get_constant_index(&self) -> Option<usize> {
        None
    }
}

/// Boxes the given `LuaObj`, and returns the address of the box.
pub fn to_boxed(obj: Box<LuaObj>) -> usize {
    let bx = Box::into_raw(Box::new(obj));
    debug_assert_eq!(std::mem::size_of_val(&bx), 8);
    bx as usize
}

pub struct LuaInt {
    pub v: i64,
}

impl LuaObj for LuaInt {
    fn clone_box(&self) -> Box<LuaObj> {
        Box::new(LuaInt { v: self.v })
    }

    fn to_int(&self) -> Result<i64, LuaError> {
        Ok(self.v)
    }

    fn is_int(&self) -> bool {
        true
    }

    fn is_float(&self) -> bool {
        false
    }

    fn is_number(&self) -> bool {
        true
    }

    fn is_aop_float(&self) -> bool {
        false
    }

    fn is_string(&self) -> bool {
        false
    }

    fn to_float(&self) -> Result<f64, LuaError> {
        Ok(self.v as f64)
    }

    fn to_string(&self) -> Result<String, LuaError> {
        Ok(self.v.to_string())
    }
}

pub struct LuaFloat {
    pub v: f64,
}

impl LuaObj for LuaFloat {
    fn clone_box(&self) -> Box<LuaObj> {
        Box::new(LuaFloat { v: self.v })
    }

    fn is_int(&self) -> bool {
        false
    }

    fn is_float(&self) -> bool {
        true
    }

    fn is_number(&self) -> bool {
        true
    }

    fn is_aop_float(&self) -> bool {
        true
    }

    fn is_string(&self) -> bool {
        false
    }

    fn to_int(&self) -> Result<i64, LuaError> {
        Err(LuaError::IntConversionErr)
    }

    fn to_float(&self) -> Result<f64, LuaError> {
        Ok(self.v)
    }

    fn to_string(&self) -> Result<String, LuaError> {
        Ok(self.v.to_string())
    }
}

pub struct LuaString {
    pub v: String,
    /// The index of the string in the constant table.
    pub const_index: Option<usize>,
}

impl LuaObj for LuaString {
    fn clone_box(&self) -> Box<LuaObj> {
        Box::new(LuaString {
            v: self.v.clone(),
            const_index: self.const_index,
        })
    }

    fn is_int(&self) -> bool {
        false
    }

    fn is_float(&self) -> bool {
        false
    }

    fn is_number(&self) -> bool {
        false
    }

    fn is_aop_float(&self) -> bool {
        true
    }

    fn is_string(&self) -> bool {
        true
    }

    fn to_int(&self) -> Result<i64, LuaError> {
        self.v.parse().map_err(|_| LuaError::IntConversionErr)
    }

    fn to_float(&self) -> Result<f64, LuaError> {
        self.v.parse().map_err(|_| LuaError::FloatConversionErr)
    }

    fn to_string(&self) -> Result<String, LuaError> {
        Ok(self.v.clone())
    }

    fn get_string_ref(&self) -> Option<&str> {
        Some(&self.v)
    }

    fn get_constant_index(&self) -> Option<usize> {
        self.const_index
    }
}
