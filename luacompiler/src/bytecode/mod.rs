pub mod instructions;

use bincode::{ serialize, deserialize };
use std::vec::Vec;
use std::fs::File;
use std::fmt;
use std::io::{ self, Write };
use constants_map::ConstantsMap;

/// A simpler representation of Lua
#[derive(Serialize, Deserialize)]
pub struct LuaBytecode {
    reg_count: u8,
    ints: Vec<i64>,
    floats: Vec<f64>,
    strings: Vec<String>,
    block: Vec<u32>
}

impl LuaBytecode {
    /// Create a new bytecode structure.
    /// * `instrs` - the instructions of the bytecode
    /// * `const_map` - a mapping between constants and their index in the constant table
    /// * `reg_count` - the total number of registers used by the instructions
    pub fn new(instrs: Vec<u32>, const_map: ConstantsMap, reg_count: u8) -> LuaBytecode {
        LuaBytecode {
            reg_count,
            ints: const_map.get_ints(),
            floats: const_map.get_floats(),
            strings: const_map.get_strings(),
            block: instrs
        }
    }

    /// Create a new bytecode structure out of the given bytes.
    /// * `bytes` - the serialized version of a LuaBytecode instance
    /// # Panics
    /// This panics if the given vector of bytes does not represent a LuaBytecode instance
    pub fn new_from_bytes(bytes: Vec<u8>) -> LuaBytecode {
        deserialize(&bytes[..]).unwrap()
    }

    /// Get the number of instructions that are part of this block.
    pub fn instrs_len(&self) -> usize {
        self.block.len()
    }

    /// Get the list of instructions that can be executed in order
    /// to perform some computation.
    pub fn get_instr(&self, index: usize) -> u32 {
        self.block[index]
    }

    /// Get the number of registers that this bytecode uses in order to encode
    /// instructions.
    pub fn reg_count(&self) -> u8 {
        self.reg_count
    }

    /// Retrieve the integer at index <i> in the constant table.
    pub fn get_int(&self, i: u8) -> i64 {
        self.ints[i as usize]
    }

    /// Retrieve the float at index <i> in the constant table.
    pub fn get_float(&self, i: u8) -> f64 {
        self.floats[i as usize]
    }

    /// Retrieve the string at index <i> in the constant table.
    pub fn get_string(&self, i: u8) -> &str {
        &self.strings[i as usize]
    }

    /// Serialize the bytecode to a file using bincode.
    pub fn serialize_to_file(&self, file: &str) -> io::Result<()> {
        let mut f = File::create(file)?;
        let encoded: Vec<u8> = serialize(&self).unwrap();
        Ok(f.write_all(encoded.as_slice())?)
    }
}

impl fmt::Display for LuaBytecode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{\n");
        for instr in &self.block {
            write!(f, "  {}\n", instr);
        }
        write!(f, "}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::remove_file;
    use std::io::Read;

    fn setup() -> LuaBytecode {
        // x = 1 + 2 * 3 / 2 ^ 2.0 // 1 - 2
        let instrs = vec![
            1, 273, 545, 8502, 321, 82, 21610, 25463,
            129, 34713, 37028, 433, 47813, 3280
        ];
        let mut const_map = ConstantsMap::new();
        for i in vec![1, 2, 3] {
            const_map.get_int(i);
        }
        for i in vec!["2.0"] {
            const_map.get_float(i.to_string());
        }
        LuaBytecode::new(instrs, const_map, 14)
    }

    #[test]
    fn bytecode_works_correctly() {
        let bc = setup();
        assert_eq!(bc.reg_count(), 14);
        assert_eq!(bc.instrs_len(), 14);
        assert_eq!(bc.get_instr(0), 1);
    }

    #[test]
    fn bytecode_serialize_deserialize() {
        let bc = setup();
        let name = "test_file.luabc";
        bc.serialize_to_file(&name).expect("Failed to serialized to file.");
        let mut file = File::open(&name).unwrap();
        let mut contents = vec![];
        file.read_to_end(&mut contents).unwrap();
        remove_file(name).unwrap();
        let bc2 = LuaBytecode::new_from_bytes(contents);
        assert_eq!(bc.reg_count, bc2.reg_count);
        assert_eq!(bc.ints, bc2.ints);
        assert_eq!(bc.floats.len(), bc2.floats.len());
        assert_eq!(bc.strings, bc2.strings);
        assert_eq!(bc.block, bc2.block);
    }
}
