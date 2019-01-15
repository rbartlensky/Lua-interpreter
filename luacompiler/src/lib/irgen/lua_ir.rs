use irgen::{compiled_func::CompiledFunc, constants_map::ConstantsMap};

/// Represents an IR in which all instructions are in SSA form.
pub struct LuaIR {
    pub functions: Vec<CompiledFunc>,
    pub main_func: usize,
    pub const_map: ConstantsMap,
}

impl LuaIR {
    pub fn new(functions: Vec<CompiledFunc>, main_func: usize, const_map: ConstantsMap) -> LuaIR {
        LuaIR {
            functions,
            main_func,
            const_map,
        }
    }
}
