use bytecode::instructions::HLInstr;
use irgen::register_map::Lifetime;

/// Represents a compiled function in Lua.
#[derive(Debug)]
pub struct CompiledFunc {
    index: usize,
    functions: Vec<usize>,
    lifetimes: Vec<Lifetime>,
    instrs: Vec<HLInstr>,
}

impl CompiledFunc {
    /// Create a new empty function with the given index.
    pub fn new(index: usize) -> CompiledFunc {
        CompiledFunc {
            index,
            functions: vec![],
            lifetimes: vec![],
            instrs: vec![],
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    /// Push the id of the function that was compiled in the scope of this function.
    pub fn push_func(&mut self, i: usize) {
        self.functions.push(i);
    }

    /// Get the lifetimes of the registers of this function.
    pub fn lifetimes(&self) -> &Vec<Lifetime> {
        &self.lifetimes
    }

    pub fn set_lifetimes(&mut self, lifetimes: Vec<Lifetime>) {
        self.lifetimes = lifetimes;
    }

    /// Add an instruction to this function.
    pub fn push_instr(&mut self, instr: HLInstr) {
        self.instrs.push(instr);
    }

    /// Get a reference to all the instructions of this function.
    pub fn instrs(&self) -> &Vec<HLInstr> {
        &self.instrs
    }

    pub(crate) fn extract_functions(self) -> Vec<usize> {
        self.functions
    }
}
