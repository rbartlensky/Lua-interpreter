use std::vec::Vec;
use std::cell::RefCell;
use std::collections::HashMap;
use bytecode::instructions::Reg;

pub struct RegisterMap {
    registers: RefCell<Vec<Reg>>,
    reg_map: RefCell<HashMap<String, usize>>
}

impl RegisterMap {
    pub fn new() -> RegisterMap {
        RegisterMap {
            registers: RefCell::new(vec![]),
            reg_map: RefCell::new(HashMap::new())
        }
    }

    /// Generates a fresh register and returns it.
    /// This is used in cases like `x = 1 + 2 + 3` to generate intermmediate
    /// registers in which, for instance, the result of 2 + 3 is stored.
    pub fn new_reg(&self) -> usize {
        let len = self.registers.borrow().len();
        self.registers.borrow_mut().push(Reg::new(len));
        len + 1
    }

    /// Get the register that corresponds to the given identifier.
    /// If the corresponding register is not found, a new register is created
    /// and returned.
    pub fn get_reg(&self, name: &str) -> usize {
        *self.reg_map.borrow_mut().entry(name.to_string()).or_insert(self.new_reg())
    }

    pub fn get_registers(self) -> Vec<Reg> {
        self.registers.borrow_mut().to_vec()
    }
}
