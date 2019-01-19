use bytecode::instructions::HLInstr;
use irgen::register_map::{Lifetime, RegisterMap};

/// Represents a compiled function in Lua.
pub struct CompiledFunc<'a> {
    index: usize,
    functions: Vec<usize>,
    instrs: Vec<HLInstr>,
    reg_map: RegisterMap<'a>,
    param_count: usize,
}

impl<'a> CompiledFunc<'a> {
    /// Create a new empty function with the given index.
    pub fn new(index: usize) -> CompiledFunc<'a> {
        CompiledFunc {
            index,
            functions: vec![],
            instrs: vec![],
            reg_map: RegisterMap::new(),
            param_count: 0,
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    /// Push the id of the function that was compiled in the scope of this function.
    pub fn push_func(&mut self, i: usize) {
        self.functions.push(i);
    }

    pub fn funcs_len(&self) -> usize {
        self.functions.len()
    }

    /// Add an instruction to this function.
    pub fn push_instr(&mut self, instr: HLInstr) {
        self.instrs.push(instr);
    }

    /// Get a reference to all the instructions of this function.
    pub fn instrs(&self) -> &Vec<HLInstr> {
        &self.instrs
    }

    pub fn reg_map(&self) -> &RegisterMap<'a> {
        &self.reg_map
    }

    pub fn mut_reg_map(&mut self) -> &mut RegisterMap<'a> {
        &mut self.reg_map
    }

    pub fn lifetimes(&self) -> &Vec<Lifetime> {
        self.reg_map.lifetimes()
    }

    pub(crate) fn extract_functions(self) -> Vec<usize> {
        self.functions
    }

    pub fn param_count(&self) -> usize {
        self.param_count
    }

    pub fn set_param_count(&mut self, count: usize) {
        self.param_count = count;
    }
}
