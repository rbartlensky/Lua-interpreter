use bytecode::{Function, LuaBytecode};
use irgen::constants_map::ConstantsMap;
use irgen::lua_ir::LuaIR;

pub fn compile_to_bytecode(ir: LuaIR) -> LuaBytecode {
    LuaIRToLuaBc::new(ir).compile()
}

struct LuaIRToLuaBc<'a> {
    ir: LuaIR<'a>,
    const_map: ConstantsMap,
}

impl<'a> LuaIRToLuaBc<'a> {
    /// Compile the given LuaIR to LuaBytecode.
    fn new(ir: LuaIR) -> LuaIRToLuaBc {
        LuaIRToLuaBc {
            ir,
            const_map: ConstantsMap::new(),
        }
    }

    fn compile(mut self) -> LuaBytecode {
        let mut functions = Vec::with_capacity(self.ir.functions.len());
        for i in 0..self.ir.functions.len() {
            assert!(self.ir.functions[i].reg_map().reg_count() < 256);
            functions.push(self.compile_function(i));
        }
        LuaBytecode::new(functions, self.ir.main_func, self.const_map)
    }

    fn compile_function(&mut self, _i: usize) -> Function {
        Function::new()
    }
}
