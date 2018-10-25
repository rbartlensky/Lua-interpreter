use std::cell::RefCell;
use std::collections::HashMap;

/// Represents a structure that is used to keep track of the mapping
/// between Lua variables and register ids.
pub struct RegisterMap {
    reg_count: RefCell<u8>,
    reg_map: RefCell<HashMap<String, u8>>
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
    pub fn new_reg(&self) -> u8 {
        let to_return = *self.reg_count.borrow();
        *self.reg_count.borrow_mut() += 1;
        to_return
    }

    /// Get the register that corresponds to the given identifier.
    /// If the corresponding register is not found, a new register is created
    /// and returned.
    pub fn get_reg(&self, name: &str) -> u8 {
        *self.reg_map.borrow_mut().entry(name.to_string())
            .or_insert_with(|| self.new_reg())
    }

    /// Get the total number of registers that were needed.
    pub fn reg_count(self) -> u8 {
        *self.reg_count.borrow_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_reg_correctly_increments_counter() {
        let rm = RegisterMap::new();
        for i in 0..10 {
            assert_eq!(rm.new_reg(), i);
            assert_eq!(*rm.reg_count.borrow(), i+1);
        }
        assert_eq!(rm.reg_count(), 10);
    }

    #[test]
    fn get_reg_correctly_maps_strings_to_registers() {
        let rm = RegisterMap::new();
        // create a new register
        assert_eq!(rm.new_reg(), 0);
        assert_eq!(*rm.reg_count.borrow(), 1);
        // create a mapping
        assert_eq!(rm.get_reg("foo"), 1);
        assert_eq!(*rm.reg_count.borrow(), 2);
        assert_eq!(*rm.reg_map.borrow().get("foo").unwrap(), 1);
        assert_eq!(rm.get_reg("foo"), 1);
        assert_eq!(*rm.reg_map.borrow().get("foo").unwrap(), 1);
        assert_eq!(*rm.reg_count.borrow(), 2);
        // test total number of registers created
        assert_eq!(rm.reg_count(), 2);
    }
}
