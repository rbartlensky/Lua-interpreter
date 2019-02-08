use irgen::compiled_func::CompiledFunc;

/// Represents an IR in which all instructions are in SSA form.
pub struct LuaIR<'a> {
    pub functions: Vec<CompiledFunc<'a>>,
    pub main_func: usize,
}

impl<'a> LuaIR<'a> {
    pub fn new(functions: Vec<CompiledFunc<'a>>, main_func: usize) -> LuaIR<'a> {
        LuaIR {
            functions,
            main_func,
        }
    }
}
