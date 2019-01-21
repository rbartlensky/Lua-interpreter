use bytecode::{Function, LuaBytecode};
use irgen::lua_ir::LuaIR;

pub fn compile_to_bytecode(ir: LuaIR) -> LuaBytecode {
    LuaIRToLuaBc::new(ir).compile()
}

struct LuaIRToLuaBc {
    ir: LuaIR,
}

impl LuaIRToLuaBc {
    /// Compile the given LuaIR to LuaBytecode.
    fn new(ir: LuaIR) -> LuaIRToLuaBc {
        LuaIRToLuaBc { ir }
    }

    fn compile(self) -> LuaBytecode {
        let functions = self.ir.functions;
        assert!(functions[self.ir.main_func].lifetimes().len() < 256);
        LuaBytecode::new(
            functions.into_iter().map(|i| Function::from(i)).collect(),
            self.ir.main_func,
            self.ir.const_map,
        )
    }
}
