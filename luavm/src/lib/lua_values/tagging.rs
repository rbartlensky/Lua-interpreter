use super::lua_closure::LuaClosure;
use super::lua_table::LuaTable;
use gc::Gc;
use std::{mem::size_of, ops::BitXor};

/// Used for extracting the tag out of an address.
pub const MASK: usize = size_of::<usize>() - 1;
/// Used to shift raw integers in order to encode the INT tag.
pub const TAG_SHIFT: usize = 3;

/// Represents the type of a lua value.
#[derive(PartialEq, Eq, Debug)]
pub enum LuaValKind {
    BOXED = 0,
    INT = 1,
    FLOAT = 2,
    TABLE = 3,
    NIL = 4,
    CLOSURE = 5,
}

impl From<usize> for LuaValKind {
    fn from(encoded_ptr: usize) -> LuaValKind {
        if encoded_ptr == 0 {
            LuaValKind::NIL
        } else {
            let masked_ptr = encoded_ptr & MASK;
            match masked_ptr {
                0 => LuaValKind::BOXED,
                1 => LuaValKind::INT,
                2 => LuaValKind::FLOAT,
                3 => LuaValKind::TABLE,
                5 => LuaValKind::CLOSURE,
                _ => unreachable!(),
            }
        }
    }
}

impl BitXor<usize> for LuaValKind {
    type Output = usize;
    fn bitxor(self, rhs: usize) -> usize {
        (self as usize) ^ rhs
    }
}

/// Creates a raw pointer from the given value, and returns its address.
pub fn to_raw_ptr<T>(val: T) -> usize {
    Box::into_raw(Box::new(val)) as usize
}

/// Untags the given pointer, and returns a mutable pointer to Gc<LuaTable>.
pub fn table_ptr(encoded_ptr: usize) -> *mut Gc<LuaTable> {
    (encoded_ptr ^ LuaValKind::TABLE as usize) as *mut Gc<LuaTable>
}

/// Untags the given pointer, and returns a mutable pointer to Gc<LuaClosure>.
pub fn closure_ptr(encoded_ptr: usize) -> *mut Gc<LuaClosure> {
    (encoded_ptr ^ LuaValKind::CLOSURE as usize) as *mut Gc<LuaClosure>
}
