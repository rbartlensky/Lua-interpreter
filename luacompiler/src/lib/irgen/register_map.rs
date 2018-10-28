use std::{collections::HashMap, vec::Vec};

/// Represents a tuple which is used to specify the lifetime of a register.
/// For example if a register is first used by the 4th instruction of the bytecode, and
/// used last by the 7th instruction, the register's lifetime would be (4, 8).
pub struct Lifetime(usize, usize);

impl Lifetime {
    pub fn new(sp: usize) -> Lifetime {
        Lifetime(sp, sp + 1)
    }

    /// Get the start point of the register.
    pub fn start_point(&self) -> usize {
        self.0
    }

    /// Get the end point of the register.
    pub fn end_point(&self) -> usize {
        self.1
    }

    fn set_end_point(&mut self, ep: usize) {
        self.1 = ep
    }
}

/// Represents a structure that is used to map Lua variables to registers, and to keep
/// track of their lifetimes.
pub struct RegisterMap<'a> {
    current_instr: usize,
    lifetimes: Vec<Lifetime>,
    reg_map: HashMap<&'a str, usize>,
}

impl<'a> RegisterMap<'a> {
    pub fn new() -> RegisterMap<'a> {
        RegisterMap {
            current_instr: 0,
            lifetimes: vec![],
            reg_map: HashMap::new(),
        }
    }

    /// Increments the current instruction by 1. This method is used by the IR generator
    /// in order to signify that a new instruction has been processed.
    pub fn step(&mut self) {
        self.current_instr += 1;
    }

    /// Creates and returns a new register, whose lifetime begins from self.current_instr.
    pub fn get_new_reg(&mut self) -> usize {
        self.lifetimes.push(Lifetime::new(self.current_instr));
        self.lifetimes.len() - 1
    }

    /// Get the register of <name>, or create it if it doesn't exist.
    pub fn get_reg(&mut self, name: &'a str) -> usize {
        if let Some(&reg) = self.reg_map.get(name) {
            // the variable is referenced again, that means its lifetime is increased
            self.lifetimes[reg].set_end_point(self.current_instr + 1);
            reg
        } else {
            let new_reg = self.get_new_reg();
            self.reg_map.insert(name, new_reg);
            new_reg
        }
    }

    /// Set the register of <name> to <reg>.
    pub fn set_reg(&mut self, name: &'a str, reg: usize) {
        self.reg_map.insert(name, reg);
    }

    /// Get the total number of registers that were needed.
    pub fn reg_count(self) -> usize {
        self.lifetimes.len()
    }

    pub fn get_lifetimes(self) -> Vec<Lifetime> {
        self.lifetimes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_reg_correctly_increments_counter() {
        let mut rm = RegisterMap::new();
        for i in 0..10 {
            assert_eq!(rm.get_new_reg(), i);
        }
        assert_eq!(rm.reg_count(), 10);
    }

    #[test]
    fn get_reg_correctly_maps_strings_to_registers() {
        let mut rm = RegisterMap::new();
        // create a new register
        assert_eq!(rm.get_new_reg(), 0);
        // create a mapping
        assert_eq!(rm.get_reg("foo"), 1);
        assert_eq!(*rm.reg_map.get("foo").unwrap(), 1);
        assert_eq!(rm.get_reg("foo"), 1);
        assert_eq!(*rm.reg_map.get("foo").unwrap(), 1);
        // test total number of registers created
        assert_eq!(rm.reg_count(), 2);
    }

    #[test]
    fn lifetimes_are_correcly_updated() {
        let mut rm = RegisterMap::new();
        let reg1 = rm.get_new_reg();
        assert_eq!(rm.lifetimes[reg1].0, 0);
        assert_eq!(rm.lifetimes[reg1].1, 1);
        rm.step();
        let reg2 = rm.get_reg("reg");
        assert_eq!(rm.lifetimes[reg2].0, 1);
        assert_eq!(rm.lifetimes[reg2].1, 2);
        rm.step();
        rm.get_reg("reg");
        assert_eq!(rm.lifetimes[reg2].0, 1);
        assert_eq!(rm.lifetimes[reg2].1, 3);
        assert_eq!(rm.reg_count(), 2);
    }
}
