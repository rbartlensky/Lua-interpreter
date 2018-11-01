use bytecode::instructions::HLInstr;
use irgen::{constants_map::ConstantsMap, register_map::Lifetime};

/// Represents an IR in which all instructions are in SSA form.
pub struct LuaIR {
    pub instrs: Vec<HLInstr>,
    pub const_map: ConstantsMap,
    pub lifetimes: Vec<Lifetime>,
}

impl LuaIR {
    pub fn new(
        instrs: Vec<HLInstr>,
        const_map: ConstantsMap,
        mut lifetimes: Vec<Lifetime>,
    ) -> LuaIR {
        lifetimes.sort_by(|x, y| x.start_point().cmp(&y.start_point()));
        LuaIR {
            instrs,
            const_map,
            lifetimes,
        }
    }
}
