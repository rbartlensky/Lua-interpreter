use super::instr::{Arg, Instr};
use super::register_map::RegisterMap;
use bytecode::instructions::Opcode;

pub struct BasicBlock {
    instrs: Vec<Instr>,
}

impl BasicBlock {
    pub fn new() -> BasicBlock {
        BasicBlock { instrs: vec![] }
    }

    pub fn push_instr(&mut self, opcode: Opcode, args: Vec<Arg>) {
        self.instrs.push(Instr { opcode, args });
    }

    pub fn instrs(&self) -> &Vec<Instr> {
        &self.instrs
    }

    pub fn get_mut(&mut self, i: usize) -> &mut Instr {
        &mut self.instrs[i]
    }

    pub fn get_instr_with_opcode(&mut self, op: Opcode) -> &mut Instr {
        let mut index = 0;
        for i in (0..self.instrs.len()).rev() {
            if self.instrs[i].opcode == op {
                index = i;
                break;
            }
        }
        &mut self.instrs[index]
    }
}

/// Represents a compiled function in Lua.
pub struct CompiledFunc<'a> {
    reg_map: RegisterMap<'a>,
    param_count: usize,
    basic_blocks: Vec<BasicBlock>,
    is_vararg: bool,
}

impl<'a> CompiledFunc<'a> {
    /// Create a new empty function with the given index.
    pub fn new(param_count: usize, is_vararg: bool) -> CompiledFunc<'a> {
        CompiledFunc {
            reg_map: RegisterMap::new(),
            basic_blocks: vec![],
            param_count,
            is_vararg,
        }
    }

    pub fn blocks(&self) -> &Vec<BasicBlock> {
        &self.basic_blocks
    }

    pub fn create_block(&mut self) -> usize {
        self.basic_blocks.push(BasicBlock::new());
        self.basic_blocks.len() - 1
    }

    pub fn get_block(&mut self, i: usize) -> &mut BasicBlock {
        &mut self.basic_blocks[i]
    }

    pub fn reg_count(&self) -> usize {
        self.reg_map.reg_count()
    }

    pub fn reg_map(&mut self) -> &mut RegisterMap<'a> {
        &mut self.reg_map
    }

    pub fn is_vararg(&self) -> bool {
        self.is_vararg
    }

    pub fn set_vararg(&mut self, v: bool) {
        self.is_vararg = v;
    }

    pub fn param_count(&self) -> usize {
        self.param_count
    }

    pub fn set_param_count(&mut self, count: usize) {
        self.param_count = count;
    }
}
