use std::cell::RefCell;
use std::collections::HashMap;

/// Represents a structure that is used to keep track of the mapping
/// between Lua variables and register ids.
pub struct RegisterMap {
    reg_count: RefCell<usize>,
    reg_map: RefCell<HashMap<String, usize>>
}

impl RegisterMap {
    pub fn new() -> RegisterMap {
        RegisterMap {
            reg_count: RefCell::new(0),
            reg_map: RefCell::new(HashMap::new())
        }
    }

    /// Generates a fresh register and returns it.
    /// This is used in cases like `x = 1 + 2 + 3` to generate intermmediate
    /// registers in which, for instance, the result of 2 + 3 is stored.
    pub fn new_reg(&self) -> usize {
        let to_return = *self.reg_count.borrow();
        *self.reg_count.borrow_mut() += 1;
        to_return
    }

    /// Get the register that corresponds to the given identifier.
    /// If the corresponding register is not found, a new register is created
    /// and returned.
    pub fn get_reg(&self, name: &str) -> usize {
        *self.reg_map.borrow_mut().entry(name.to_string()).or_insert(self.new_reg())
    }

    /// Get the total number of registers that were needed.
    pub fn reg_count(self) -> usize {
        *self.reg_count.borrow_mut()
    }
}
