use std::{collections::HashMap, vec::Vec};

pub struct RegisterMap<'a> {
    blocks: HashMap<usize, HashMap<&'a str, usize>>,
    reg_maps: Vec<HashMap<&'a str, usize>>,
    reg_count: usize,
}

impl<'a> RegisterMap<'a> {
    pub fn new() -> RegisterMap<'a> {
        RegisterMap {
            blocks: HashMap::new(),
            reg_maps: vec![],
            reg_count: 0,
        }
    }

    /// Pushes a new map of registers. All new registers will be allocated in the newly
    /// created map.
    pub fn push_block(&mut self) {
        self.reg_maps.push(HashMap::new());
    }

    /// Pops the last map of registers.
    pub fn pop_block(&mut self, block: usize) {
        let map = self.reg_maps.pop();
        self.blocks.insert(block, map.unwrap());
    }

    /// Creates and returns a new register.
    pub fn get_new_reg(&mut self) -> usize {
        self.reg_count += 1;
        self.reg_count - 1
    }

    /// Creates a mapping between <name> and a newly created register.
    pub fn create_reg(&mut self, name: &'a str) -> usize {
        let reg = self.get_new_reg();
        self.set_reg(name, reg);
        reg
    }

    /// Get the register of <name>.
    pub fn get_reg(&mut self, name: &'a str) -> Option<usize> {
        for map in self.reg_maps.iter().rev() {
            if let Some(&reg) = map.get(name) {
                return Some(reg);
            }
        }
        None
    }

    /// Set the register of <name> to <reg>.
    pub fn set_reg(&mut self, name: &'a str, reg: usize) {
        self.reg_maps.last_mut().unwrap().insert(name, reg);
    }

    /// Get the total number of registers that were needed.
    pub fn reg_count(&self) -> usize {
        self.reg_count
    }

    pub fn is_local(&self, name: &str) -> bool {
        self.reg_maps
            .iter()
            .find(|x| x.contains_key(name))
            .is_some()
    }

    pub fn pop_last_reg(&mut self) {
        self.reg_count -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_reg_correctly_increments_counter() {
        let mut rm = RegisterMap::new();
        rm.push_block();
        for i in 0..10 {
            assert_eq!(rm.get_new_reg(), i);
        }
        assert_eq!(rm.reg_count(), 10);
    }

    #[test]
    fn correctly_maps_const_strs_to_registers() {
        let mut rm = RegisterMap::new();
        rm.push_block();
        // create a new register
        assert_eq!(rm.get_new_reg(), 0);
        // create a mapping
        assert_eq!(rm.create_reg("foo"), 1);
        assert_eq!(*rm.reg_maps[0].get("foo").unwrap(), 1);
        assert_eq!(rm.get_reg("foo"), Some(1));
        assert_eq!(*rm.reg_maps[0].get("foo").unwrap(), 1);
        assert_eq!(rm.get_reg("bar"), None);
        assert!(rm.reg_maps[0].get("bar").is_none());
        // create a new block in which we define another foo
        rm.push_block();
        assert_eq!(rm.create_reg("foo"), 2);
        assert_eq!(*rm.reg_maps[1].get("foo").unwrap(), 2);
        assert_eq!(rm.get_reg("foo"), Some(2));
        assert_eq!(*rm.reg_maps[1].get("foo").unwrap(), 2);
        assert_eq!(rm.get_reg("bar"), None);
        assert!(rm.reg_maps[1].get("bar").is_none());
        rm.pop_block(0);
        // pop the block and query foo and bar again to check if they have the same values
        assert_eq!(rm.get_reg("foo"), Some(1));
        assert_eq!(*rm.reg_maps[0].get("foo").unwrap(), 1);
        assert!(rm.get_reg("bar").is_none());
        assert!(rm.reg_maps[0].get("bar").is_none());
        // test total number of registers created
        assert_eq!(rm.reg_count(), 3);
    }

    #[test]
    fn registers_are_retrieved_in_the_correct_order() {
        let mut rm = RegisterMap::new();
        rm.push_block();
        for _ in 0..3 {
            rm.push_block();
            rm.create_reg("foo");
        }
        for i in 0..3 {
            assert_eq!(rm.get_reg("foo"), Some(2 - i));
            rm.pop_block(0);
        }
    }
}
