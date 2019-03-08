use gc::gc::GcBox;
use lua_values::gc_val::GcVal;
use std::{mem::size_of, ops::BitXor};

/// Used for extracting the tag out of an address.
pub const MASK: usize = size_of::<usize>() - 1;
/// Used to shift raw integers in order to encode the INT tag.
pub const TAG_SHIFT: usize = 3;

/// Represents the type of a lua value.
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum LuaValKind {
    BOXED = 0,
    INT = 1,
    FLOAT = 2,
    Gc = 3,
    GcRoot = 4,
    BOOL = 5,
    NIL = 6,
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
                3 => LuaValKind::Gc,
                4 => LuaValKind::GcRoot,
                5 => LuaValKind::BOOL,
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

#[inline(always)]
pub fn set_tag(ptr: usize, tag: LuaValKind) -> usize {
    let old_tag = ptr & MASK;
    tag.clone() ^ (ptr ^ old_tag)
}

#[inline(always)]
pub fn untag(ptr: usize) -> usize {
    ptr ^ (ptr & MASK)
}

/// Untags the given pointer, and returns a mutable pointer to Gc<LuaTable>.
#[inline(always)]
pub fn gc_ptr(encoded_ptr: usize) -> *mut GcBox<Box<dyn GcVal>> {
    untag(encoded_ptr) as *mut GcBox<Box<dyn GcVal>>
}
