use irgen::{compiled_func::CompiledFunc, constants_map::ConstantsMap};

/// Represents an IR in which all instructions can address 2^64 registers.
pub struct LuaIR<'a> {
    pub functions: Vec<CompiledFunc<'a>>,
    pub main_func: usize,
    pub const_map: ConstantsMap,
}

impl<'a> LuaIR<'a> {
    pub fn new(
        functions: Vec<CompiledFunc<'a>>,
        main_func: usize,
        const_map: ConstantsMap,
    ) -> LuaIR<'a> {
        LuaIR {
            functions,
            main_func,
            const_map,
        }
    }
}
