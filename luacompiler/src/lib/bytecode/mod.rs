pub mod instructions;

use self::instructions::format_instr;
use bincode::{deserialize, serialize};
use bytecodegen::constants_map::ConstantsMap;
use irgen::compiled_func::ProviderType;
use std::{
    collections::HashMap,
    fmt,
    fs::File,
    io::{self, Write},
    vec::Vec,
};

#[derive(Serialize, Deserialize)]
pub enum BCProviderType {
    Reg(u8),
    Upval(u8),
}

impl From<&ProviderType> for BCProviderType {
    fn from(pt: &ProviderType) -> Self {
        match pt {
            ProviderType::Reg(r) => {
                if *r <= u8::max_value() as usize {
                    BCProviderType::Reg(*r as u8)
                } else {
                    panic!("ProviderType argument is too large!")
                }
            }
            ProviderType::Upval(r) => {
                if *r <= u8::max_value() as usize {
                    BCProviderType::Upval(*r as u8)
                } else {
                    panic!("ProviderType argument is too large!")
                }
            }
        }
    }
}

/// Represents a function in Lua.
#[derive(Serialize, Deserialize)]
pub struct Function {
    index: usize,
    reg_count: usize,
    param_count: usize,
    upvals_count: usize,
    provides: HashMap<u8, Vec<(BCProviderType, u8)>>,
    instrs: Vec<u32>,
}

impl Function {
    pub fn new(
        index: usize,
        reg_count: usize,
        param_count: usize,
        upvals_count: usize,
        provides: HashMap<u8, Vec<(BCProviderType, u8)>>,
        instrs: Vec<u32>,
    ) -> Function {
        Function {
            index,
            reg_count,
            param_count,
            upvals_count,
            provides,
            instrs,
        }
    }

    /// Create a function which holds the given instructions.
    pub fn from_u32_instrs(instrs: Vec<u32>) -> Function {
        Function {
            index: 0,
            reg_count: 0,
            param_count: 0,
            upvals_count: 0,
            provides: HashMap::new(),
            instrs,
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn instrs_len(&self) -> usize {
        self.instrs.len()
    }

    pub fn get_instr(&self, i: usize) -> u32 {
        self.instrs[i]
    }

    /// The total number of registers that this function uses.
    pub fn reg_count(&self) -> usize {
        self.reg_count
    }

    pub fn param_count(&self) -> usize {
        self.param_count
    }
}

/// A simpler representation of Lua
#[derive(Serialize, Deserialize)]
pub struct LuaBytecode {
    ints: Vec<i64>,
    floats: Vec<f64>,
    strings: Vec<String>,
    functions: Vec<Function>,
    main_function: usize,
}

impl LuaBytecode {
    /// Create a new bytecode structure.
    /// * `main_function` - the id of the main function
    /// * `const_map` - a mapping between constants and their index in the constant table
    pub fn new(
        functions: Vec<Function>,
        main_function: usize,
        const_map: ConstantsMap,
    ) -> LuaBytecode {
        LuaBytecode {
            ints: const_map.get_ints(),
            floats: const_map.get_floats(),
            strings: const_map.get_strings(),
            functions,
            main_function,
        }
    }

    /// Create a new bytecode structure out of the given bytes.
    /// * `bytes` - the serialized version of a LuaBytecode instance
    /// # Panics
    /// This panics if the given vector of bytes does not represent a LuaBytecode instance
    pub fn new_from_bytes(bytes: Vec<u8>) -> LuaBytecode {
        deserialize(&bytes[..]).unwrap()
    }

    pub fn get_function(&self, i: usize) -> &Function {
        &self.functions[i]
    }

    pub fn get_main_function(&self) -> usize {
        self.main_function
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

    pub fn strings(&self) -> &Vec<String> {
        &self.strings
    }

    /// Gets the size of the string constant table.
    pub fn get_strings_len(&self) -> usize {
        self.strings.len()
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
        for (i, function) in self.functions.iter().enumerate() {
            write!(f, "Function {} {{\n", i)?;
            for instr in &function.instrs {
                write!(f, "  {}\n", format_instr(*instr))?;
            }
            write!(f, "}}\n")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::remove_file, io::Read};

    fn setup() -> LuaBytecode {
        // x = 1 + 2 * 3 / 2 ^ 2.0 // 1 - 2
        let instrs = vec![
            1, 273, 545, 8502, 321, 82, 21610, 25463, 129, 34713, 37028, 433, 47813, 3280,
        ];
        let mut const_map = ConstantsMap::new();
        for i in vec![1, 2, 3] {
            const_map.get_int(i);
        }
        for i in vec!["2.0"] {
            const_map.get_float(i.to_string());
        }
        let function = Function::from_u32_instrs(instrs);
        LuaBytecode::new(vec![function], 0, const_map)
    }

    #[test]
    fn bytecode_works_correctly() {
        let bc = setup();
        let function = bc.get_function(bc.get_main_function());
        assert_eq!(function.instrs_len(), 14);
        assert_eq!(function.get_instr(0), 1);
    }

    #[test]
    fn bytecode_serialize_deserialize() {
        let bc = setup();
        let name = "test_file.luabc";
        bc.serialize_to_file(&name)
            .expect("Failed to serialized to file.");
        let mut file = File::open(&name).unwrap();
        let mut contents = vec![];
        file.read_to_end(&mut contents).unwrap();
        remove_file(name).unwrap();
        let bc2 = LuaBytecode::new_from_bytes(contents);
        let function = bc.get_function(bc.get_main_function());
        let function2 = bc2.get_function(bc2.get_main_function());
        assert_eq!(function.reg_count, function2.reg_count);
        assert_eq!(bc.ints, bc2.ints);
        assert_eq!(bc.floats.len(), bc2.floats.len());
        assert_eq!(bc.strings, bc2.strings);
        assert_eq!(function.instrs, function2.instrs);
    }
}
