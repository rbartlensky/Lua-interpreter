/// Represents a closure in Lua.
#[derive(Trace, Finalize)]
pub struct LuaClosure {
    index: usize,
}

impl LuaClosure {
    /// Creates an empty closure.
    pub fn new(index: usize) -> LuaClosure {
        LuaClosure { index }
    }

    /// Get which chunk to jump to when the closure is called.
    pub fn index(&self) -> usize {
        self.index
    }
}
