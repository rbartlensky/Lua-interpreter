use super::instr::{Arg, Instr};
use irgen::opcodes::IROpcode;
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashMap};

pub struct BasicBlock<'a> {
    parents: Vec<usize>,
    dominators: Vec<usize>,
    instrs: Vec<Instr>,
    non_locals: BTreeMap<&'a str, Vec<usize>>,
    locals: BTreeMap<&'a str, Vec<usize>>,
}

impl<'a> BasicBlock<'a> {
    pub fn new() -> BasicBlock<'a> {
        BasicBlock {
            parents: vec![],
            dominators: vec![],
            instrs: vec![],
            non_locals: BTreeMap::new(),
            locals: BTreeMap::new(),
        }
    }

    pub fn with_parents(parents: Vec<usize>) -> BasicBlock<'a> {
        BasicBlock {
            parents,
            dominators: vec![],
            instrs: vec![],
            non_locals: BTreeMap::new(),
            locals: BTreeMap::new(),
        }
    }

    pub fn instrs(&self) -> &Vec<Instr> {
        &self.instrs
    }

    pub fn mut_instrs(&mut self) -> &mut Vec<Instr> {
        &mut self.instrs
    }

    pub fn get(&self, i: usize) -> &Instr {
        &self.instrs[i]
    }

    pub fn get_mut(&mut self, i: usize) -> &mut Instr {
        &mut self.instrs[i]
    }

    pub fn get_instr_with_opcode(&mut self, op: IROpcode) -> &mut Instr {
        let mut index = 0;
        for i in (0..self.instrs.len()).rev() {
            if self.instrs[i].opcode() == op {
                index = i;
                break;
            }
        }
        &mut self.instrs[index]
    }

    pub fn parents(&self) -> &Vec<usize> {
        &self.parents
    }

    pub fn set_parents(&mut self, parents: Vec<usize>) {
        self.parents = parents;
    }

    pub fn push_parent(&mut self, parent: usize) {
        self.parents.push(parent);
    }

    pub fn dominators(&self) -> &Vec<usize> {
        &self.dominators
    }

    pub fn push_dominator(&mut self, bb: usize) {
        self.dominators.push(bb);
    }

    pub fn set_reg_name(&mut self, reg: usize, name: &'a str, is_local_decl: bool) {
        let mut instrs = vec![];
        // `local <name> = ...` is declared
        if is_local_decl {
            // check if we shadow any variable with the same name
            if let Some(non_locals_vec) = self.non_locals.get_mut(name) {
                // we modified an outer variable multiple times, thus emit a phi
                if non_locals_vec.len() > 1 {
                    BasicBlock::gen_phi(&mut instrs, non_locals_vec);
                }
            }
            self.locals
                .entry(name)
                .and_modify(|locals_vec| {
                    if locals_vec.len() > 1 {
                        BasicBlock::gen_phi(&mut instrs, locals_vec);
                    }
                    locals_vec[0] = reg;
                })
                .or_insert_with(|| vec![reg]);
        } else {
            if let Entry::Occupied(mut locals) = self.locals.entry(name) {
                locals.get_mut().push(reg);
                return;
            }
            let non_local_entry = self.non_locals.entry(name);
            if let Entry::Occupied(mut non_locals) = non_local_entry {
                non_locals.get_mut().push(reg);
                return;
            }
            non_local_entry.or_insert(vec![reg]);
        }
        self.instrs.extend(instrs);
    }

    fn gen_phi(instrs: &mut Vec<Instr>, var_vec: &mut Vec<usize>) {
        let merge_reg = var_vec[0];
        let phi_args: Vec<Arg> = var_vec.iter().map(|r| Arg::Reg(*r)).collect();
        let phi = Instr::NArg(IROpcode::Phi, phi_args);
        instrs.push(phi);
        var_vec.clear();
        var_vec.push(merge_reg);
    }

    pub fn generate_phis(&mut self) {
        let mut instrs = vec![];
        for (_, non_locals) in self.non_locals.iter_mut() {
            if non_locals.len() > 1 {
                BasicBlock::gen_phi(&mut instrs, non_locals);
            }
        }
        for (_, locals) in self.locals.iter_mut() {
            if locals.len() > 1 {
                BasicBlock::gen_phi(&mut instrs, locals);
            }
        }
        self.instrs.extend(instrs);
    }

    pub fn get_reg(&self, name: &'a str) -> Option<usize> {
        let res = self.locals.get(name);
        if res.is_some() {
            return res.map(|v| *v.last().unwrap());
        }
        self.non_locals.get(name).map(|v| *v.last().unwrap())
    }

    pub fn locals(&self) -> &BTreeMap<&'a str, Vec<usize>> {
        &self.locals
    }

    pub fn non_locals(&self) -> &BTreeMap<&'a str, Vec<usize>> {
        &self.non_locals
    }

    pub fn replace_regs_with(&mut self, regs: &[usize], with: usize) {
        for mut instr in &mut self.instrs {
            instr.replace_regs_with(regs, with);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProviderType {
    Upval(usize),
    Reg(usize),
}

/// Represents a compiled function in Lua.
pub struct IRFunc<'a> {
    parent_func: Option<usize>,
    parent_block: Option<usize>,
    upvals: BTreeMap<&'a str, usize>,
    // provides another function with the following upvalues
    provides: HashMap<usize, BTreeMap<usize, ProviderType>>,
    reg_count: usize,
    param_count: usize,
    basic_blocks: Vec<BasicBlock<'a>>,
    is_vararg: bool,
}

impl<'a> IRFunc<'a> {
    /// Create a new empty function with the given index.
    pub fn new(param_count: usize, is_vararg: bool) -> IRFunc<'a> {
        IRFunc {
            parent_func: None,
            parent_block: None,
            upvals: BTreeMap::new(),
            provides: HashMap::new(),
            reg_count: 0,
            param_count,
            basic_blocks: vec![],
            is_vararg,
        }
    }

    /// The function in which this was declared.
    /// Returns `None` if it is the top-level function.
    pub fn parent_func(&self) -> Option<usize> {
        self.parent_func
    }

    /// The block of the parent function in which this was declared.
    /// Returns `None` if it is the top-level function.
    pub fn parent_block(&self) -> Option<usize> {
        self.parent_block
    }

    /// Set the parent function, in which this was declared.
    pub fn set_parent_func(&mut self, p: usize) {
        self.parent_func = Some(p);
    }

    /// Set the parent block, in which this was declared.
    pub fn set_parent_block(&mut self, p: usize) {
        self.parent_block = Some(p);
    }

    /// The values that this function depends on, i.e. names which are declared
    /// in parent functions and used in the current function.
    pub fn upvals(&self) -> &BTreeMap<&'a str, usize> {
        &self.upvals
    }

    pub fn push_upval(&mut self, name: &'a str) -> usize {
        let len = self.upvals.len();
        self.upvals.insert(name, len);
        len
    }

    pub fn provides(&self) -> &HashMap<usize, BTreeMap<usize, ProviderType>> {
        &self.provides
    }

    pub fn provides_mut(&mut self) -> &mut HashMap<usize, BTreeMap<usize, ProviderType>> {
        &mut self.provides
    }

    pub fn push_provider(&mut self, func_idx: usize, upval_idx: usize, pt: ProviderType) {
        self.provides
            .entry(func_idx)
            .and_modify(|v| {
                v.insert(upval_idx, pt.clone());
            })
            .or_insert_with(|| {
                let mut set = BTreeMap::new();
                set.insert(upval_idx, pt);
                set
            });
    }

    pub fn get_new_reg(&mut self) -> usize {
        self.reg_count += 1;
        self.reg_count - 1
    }

    pub fn pop_last_reg(&mut self) {
        self.reg_count -= 1;
    }

    pub fn blocks(&self) -> &Vec<BasicBlock<'a>> {
        &self.basic_blocks
    }

    pub fn create_block(&mut self) -> usize {
        self.basic_blocks.push(BasicBlock::new());
        self.basic_blocks.len() - 1
    }

    pub fn create_block_with_parents(&mut self, parents: Vec<usize>) -> usize {
        self.basic_blocks.push(BasicBlock::with_parents(parents));
        self.basic_blocks.len() - 1
    }

    pub fn get_block(&self, i: usize) -> &BasicBlock<'a> {
        &self.basic_blocks[i]
    }

    pub fn get_mut_block(&mut self, i: usize) -> &mut BasicBlock<'a> {
        &mut self.basic_blocks[i]
    }

    pub fn reg_count(&self) -> usize {
        self.reg_count
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

    pub fn get_mut_blocks(&mut self) -> &mut Vec<BasicBlock<'a>> {
        &mut self.basic_blocks
    }
}
