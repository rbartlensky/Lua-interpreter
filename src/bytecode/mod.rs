pub mod instructions;

use std::vec::Vec;
use std::fmt;
use self::instructions::Instr;

pub struct LuaBytecode {
    block: Vec<Instr>,
    ids: u64
}

impl LuaBytecode {
    pub fn new() -> LuaBytecode {
        LuaBytecode {
            block: vec![],
            ids: 0
        }
    }

    pub fn get_instrs(&self) -> &Vec<Instr> {
        &self.block
    }

    pub fn add_instr(&mut self, instr: Instr) {
        self.block.push(instr);
    }

    pub fn get_new_var(&mut self) -> String {
        let id = self.ids;
        self.ids += 1;
        format!("${}", id).to_string()
    }
}

impl fmt::Display for LuaBytecode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut to_write = String::from("{\n");
        for ref instr in &self.block {
            let repr = instr.to_string();
            to_write = format!("{}  {}\n", to_write, &repr)
        }
        to_write += "}";
        write!(f, "{}", to_write)
    }
}
