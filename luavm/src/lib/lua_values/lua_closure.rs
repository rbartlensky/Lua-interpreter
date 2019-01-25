use gc::GcCell;

/// Represents a closure in Lua.
#[derive(Trace, Finalize)]
pub struct LuaClosure {
    index: usize,
    args_count: GcCell<usize>,
    args_start: GcCell<usize>,
}

impl LuaClosure {
    /// Creates an empty closure.
    pub fn new(index: usize) -> LuaClosure {
        LuaClosure {
            index,
            args_count: GcCell::new(0),
            args_start: GcCell::new(0),
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn args_count(&self) -> usize {
        self.args_count.borrow().clone()
    }

    pub fn set_args_count(&self, count: usize) {
        *self.args_count.borrow_mut() = count;
    }

    pub fn args_start(&self) -> usize {
        self.args_start.borrow().clone()
    }

    pub fn set_args_start(&self, count: usize) {
        *self.args_start.borrow_mut() = count;
    }
}
