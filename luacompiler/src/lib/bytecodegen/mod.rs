mod register_allocator;

use self::register_allocator::LinearScan;
use bytecode::LuaBytecode;
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
        // println!("{:#?}", self.ir.instrs);
        LinearScan::get_reg_allocation(2, &self.ir.lifetimes);
        LuaBytecode::new(
            self.ir.instrs.iter().map(|i| i.to_32bit()).collect(),
            self.ir.const_map,
            self.ir.lifetimes.len() as u8,
        )
    }
}
