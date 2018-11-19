use std::{collections::HashMap, vec::Vec};

/// Represents a tuple which is used to specify the lifetime of a register.
/// For example if a register is first used by the 4th instruction of the bytecode, and
/// used last by the 7th instruction, the register's lifetime would be (4, 8).
#[derive(PartialEq, Eq, Debug)]
pub struct Lifetime(usize, usize);

impl Lifetime {
    pub fn new(sp: usize) -> Lifetime {
        Lifetime(sp, sp + 1)
    }

    pub fn with_end_point(sp: usize, ep: usize) -> Lifetime {
        Lifetime(sp, ep)
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
/// track of their lifetimes. Each Lua module has its own register map.
pub struct RegisterMap<'a> {
    lifetimes: Vec<Lifetime>,
    reg_maps: Vec<HashMap<&'a str, usize>>,
}

impl<'a> RegisterMap<'a> {
    pub fn new() -> RegisterMap<'a> {
        RegisterMap {
            lifetimes: vec![],
            // the first map holds the variables of the module
            reg_maps: vec![HashMap::new()],
        }
    }

    /// Pushes a new map of registers. All new registers will be allocated in the newly
    /// created map.
    pub fn push_scope(&mut self) {
        self.reg_maps.push(HashMap::new());
    }

    /// Pops the last map of registers.
    pub fn pop_scope(&mut self) {
        self.reg_maps.pop();
    }

    /// Creates and returns a new register.
    pub fn get_new_reg(&mut self) -> usize {
        let lifetime = Lifetime::new(self.lifetimes.len());
        self.lifetimes.push(lifetime);
        self.lifetimes.len() - 1
    }

    /// Creates a mapping between <name> and a newly created register.
    pub fn create_reg(&mut self, name: &'a str) -> usize {
        let reg = self.get_new_reg();
        self.set_reg(name, reg);
        reg
    }

    /// Get the register of <name>.
    pub fn get_reg(&mut self, name: &'a str) -> usize {
        let lifetimes = &mut self.lifetimes;
        for map in self.reg_maps[1..].iter().rev() {
            if let Some(&reg) = map.get(name) {
                return reg;
            }
        }
        // In lua, if a variable is queried, but isn't in scope, a Nil is returned instead
        // If none of the maps have a definition for <name> that means we have to define
        // it ourselves in the map of the module (the first map in <reg_maps>).
        *self.reg_maps[0]
            .entry(name)
            .and_modify(|reg| {
                let len = lifetimes.len();
                lifetimes[*reg].set_end_point(len + 1);
            })
            .or_insert_with(|| {
                let lifetime = Lifetime::new(lifetimes.len());
                lifetimes.push(lifetime);
                lifetimes.len() - 1
            })
    }

    /// Set the register of <name> to <reg>.
    pub fn set_reg(&mut self, name: &'a str, reg: usize) {
        self.reg_maps.last_mut().unwrap().insert(name, reg);
    }

    /// Get the total number of registers that were needed.
    pub fn reg_count(self) -> usize {
        self.lifetimes.len()
    }

    pub(crate) fn get_lifetimes(self) -> Vec<Lifetime> {
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
    fn correctly_maps_strings_to_registers() {
        let mut rm = RegisterMap::new();
        // create a new register
        assert_eq!(rm.get_new_reg(), 0);
        // create a mapping
        assert_eq!(rm.create_reg("foo"), 1);
        assert_eq!(*rm.reg_maps[0].get("foo").unwrap(), 1);
        assert_eq!(rm.get_reg("foo"), 1);
        assert_eq!(*rm.reg_maps[0].get("foo").unwrap(), 1);
        assert_eq!(rm.get_reg("bar"), 2);
        assert_eq!(*rm.reg_maps[0].get("bar").unwrap(), 2);
        // create a new scope in which we define another foo
        rm.push_scope();
        assert_eq!(rm.create_reg("foo"), 3);
        assert_eq!(*rm.reg_maps[1].get("foo").unwrap(), 3);
        assert_eq!(rm.get_reg("foo"), 3);
        assert_eq!(*rm.reg_maps[1].get("foo").unwrap(), 3);
        assert_eq!(rm.get_reg("bar"), 2);
        assert_eq!(*rm.reg_maps[0].get("bar").unwrap(), 2);
        assert!(rm.reg_maps[1].get("bar").is_none());
        rm.pop_scope();
        // pop the scope and query foo and bar again to check if they have the same values
        assert_eq!(rm.get_reg("foo"), 1);
        assert_eq!(*rm.reg_maps[0].get("foo").unwrap(), 1);
        assert_eq!(rm.get_reg("bar"), 2);
        assert_eq!(*rm.reg_maps[0].get("bar").unwrap(), 2);
        // test total number of registers created
        assert_eq!(rm.reg_count(), 4);
    }

    #[test]
    fn lifetimes_are_correcly_updated() {
        let mut rm = RegisterMap::new();
        let reg1 = rm.get_new_reg();
        assert_eq!(rm.lifetimes[reg1].0, 0);
        assert_eq!(rm.lifetimes[reg1].1, 1);
        let reg2 = rm.create_reg("reg");
        assert_eq!(rm.lifetimes[reg2].0, 1);
        assert_eq!(rm.lifetimes[reg2].1, 2);
        rm.get_reg("reg");
        assert_eq!(rm.lifetimes[reg2].0, 1);
        assert_eq!(rm.lifetimes[reg2].1, 3);
        rm.push_scope();
        let reg3 = rm.create_reg("reg3");
        rm.pop_scope();
        assert_eq!(rm.lifetimes[reg3].0, 2);
        assert_eq!(rm.lifetimes[reg3].1, 3);
        assert_eq!(rm.reg_count(), 3);
    }

    #[test]
    fn registers_are_retrieved_in_the_correct_order() {
        let mut rm = RegisterMap::new();
        for i in 0..3 {
            rm.push_scope();
            rm.create_reg("foo");
        }
        for i in 0..3 {
            assert_eq!(rm.get_reg("foo"), 2 - i);
            rm.pop_scope();
        }
    }
}
