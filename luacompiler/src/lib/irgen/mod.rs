pub mod compiled_func;
pub mod instr;
pub mod lua_ir;
pub mod opcodes;
mod utils;

use self::compiled_func::{BasicBlock, CompiledFunc, ProviderType};
use self::instr::{Arg, Instr};
use self::lua_ir::LuaIR;
use self::opcodes::IROpcode::*;
use self::utils::{find_term, get_nodes, is_nonterm, is_term};
use cfgrammar::RIdx;
use lrpar::Node::{self, *};
use lua5_3_l;
use lua5_3_y;
use std::collections::{BTreeSet, HashMap};
use LuaParseTree;

/// Compile the given parse tree into an SSA IR.
pub fn compile_to_ir(pt: &LuaParseTree) -> LuaIR {
    LuaToIR::new(pt).to_lua_ir()
}

#[derive(Debug, PartialEq, Eq)]
enum AssignmentType {
    /// Whether the assignment is a local one: `local a ...`.
    LocalDecl = 0,
    /// The environment will be updated by the caller.
    Postponed = 1,
    /// If the variable is global, then the environment is updated as well.
    Regular = 2,
}

enum ResultType {
    Local(usize),
    Global(usize),
    Dict(usize),
}

impl ResultType {
    fn get_reg(&self) -> usize {
        match *self {
            ResultType::Local(reg) => reg,
            ResultType::Global(reg) => reg,
            ResultType::Dict(..) => panic!("ResultType::Dict has no register"),
        }
    }
}

#[derive(Clone, Copy)]
enum VarType<'a> {
    Name(&'a str),
    // from_reg , attr_reg
    Dict(usize, usize),
}

impl<'a> VarType<'a> {
    fn get_str(&self) -> &'a str {
        match *self {
            VarType::Name(name) => name,
            VarType::Dict(..) => panic!("VarType::Dict has no name."),
        }
    }
}

/// Represents a compiler which translates a given Lua parse tree to an SSA IR.
struct LuaToIR<'a> {
    pt: &'a LuaParseTree,
    functions: Vec<CompiledFunc<'a>>,
    curr_func: usize,
    curr_block: usize,
}

impl<'a> LuaToIR<'a> {
    fn new(pt: &'a LuaParseTree) -> LuaToIR<'a> {
        let functions = vec![CompiledFunc::new(0, false)];
        LuaToIR {
            pt,
            functions,
            curr_func: 0,
            curr_block: 0,
        }
    }

    /// Compile and return the intermediate representation of the given lua parse tree.
    pub fn to_lua_ir(mut self) -> LuaIR<'a> {
        let new_block = self.curr_func().create_block();
        self.compile_block_in_basic_block(&self.pt.tree, new_block);
        for mut func in &mut self.functions {
            func.get_mut_blocks().last_mut().unwrap().generate_phis();
        }
        LuaIR::new(self.functions, 0)
    }

    fn curr_func(&mut self) -> &mut CompiledFunc<'a> {
        &mut self.functions[self.curr_func]
    }

    fn curr_block(&mut self) -> &mut BasicBlock<'a> {
        let i = self.curr_block;
        self.functions[self.curr_func].get_mut_block(i)
    }

    fn get_block(&mut self, i: usize) -> &mut BasicBlock<'a> {
        self.functions[self.curr_func].get_mut_block(i)
    }

    fn instrs(&mut self) -> &mut Vec<Instr> {
        self.curr_block().mut_instrs()
    }

    fn get_reg(&self, name: &'a str) -> Option<usize> {
        self.get_reg_from_block(name, self.curr_func, self.curr_block)
    }

    fn set_reg_name(&mut self, reg: usize, name: &'a str, local_decl: bool) {
        self.curr_block().set_reg_name(reg, name, local_decl);
    }

    fn generate_phis_for_bb(&mut self, bb: usize) {
        self.curr_func().get_mut_block(bb).generate_phis();
    }

    fn get_upval(&mut self, name: &'a str) -> Option<usize> {
        // does the current function define the upvalue already?
        if let Some(&upval_idx) = self.curr_func().upvals().get(name) {
            return Some(upval_idx);
        }
        let mut func = self.curr_func().parent_func();
        let mut block = self.curr_func().parent_block();
        // go through all the parents, and check if we can find <name>
        while func.is_some() {
            // found <name>, so we can create a new upvalue, and load it
            if let Some(reg) = self.get_reg_from_block(name, func.unwrap(), block.unwrap()) {
                let upval_idx = self.curr_func().push_upval(name);
                self.functions[func.unwrap()].push_provider(
                    self.curr_func,
                    upval_idx + 1,
                    ProviderType::Reg(reg),
                );
                return Some(upval_idx);
            } else {
                let p_func = &self.functions[func.unwrap()];
                block = p_func.parent_block();
                func = p_func.parent_func();
            }
        }
        None
    }

    fn set_upval(&mut self, name: &'a str, value: usize) {
        if let Some(upval_idx) = self.get_upval(name) {
            self.instrs().push(Instr::TwoArg(
                SetUpVal,
                Arg::Some(upval_idx + 1),
                Arg::Reg(value),
            ));
        } else {
            self.instrs().push(Instr::ThreeArg(
                SetUpAttr,
                Arg::Some(0),
                Arg::Str(name.to_string()),
                Arg::Reg(value),
            ));
        }
    }

    fn get_reg_from_block(&self, name: &'a str, func: usize, bb: usize) -> Option<usize> {
        let curr_func = &self.functions[func];
        let curr_block = curr_func.get_block(bb);
        let res = curr_block.get_reg(name);
        if res.is_some() {
            return res;
        }
        for &d in curr_block.dominators() {
            let res = self.get_reg_from_block(name, func, d);
            if res.is_some() {
                return res;
            }
        }
        None
    }

    fn is_locally_declared_in_doms(&self, name: &'a str) -> bool {
        let curr_func = &self.functions[self.curr_func];
        let curr_block = curr_func.get_block(self.curr_block);
        for &d in curr_block.dominators() {
            if curr_func.get_block(d).locals().contains_key(name) {
                return true;
            }
        }
        false
    }

    fn is_local(&self, name: &'a str) -> bool {
        self.get_reg(name).is_some()
    }

    /// Compile a <block>.
    fn compile_block(&mut self, node: &'a Node<u8>) -> usize {
        let parent = self.curr_block;
        self.generate_phis_for_bb(parent);
        let curr_block = self.curr_func().create_block_with_parents(vec![parent]);
        self.curr_block = curr_block;
        self.curr_block().push_dominator(parent);
        self.compile_block_in_basic_block(node, curr_block);
        curr_block
    }

    fn compile_block_in_basic_block(&mut self, node: &'a Node<u8>, i: usize) {
        let old_block = self.curr_block;
        self.curr_block = i;
        // nodes = [<statlistopt>, <retstatopt>]
        let nodes = get_nodes(node, lua5_3_y::R_BLOCK);
        self.compile_stat_list(&nodes[0]);
        self.compile_retstat(&nodes[1]);
        self.curr_block = old_block;
    }

    fn create_child_block(&mut self) -> usize {
        let parent = self.curr_block;
        self.generate_phis_for_bb(parent);
        let curr_block = self.curr_func().create_block_with_parents(vec![parent]);
        self.curr_block = curr_block;
        self.curr_block().push_dominator(parent);
        self.curr_block
    }

    /// Compile <retstatopt>
    fn compile_retstat(&mut self, node: &'a Node<u8>) {
        match *node {
            Nonterm {
                ridx: RIdx(ridx),
                ref nodes,
            } if ridx == lua5_3_y::R_RETSTATOPT => {
                if nodes.len() > 0 {
                    let exprs = self.get_underlying_exprs(&nodes[1]);
                    // push the first n-1 return values to the stack
                    for i in 0..(exprs.len() - 1) {
                        let reg = self.compile_expr(exprs[i]);
                        self.instrs().push(Instr::ThreeArg(
                            PUSH,
                            Arg::Reg(reg),
                            Arg::Some(0),
                            Arg::Some(1),
                        ));
                    }
                    self.unpack_to_stack(&exprs.last().unwrap(), true);
                    self.instrs().push(Instr::ZeroArg(RET));
                }
            }
            _ => panic!("Expected a <retstatopt>, but got {:#?}", node),
        }
    }

    fn unpack_to_stack(&mut self, last_expr: &'a Node<u8>, increment_ret_vals: bool) {
        let reg = self.compile_expr(last_expr);
        if self.is_unpackable(last_expr) {
            {
                let len = self.curr_block().instrs().len();
                // this is either a VarArg instr, or a MOVR
                let last_instr = self.curr_block().get_mut(len - 1);
                debug_assert!(last_instr.opcode() == MOVR || last_instr.opcode() == VarArg);
                // check bytecode/instructions.rs for more info on why we set the third
                // argument to 1 or 2
                *last_instr = Instr::OneArg(
                    last_instr.opcode(),
                    Arg::Some(1 + increment_ret_vals as usize),
                );
            }
            // compile_expr will generate (VarArg/MOVR <new_reg> <op2> <op3>)
            // but because we are modifying the last instruction, there is
            // no need to keep the previously allocated register
            self.curr_func().pop_last_reg();
        } else {
            if increment_ret_vals {
                self.instrs().push(Instr::ThreeArg(
                    PUSH,
                    Arg::Reg(reg),
                    Arg::Some(0),
                    Arg::Some(1),
                ));
            } else {
                self.instrs().push(Instr::OneArg(PUSH, Arg::Reg(reg)));
            }
        }
    }

    /// Compile a <statlist> or a <statlistopt>.
    fn compile_stat_list(&mut self, node: &'a Node<u8>) {
        match *node {
            Nonterm {
                ridx: RIdx(ridx),
                ref nodes,
            } if ridx == lua5_3_y::R_STATLIST => {
                // nodes = <stat>
                if nodes.len() == 1 {
                    self.compile_stat(get_nodes(&nodes[0], lua5_3_y::R_STAT));
                } else {
                    // nodes = [<statlist>, <stat>]
                    self.compile_stat_list(&nodes[0]);
                    self.compile_stat(get_nodes(&nodes[1], lua5_3_y::R_STAT));
                }
            }
            Nonterm {
                ridx: RIdx(ridx),
                ref nodes,
            } if ridx == lua5_3_y::R_STATLISTOPT => {
                // nodes = <statlist>
                if nodes.len() == 1 {
                    self.compile_stat_list(&nodes[0]);
                }
            }
            _ => panic!(
                "Expected a <statlist> or <statlistopt>, but got {:#?}",
                node
            ),
        }
    }

    /// Compile the children of a <stat> node.
    /// The method can only compile variable assignments.
    fn compile_stat(&mut self, stat_nodes: &'a Vec<Node<u8>>) {
        let len = stat_nodes.len();
        if len == 3 {
            // look for stat_nodes = [<local>, <namelist>, <eqexplistopt>]
            if is_term(&stat_nodes[0], lua5_3_l::T_LOCAL) {
                match stat_nodes[2] {
                    // nodes = [<eq>, <explist>]
                    Nonterm {
                        ridx: RIdx(ridx),
                        ref nodes,
                    } if ridx == lua5_3_y::R_EQEXPLISTOPT => {
                        let names = self.compile_names(&stat_nodes[1]);
                        let exprs = if nodes.len() > 0 {
                            self.get_underlying_exprs(&nodes[1])
                        } else {
                            vec![]
                        };
                        self.compile_local_assignments(names, exprs);
                    }
                    _ => {}
                }
            } else if is_term(&stat_nodes[0], lua5_3_l::T_DO) {
                self.compile_do_block(&stat_nodes[1]);
            } else {
                match (&stat_nodes[0], &stat_nodes[1]) {
                    // stat_nodes = [<function>, <funcname>, <funcbody>]
                    (Term { lexeme }, _) if lexeme.tok_id() == lua5_3_l::T_FUNCTION => {
                        let name = self.compile_var_or_name(&stat_nodes[1]);
                        self.compile_assignment(name, &stat_nodes[2], AssignmentType::Regular);
                    }
                    // stat_nodes = [<varlist>, <eq>, <explist>]
                    (_, Term { lexeme }) if lexeme.tok_id() == lua5_3_l::T_EQ => {
                        // x, y, z = 1, 2
                        let names = self.compile_var_list(&stat_nodes[0]);
                        let exprs = self.get_underlying_exprs(&stat_nodes[2]);
                        self.compile_assignments(names, exprs);
                    }
                    _ => {}
                }
            }
        } else if len == 1 {
            // stat_nodes = <functioncall>
            match stat_nodes[0] {
                Nonterm {
                    ridx: RIdx(ridx),
                    ref nodes,
                } if ridx == lua5_3_y::R_FUNCTIONCALL => {
                    // nodes = [<prefixexp>, <args>]
                    if nodes.len() == 2 {
                        self.compile_call(&nodes[0], &nodes[1])
                    } else {
                        self.compile_method(&nodes[0], &nodes[2], &nodes[3])
                    }
                }
                _ => {}
            }
        } else {
            // stat_nodes = [<IF>, <exp>, <THEN>, <block>, <elselistopt>, <elseopt>, <END>]
            if is_term(&stat_nodes[0], lua5_3_l::T_IF) {
                self.compile_if(
                    &stat_nodes[1],
                    &stat_nodes[3],
                    &stat_nodes[4],
                    &stat_nodes[5],
                );
            } else if is_term(&stat_nodes[0], lua5_3_l::T_WHILE) {
                self.compile_while(&stat_nodes[1], &stat_nodes[3]);
            } else if is_term(&stat_nodes[0], lua5_3_l::T_FOR) && stat_nodes.len() == 9 {
                // stat_nodes = [<FOR>, <NAME>, <EQ>, <exp>, <COMMA>,
                //               <explist>, <DO>, <block>, <END>]
                let name = self.compile_var_or_name(&stat_nodes[1]);
                self.compile_for_count(name, &stat_nodes[3], &stat_nodes[5], &stat_nodes[7]);
            } else if is_term(&stat_nodes[0], lua5_3_l::T_LOCAL) {
                // stat_nodes = [<LOCAL>, <FUNCTION>, <NAME>, <funcbody>]
                let name = self.compile_var_or_name(&stat_nodes[2]);
                let new_reg = self.curr_func().get_new_reg();
                self.instrs()
                    .push(Instr::TwoArg(MOV, Arg::Reg(new_reg), Arg::Nil));
                self.set_reg_name(new_reg, name.get_str(), true);
                self.compile_assignment(name, &stat_nodes[3], AssignmentType::Regular);
            }
        }
    }

    /// Compiles a local multi-assignemnt.
    /// * `names` - the variable names
    /// * `exprs` - the expressions that are assigned
    fn compile_local_assignments(&mut self, names: Vec<&'a str>, exprs: Vec<&'a Node<u8>>) {
        // example: local a, b, c, d = 1, 2
        // compile local a = 1, local b = 2
        for i in 0..exprs.len() {
            // left hand-side = <namelist> and right hand-side = <explist>
            self.compile_assignment(VarType::Name(names[i]), exprs[i], AssignmentType::LocalDecl);
        }
        // for all the remaining names (c, d), create a new empty register, because the
        // user might access the variable later
        if names.len() > exprs.len() {
            let mut regs = vec![];
            for i in exprs.len()..names.len() {
                let new_reg = self.curr_func().get_new_reg();
                self.set_reg_name(new_reg, names[i], true);
                regs.push(new_reg);
            }
            // check if the last expression is a vararg, so that we can emit the correct
            // instruction
            let mut assign_nils = false;
            if let Some(expr) = exprs.last() {
                if self.is_unpackable(expr) {
                    self.unpack(&regs, expr);
                } else {
                    assign_nils = true;
                }
            } else {
                assign_nils = true;
            }
            if assign_nils {
                for reg in regs {
                    self.instrs()
                        .push(Instr::TwoArg(MOV, Arg::Reg(reg), Arg::Nil));
                }
            }
        } else if names.len() < exprs.len() {
            // make sure we also compile every expression on the right side
            // local a = 1, 2, f(); we have to also compile 2, and f()
            for i in names.len()..exprs.len() {
                self.compile_expr(exprs[i]);
            }
        }
    }

    /// Compiles a multi-assignemnt (a combination of local and global assignments).
    /// * `names` - the variable names
    /// * `exprs` - the expressions that are assigned
    fn compile_assignments(&mut self, names: Vec<&'a Node<u8>>, exprs: Vec<&'a Node<u8>>) {
        // we want to emit _ENV[<name>] = <reg> only after we assign all expressions into
        // registers. This is because of how vararg expects registers to be ordered.
        // For instance `a, b = ...`, will generate `VarArg 3, 2, 0` meaning that the vm
        // will copy two variable arguments into registers 3 and 4. We have to make sure
        // that a, and b point to consecutive registers, but a global assignment will
        // generate additional instructions, which we try to postpone
        let mut postponed_instrs: Vec<(VarType<'a>, usize)> = vec![];
        // example: x, y, z, w = 1, 2
        // compile x = 1, y = 2
        for (name, expr) in names.iter().zip(exprs.iter()) {
            let var = self.compile_var_or_name(name);
            let res = self.compile_assignment(var, expr, AssignmentType::Postponed);
            match res {
                ResultType::Global(reg) => {
                    postponed_instrs.push((var, reg));
                }
                ResultType::Dict(reg) => {
                    postponed_instrs.push((var, reg));
                }
                _ => {}
            }
        }
        // for all the remaining names (z, w), create a new empty register, and update
        // _ENV if the variable has not been declared as local in some outer scope
        // names.len() == exprs.len() is intentionally left out because that case is
        // handled by the loop above
        if names.len() > exprs.len() {
            let mut regs = vec![];
            for i in exprs.len()..names.len() {
                let var = self.compile_var_or_name(names[i]);
                let reg = self.curr_func().get_new_reg();
                match var {
                    VarType::Name(name) => {
                        if !self.is_local(name) {
                            postponed_instrs.push((var, reg));
                        } else {
                            self.set_reg_name(reg, name, false);
                        }
                    }
                    VarType::Dict(..) => {
                        postponed_instrs.push((var, reg));
                    }
                }
                regs.push(reg);
            }
            let mut assign_nils = false;
            if let Some(expr) = exprs.last() {
                if self.is_unpackable(expr) {
                    self.unpack(&regs, expr);
                } else {
                    assign_nils = true;
                }
            } else {
                assign_nils = true;
            }
            if assign_nils {
                for reg in regs {
                    self.instrs()
                        .push(Instr::TwoArg(MOV, Arg::Reg(reg), Arg::Nil));
                }
            }
        } else if names.len() < exprs.len() {
            // make sure we also compile every expression on the right side
            // a = 1, 2, f(); we have to also compile 2, and f()
            for i in names.len()..exprs.len() {
                self.compile_expr(exprs[i]);
            }
        }
        // generate the missing instructions that were postponed
        for (var, reg) in postponed_instrs {
            match var {
                VarType::Name(name) => {
                    self.set_upval(name, reg);
                }
                VarType::Dict(from, attr) => {
                    self.instrs().push(Instr::ThreeArg(
                        SetAttr,
                        Arg::Reg(from),
                        Arg::Reg(attr),
                        Arg::Reg(reg),
                    ));
                }
            }
        }
    }

    fn is_unpackable(&self, expr: &Node<u8>) -> bool {
        self.is_vararg(expr) || self.is_functioncall(expr)
    }

    fn unpack(&mut self, regs: &Vec<usize>, expr: &Node<u8>) {
        // local a, b, c = f(2)
        // we are unpacking f(2) into a, b, and c, but we have already pushed a
        // MOVR in compile_assignemnts, thus we have to unpack the rest of the
        // values into b, and c
        let opcode = if self.is_vararg(expr) { VarArg } else { MOVR };
        for (i, reg) in regs.iter().enumerate() {
            self.instrs()
                .push(Instr::TwoArg(opcode, Arg::Reg(*reg), Arg::Some(i + 1)));
        }
    }

    fn find_name(&mut self, name: &'a str) -> usize {
        match self.get_reg(name) {
            Some(reg) => reg,
            // check to see if any parent functions or blocks contain this variable
            None => {
                let reg = self.curr_func().get_new_reg();
                if let Some(upval_idx) = self.get_upval(name) {
                    self.instrs().push(Instr::TwoArg(
                        GetUpVal,
                        Arg::Reg(reg),
                        Arg::Some(upval_idx + 1),
                    ))
                } else {
                    self.instrs().push(Instr::ThreeArg(
                        GetUpAttr,
                        Arg::Reg(reg),
                        Arg::Some(0),
                        Arg::Str(name.to_string()),
                    ));
                }
                reg
            }
        }
    }

    fn compile_prefix_exp(&mut self, node: &'a Node<u8>) -> usize {
        match *node {
            Nonterm {
                ridx: RIdx(ridx),
                ref nodes,
            } if ridx == lua5_3_y::R_PREFIXEXP => {
                if nodes.len() > 1 {
                    self.compile_expr(&nodes[1])
                } else if is_nonterm(&nodes[0], lua5_3_y::R_FUNCTIONCALL) {
                    self.compile_expr(&nodes[0])
                } else {
                    let var = self.compile_var_or_name(&nodes[0]);
                    match var {
                        VarType::Name(name) => self.find_name(name),
                        VarType::Dict(from, attr) => {
                            let reg = self.curr_func().get_new_reg();
                            self.instrs().push(Instr::ThreeArg(
                                GetAttr,
                                Arg::Reg(reg),
                                Arg::Reg(from),
                                Arg::Reg(attr),
                            ));
                            reg
                        }
                    }
                }
            }
            _ => panic!("Expected <prefixexp>, got {:?}", node),
        }
    }

    fn compile_var_list(&self, node: &'a Node<u8>) -> Vec<&'a Node<u8>> {
        let mut vars = vec![];
        match *node {
            Nonterm {
                ridx: RIdx(ridx),
                ref nodes,
            } if ridx == lua5_3_y::R_VARLIST => {
                if nodes.len() > 1 {
                    vars.extend(self.compile_var_list(&nodes[0]));
                    vars.push(&nodes[2]);
                } else {
                    vars.push(&nodes[0]);
                }
            }
            _ => panic!("Expected <varlist> got {:?}", node),
        }
        vars
    }

    /// Compile an assignment by compiling <right> and then storing the result in <left>.
    /// * `left` - The name of the variable in which the result is stored
    /// * `right` - The expression that is evaluated
    /// * `action` - How the compiler should behave, see @AssignmentType for more info.
    /// Returns whether the assignment was local or global.
    fn compile_assignment(
        &mut self,
        var: VarType<'a>,
        right: &'a Node<u8>,
        action: AssignmentType,
    ) -> ResultType {
        match var {
            VarType::Name(name) => {
                let old_len = self.curr_block().instrs().len();
                let mut value = self.compile_expr(right);
                // the register map only keeps track of local variables
                // if we are compiling: `x = 3`, then we also have to check if x is in `reg_map`
                // if it is, then it is a local assignment (because `reg_map` only stores
                // mappings of local variable to registers), if it isn't then we load it from
                // the global mapping
                if action == AssignmentType::LocalDecl || self.get_reg(name).is_some() {
                    // No new instructions were added, which means that <right> has already been
                    // computed and stored in some register. Because we are compiling an
                    // assignment, we will create a copy of this result and store it in <left>.
                    // See test `load_string_multiple_times`.
                    if self.curr_block().instrs().len() == old_len {
                        let new_reg = self.curr_func().get_new_reg();
                        self.instrs()
                            .push(Instr::TwoArg(MOV, Arg::Reg(new_reg), Arg::Reg(value)));
                        value = new_reg;
                    }
                    // if a variable is assigned a value multiple times, we have to make sure
                    // that the map knows the new register which holds the new value
                    self.set_reg_name(value, name, action == AssignmentType::LocalDecl);
                    ResultType::Local(value)
                } else {
                    if action != AssignmentType::Postponed {
                        self.set_upval(name, value);
                    }
                    ResultType::Global(value)
                }
            }
            VarType::Dict(from, attr) => {
                let reg = self.compile_expr(right);
                if action != AssignmentType::Postponed {
                    self.instrs().push(Instr::ThreeArg(
                        SetAttr,
                        Arg::Reg(from),
                        Arg::Reg(attr),
                        Arg::Reg(reg),
                    ));
                }
                ResultType::Dict(reg)
            }
        }
    }

    fn compile_var_or_name(&mut self, node: &'a Node<u8>) -> VarType<'a> {
        match *node {
            Nonterm {
                ridx: RIdx(ridx),
                ref nodes,
            } if ridx == lua5_3_y::R_VAR => {
                if nodes.len() == 1 {
                    VarType::Name(self.get_str(&nodes[0]))
                } else if nodes.len() == 4 {
                    // nodes = [<prefixexp, <LSQUARE>, <exp>, <RSQUARE>]
                    let prefixexp = self.compile_prefix_exp(&nodes[0]);
                    let expr = self.compile_expr(&nodes[2]);
                    VarType::Dict(prefixexp, expr)
                } else {
                    let prefixexp = self.compile_prefix_exp(&nodes[0]);
                    let string = self.get_str(&nodes[2]);
                    let reg = self.curr_func().get_new_reg();
                    self.instrs().push(Instr::TwoArg(
                        MOV,
                        Arg::Reg(reg),
                        Arg::Str(string.to_string()),
                    ));
                    VarType::Dict(prefixexp, reg)
                }
            }
            _ => {
                let name = find_term(node, lua5_3_l::T_NAME).unwrap();
                VarType::Name(self.get_str(name))
            }
        }
    }

    /// Compile the expression rooted at <node>. Any instructions that are created are
    /// simply added to the bytecode that is being generated.
    fn compile_expr(&mut self, node: &'a Node<u8>) -> usize {
        match *node {
            Nonterm {
                ridx: RIdx(ridx),
                ref nodes,
            } if ridx == lua5_3_y::R_FUNCBODY => self.compile_funcbody(nodes),
            Nonterm {
                ridx: RIdx(ridx),
                ref nodes,
            } if ridx == lua5_3_y::R_FUNCTIONCALL => {
                if nodes.len() == 2 {
                    self.compile_call(&nodes[0], &nodes[1]);
                } else {
                    self.compile_method(&nodes[0], &nodes[2], &nodes[3]);
                }
                let reg = self.curr_func().get_new_reg();
                self.instrs()
                    .push(Instr::TwoArg(MOVR, Arg::Reg(reg), Arg::Some(0)));
                reg
            }
            Nonterm {
                ridx: RIdx(ridx),
                ref nodes,
            } if ridx == lua5_3_y::R_FUNCTIONDEF => {
                // nodes = [<FUNCTION>, <functionbody>]
                self.compile_funcbody(get_nodes(&nodes[1], lua5_3_y::R_FUNCBODY))
            }
            Nonterm {
                ridx: RIdx(ridx),
                ref nodes,
            } if ridx == lua5_3_y::R_EXP => {
                if nodes.len() > 1 {
                    self.compile_or_short_circuit(nodes)
                } else {
                    self.compile_expr(&nodes[0])
                }
            }
            Nonterm {
                ridx: RIdx(ridx),
                ref nodes,
            } if ridx == lua5_3_y::R_EXP1 => {
                if nodes.len() > 1 {
                    self.compile_and_short_circuit(nodes)
                } else {
                    self.compile_expr(&nodes[0])
                }
            }
            Nonterm {
                ridx: RIdx(ridx),
                nodes: _,
            } if ridx == lua5_3_y::R_PREFIXEXP => self.compile_prefix_exp(node),
            Nonterm {
                ridx: RIdx(ridx),
                nodes: _,
            } if ridx == lua5_3_y::R_TABLECONSTRUCTOR => self.compile_table_cons(node),
            Nonterm {
                ridx: RIdx(_ridx),
                ref nodes,
            } => {
                if nodes.len() == 1 {
                    self.compile_expr(&nodes[0])
                } else if nodes.len() == 2 {
                    let right = self.compile_expr(&nodes[1]);
                    let new_var = self.curr_func().get_new_reg();
                    let instr = self.get_unary_instr(&nodes[0], new_var, right);
                    self.instrs().push(instr);
                    new_var
                } else {
                    debug_assert!(nodes.len() == 3);
                    let left = self.compile_expr(&nodes[0]);
                    let right = self.compile_expr(&nodes[2]);
                    let new_var = self.curr_func().get_new_reg();
                    let instr = self.get_instr(&nodes[1], new_var, left, right);
                    self.instrs().push(instr);
                    new_var
                }
            }
            Term { lexeme } => {
                let value = self
                    .pt
                    .get_string(lexeme.start(), lexeme.end().unwrap_or(lexeme.start()));
                match lexeme.tok_id() {
                    lua5_3_l::T_NUMERAL => {
                        let new_reg = self.curr_func().get_new_reg();
                        let arg = if value.contains(".") {
                            Arg::Float(value.parse().unwrap())
                        } else {
                            Arg::Int(value.parse().unwrap())
                        };
                        self.instrs()
                            .push(Instr::TwoArg(MOV, Arg::Reg(new_reg), arg));
                        new_reg
                    }
                    lua5_3_l::T_SHORT_STR => {
                        let new_reg = self.curr_func().get_new_reg();
                        self.instrs().push(Instr::TwoArg(
                            MOV,
                            Arg::Reg(new_reg),
                            Arg::Str(value[1..(value.len() - 1)].to_string()),
                        ));
                        new_reg
                    }
                    lua5_3_l::T_NIL => {
                        let new_reg = self.curr_func().get_new_reg();
                        self.instrs()
                            .push(Instr::TwoArg(MOV, Arg::Reg(new_reg), Arg::Nil));
                        new_reg
                    }
                    lua5_3_l::T_NAME => self.find_name(value),
                    lua5_3_l::T_DOTDOTDOT => {
                        if self.curr_func().is_vararg() {
                            let reg = self.curr_func().get_new_reg();
                            self.instrs()
                                .push(Instr::TwoArg(VarArg, Arg::Reg(reg), Arg::Some(0)));
                            reg
                        } else {
                            panic!("Cannot use '...' outside of a vararg function.")
                        }
                    }
                    lua5_3_l::T_FALSE => {
                        let new_reg = self.curr_func().get_new_reg();
                        self.instrs()
                            .push(Instr::TwoArg(MOV, Arg::Reg(new_reg), Arg::Bool(false)));
                        new_reg
                    }
                    lua5_3_l::T_TRUE => {
                        let new_reg = self.curr_func().get_new_reg();
                        self.instrs()
                            .push(Instr::TwoArg(MOV, Arg::Reg(new_reg), Arg::Bool(true)));
                        new_reg
                    }
                    _ => panic!(
                        "Cannot compile terminals that are not variable names, numbers or strings."
                    ),
                }
            }
        }
    }

    fn compile_funcbody(&mut self, nodes: &'a Vec<Node<u8>>) -> usize {
        let old_curr_func = self.curr_func;
        let old_curr_block = self.curr_block;
        // create a new `CompiledFunc` for this function
        let new_func_id = self.functions.len();
        let param_nodes = get_nodes(&nodes[1], lua5_3_y::R_PARLIST);
        let param_count = param_nodes.len();
        let is_vararg =
            param_count > 0 && is_term(param_nodes.last().unwrap(), lua5_3_l::T_DOTDOTDOT);
        let mut new_func = CompiledFunc::new(0, is_vararg);
        new_func.set_parent_func(self.curr_func);
        new_func.set_parent_block(self.curr_block);
        self.functions.push(new_func);
        self.curr_func = new_func_id;
        let new_basic_block = self.curr_func().create_block();
        self.curr_block = new_basic_block;
        // make the first N registers point to the first N parameters
        self.compile_param_list(&nodes[1]);
        self.compile_block_in_basic_block(&nodes[3], new_basic_block);
        // restore the old state so that we can create a closure instruction
        // in the outer function
        self.curr_func = old_curr_func;
        self.curr_block = old_curr_block;
        let reg = self.curr_func().get_new_reg();
        self.instrs().push(Instr::TwoArg(
            CLOSURE,
            Arg::Reg(reg),
            Arg::Func(new_func_id),
        ));
        // declare, update and move upvalues based on what the child function depends on
        let (provides, curr_func_upvals) = {
            let mut curr_func_upvals_count = self.curr_func().upvals().len();
            let new_func = &self.functions[new_func_id];
            let mut curr_func_upvals = vec![];
            let mut provides = vec![];
            // move the upvalues into the new function
            for (name, location) in new_func.upvals() {
                // does the child function have a dependency on any of the current local
                // variables?
                if let Some(local_reg) = self.get_reg(name) {
                    provides.push((new_func_id, *location + 1, ProviderType::Reg(local_reg)));
                } else {
                    // the child has a dependency on either:
                    // i)  an already created upvalue of the current function
                    // ii) a local in some outer function, which has not been
                    // declared as an upvalue
                    let upval_idx = if let Some(upval_idx) =
                        self.functions[self.curr_func].upvals().get(name)
                    {
                        *upval_idx + 1
                    } else {
                        curr_func_upvals.push(*name);
                        curr_func_upvals_count += 1;
                        curr_func_upvals_count
                    };
                    provides.push((new_func_id, *location + 1, ProviderType::Upval(upval_idx)));
                }
            }
            (provides, curr_func_upvals)
        };
        for (func_idx, upval_idx, pt) in provides {
            self.curr_func().push_provider(func_idx, upval_idx, pt);
        }
        for upval in curr_func_upvals {
            self.curr_func().push_upval(upval);
        }
        reg
    }

    fn compile_or_short_circuit(&mut self, nodes: &'a Vec<Node<u8>>) -> usize {
        let left = self.compile_expr(&nodes[0]);
        let parent = self.curr_block;
        let false_branch = self.create_child_block();
        let right = self.compile_expr(&nodes[2]);
        self.curr_block = parent;
        let merge_branch = self.create_child_block();
        let merge_reg = self.curr_func().get_new_reg();
        self.instrs().push(Instr::NArg(
            Phi,
            vec![Arg::Reg(merge_reg), Arg::Reg(left), Arg::Reg(right)],
        ));
        self.get_block(parent).mut_instrs().push(Instr::ThreeArg(
            JmpEQ,
            Arg::Reg(left),
            Arg::Some(merge_branch),
            Arg::Some(false_branch),
        ));
        merge_reg
    }

    fn compile_and_short_circuit(&mut self, nodes: &'a Vec<Node<u8>>) -> usize {
        let left = self.compile_expr(&nodes[0]);
        let parent = self.curr_block;
        let false_branch = self.create_child_block();
        let right = self.compile_expr(&nodes[2]);
        self.curr_block = parent;
        let merge_branch = self.create_child_block();
        let merge_reg = self.curr_func().get_new_reg();
        self.instrs().push(Instr::NArg(
            Phi,
            vec![Arg::Reg(merge_reg), Arg::Reg(left), Arg::Reg(right)],
        ));
        self.get_block(parent).mut_instrs().push(Instr::ThreeArg(
            JmpNE,
            Arg::Reg(left),
            Arg::Some(false_branch),
            Arg::Some(merge_branch),
        ));
        merge_reg
    }

    /// Compile an and short circuit in which the lhs is in <left> and the rhs block
    /// contains the instructions <right_operand_instrs>.
    fn compile_and_short_circuit2(
        &mut self,
        left: usize,
        right_operand_instrs: Vec<Instr>,
    ) -> usize {
        let parent = self.curr_block;
        let false_branch = self.create_child_block();
        self.curr_block().mut_instrs().extend(right_operand_instrs);
        let right =
            if let Instr::ThreeArg(_, arg1, _, _) = self.curr_block().instrs().last().unwrap() {
                arg1.get_reg()
            } else {
                panic!("Expected a three argument instruction!")
            };
        self.curr_block = parent;
        let merge_branch = self.create_child_block();
        let merge_reg = self.curr_func().get_new_reg();
        self.instrs().push(Instr::NArg(
            Phi,
            vec![Arg::Reg(merge_reg), Arg::Reg(left), Arg::Reg(right)],
        ));
        self.get_block(parent).mut_instrs().push(Instr::ThreeArg(
            JmpNE,
            Arg::Reg(left),
            Arg::Some(false_branch),
            Arg::Some(merge_branch),
        ));
        merge_reg
    }

    fn compile_or_short_circuit2(&mut self, left: usize, right: usize, parent: usize) -> usize {
        let false_branch = self.curr_block;
        self.curr_block = parent;
        let merge_branch = self.create_child_block();
        let merge_reg = self.curr_func().get_new_reg();
        self.instrs().push(Instr::NArg(
            Phi,
            vec![Arg::Reg(merge_reg), Arg::Reg(left), Arg::Reg(right)],
        ));
        self.get_block(parent).mut_instrs().push(Instr::ThreeArg(
            JmpEQ,
            Arg::Reg(left),
            Arg::Some(false_branch),
            Arg::Some(merge_branch),
        ));
        merge_reg
    }

    /// Compile an <explist> or <explistopt> and return the roots of the expressions.
    fn get_underlying_exprs(&mut self, exprs: &'a Node<u8>) -> Vec<&'a Node<u8>> {
        match *exprs {
            Nonterm {
                ridx: RIdx(ridx),
                ref nodes,
            } if ridx == lua5_3_y::R_EXPLIST => {
                let mut exprs = vec![];
                // nodes = <exp>
                if nodes.len() == 1 {
                    exprs.push(&nodes[0]);
                } else {
                    // nodes = [<explist>, <COMMA>,  <exp>]
                    exprs.extend(self.get_underlying_exprs(&nodes[0]));
                    exprs.push(&nodes[2]);
                }
                exprs
            }
            Nonterm {
                ridx: RIdx(ridx),
                ref nodes,
            } if ridx == lua5_3_y::R_EXPLISTOPT => {
                // nodes = <explist>
                if nodes.len() > 0 {
                    self.get_underlying_exprs(&nodes[0])
                } else {
                    vec![]
                }
            }
            _ => panic!("Root node was not an <explist> or <explistopt>"),
        }
    }

    fn get_str(&self, name: &'a Node<u8>) -> &'a str {
        match *name {
            Term { lexeme } if lexeme.tok_id() == lua5_3_l::T_NAME => self
                .pt
                .get_string(lexeme.start(), lexeme.end().unwrap_or(lexeme.start())),
            _ => panic!("Expected term <NAME>, got {:?}", name),
        }
    }

    /// Compile a <namelist> or a <varlist> into a vector of names.
    fn compile_names(&mut self, names: &'a Node<u8>) -> Vec<&'a str> {
        match *names {
            Nonterm {
                ridx: RIdx(ridx),
                ref nodes,
            } if ridx == lua5_3_y::R_NAMELIST || ridx == lua5_3_y::R_VARLIST => {
                let mut names = vec![];
                // nodes = <NAME>
                if nodes.len() == 1 {
                    names.push(self.get_str(&nodes[0]));
                } else {
                    // nodes = [<namelist>, <COMMA>, <NAME>]
                    names.extend(self.compile_names(&nodes[0]));
                    names.push(self.get_str(&nodes[2]));
                }
                names
            }
            _ => panic!("Root node is not a <namelist> or a <varlist>"),
        }
    }

    /// Compile a <parlist> node, and assign each name a register in the current
    /// register map.
    /// The first parameter of a function is assigned to register 0, and so on.
    /// For now the vararg parameter is ignored.
    fn compile_param_list(&mut self, params: &'a Node<u8>) {
        match *params {
            Nonterm {
                ridx: RIdx(ridx),
                ref nodes,
            } if ridx == lua5_3_y::R_PARLIST => {
                let len = nodes.len();
                if len == 0 {
                    return;
                }
                let mut names = vec![];
                // nodes = [<parlist>, <COMMA>, <...>]
                if len == 3 {
                    names.extend(self.compile_names(&nodes[0]));
                } else {
                    // either nodes = <...> or <parlist>
                    match nodes[0] {
                        Nonterm { ridx: _, nodes: _ } => {
                            names.extend(self.compile_names(&nodes[0]))
                        }
                        _ => {}
                    }
                }
                self.functions[self.curr_func].set_param_count(names.len());
                for name in names {
                    let reg = self.curr_func().get_new_reg();
                    self.set_reg_name(reg, name, true);
                }
            }
            _ => panic!("Root node was not a <parlist>"),
        }
    }

    /// Compile a <functioncall>.
    fn compile_call(&mut self, func: &'a Node<u8>, params: &'a Node<u8>) {
        let func_reg = self.compile_prefix_exp(func);
        let params = match *params {
            Nonterm {
                ridx: RIdx(ridx),
                ref nodes,
            } if ridx == lua5_3_y::R_ARGS => &nodes[1],
            _ => panic!("Missing node <args> from <functioncall>"),
        };
        self.instrs()
            .push(Instr::OneArg(SetTop, Arg::Reg(func_reg)));
        let exprs = self.get_underlying_exprs(params);
        if exprs.len() > 0 {
            // push the arguments to the function
            for i in 0..(exprs.len() - 1) {
                let reg = self.compile_expr(exprs[i]);
                self.instrs().push(Instr::OneArg(PUSH, Arg::Reg(reg)));
            }
            self.unpack_to_stack(&exprs.last().unwrap(), false);
        }
        self.instrs().push(Instr::OneArg(CALL, Arg::Reg(func_reg)));
    }

    /// Compile a method.
    fn compile_method(&mut self, func: &'a Node<u8>, name: &'a Node<u8>, params: &'a Node<u8>) {
        let prefix_reg = self.compile_prefix_exp(func);
        let name = self.get_str(name).to_string();
        let name_reg = self.curr_func().get_new_reg();
        self.instrs()
            .push(Instr::TwoArg(MOV, Arg::Reg(name_reg), Arg::Str(name)));
        let func_reg = self.curr_func().get_new_reg();
        self.instrs().push(Instr::ThreeArg(
            GetAttr,
            Arg::Reg(func_reg),
            Arg::Reg(prefix_reg),
            Arg::Reg(name_reg),
        ));
        let params = match *params {
            Nonterm {
                ridx: RIdx(ridx),
                ref nodes,
            } if ridx == lua5_3_y::R_ARGS => &nodes[1],
            _ => panic!("Missing node <args> from <functioncall>"),
        };
        self.instrs()
            .push(Instr::OneArg(SetTop, Arg::Reg(func_reg)));
        self.instrs()
            .push(Instr::OneArg(PUSH, Arg::Reg(prefix_reg)));
        let exprs = self.get_underlying_exprs(params);
        if exprs.len() > 0 {
            // push the arguments to the function
            for i in 0..(exprs.len() - 1) {
                let reg = self.compile_expr(exprs[i]);
                self.instrs().push(Instr::OneArg(PUSH, Arg::Reg(reg)));
            }
            self.unpack_to_stack(&exprs.last().unwrap(), false);
        }
        self.instrs().push(Instr::OneArg(CALL, Arg::Reg(func_reg)));
    }

    /// Checks if exp is '...'
    fn is_vararg(&self, exp: &Node<u8>) -> bool {
        match exp {
            Nonterm { ridx: _, ref nodes } => nodes.len() == 1 && self.is_vararg(&nodes[0]),
            Term { lexeme } => lexeme.tok_id() == lua5_3_l::T_DOTDOTDOT,
        }
    }

    fn is_functioncall(&self, expr: &Node<u8>) -> bool {
        if let Nonterm {
            ridx: RIdx(ridx),
            ref nodes,
        } = expr
        {
            if *ridx == lua5_3_y::R_FUNCTIONCALL {
                return true;
            } else {
                return nodes.len() == 1 && self.is_functioncall(&nodes[0]);
            }
        }
        false
    }

    /// Get the appropriate instruction for a given Node::Term.
    fn get_unary_instr(&self, node: &'a Node<u8>, reg: usize, rreg: usize) -> Instr {
        if let Term { lexeme } = node {
            let opcode = match lexeme.tok_id() {
                lua5_3_l::T_MINUS => UMN,
                _ => unimplemented!("Unary instruction {:#?}", node),
            };
            Instr::TwoArg(opcode, Arg::Reg(reg), Arg::Reg(rreg))
        } else {
            panic!("Expected a Node::Term!");
        }
    }

    /// Get the appropriate instruction for a given Node::Term.
    fn get_instr(&self, node: &'a Node<u8>, reg: usize, lreg: usize, rreg: usize) -> Instr {
        if let Term { lexeme } = node {
            let opcode = match lexeme.tok_id() {
                lua5_3_l::T_PLUS => ADD,
                lua5_3_l::T_MINUS => SUB,
                lua5_3_l::T_STAR => MUL,
                lua5_3_l::T_FSLASH => DIV,
                lua5_3_l::T_MOD => MOD,
                lua5_3_l::T_FSFS => FDIV,
                lua5_3_l::T_CARET => EXP,
                lua5_3_l::T_EQEQ => EQ,
                lua5_3_l::T_LT => LT,
                lua5_3_l::T_GT => GT,
                lua5_3_l::T_LE => LE,
                lua5_3_l::T_GE => GE,
                lua5_3_l::T_NOTEQ => NE,
                _ => unimplemented!("Instruction {:#?}", node),
            };
            Instr::ThreeArg(opcode, Arg::Reg(reg), Arg::Reg(lreg), Arg::Reg(rreg))
        } else {
            panic!("Expected a Node::Term!");
        }
    }

    /// Compile an if-statement.
    fn compile_if(
        &mut self,
        expr: &'a Node<u8>,
        block: &'a Node<u8>,
        elselistopt: &'a Node<u8>,
        elseopt: &'a Node<u8>,
    ) {
        let before_if_index = self.curr_block;
        let elselist = self.get_elselist(elselistopt);
        let mut branches: Vec<usize> = vec![(expr, block)]
            .iter()
            .chain(elselist.iter())
            .map(|(e, b)| {
                let before = self.curr_block;
                // emit the phis of the block before compiling the condition to
                // ensure the correct register contains the result
                self.generate_phis_for_bb(before);
                // compile if condition
                let expr_res = self.compile_expr(e);
                let before = self.curr_block;
                // compile true branch as a child of the current block
                let true_block = self.compile_block(b);
                self.curr_block = before;
                let last_true_block = self.curr_func().blocks().len() - 1;
                // create a new block
                let parent = self.curr_block;
                let elif_block = self.curr_func().create_block_with_parents(vec![parent]);
                self.instrs().push(Instr::ThreeArg(
                    JmpNE,
                    Arg::Reg(expr_res),
                    Arg::Some(true_block),
                    Arg::Some(elif_block),
                ));
                self.curr_block = elif_block;
                self.curr_block().push_dominator(before);
                last_true_block
            })
            .collect();
        let else_nodes = get_nodes(elseopt, lua5_3_y::R_ELSEOPT);
        let process_else = else_nodes.len() > 0;
        if process_else {
            let else_block = self.curr_block;
            self.compile_block_in_basic_block(&else_nodes[1], else_block);
            branches.push(else_block);
            self.curr_block = self.curr_func().create_block();
            self.curr_block().push_dominator(before_if_index);
        }
        for &branch in &branches {
            let curr = self.curr_block;
            self.generate_phis_for_bb(branch);
            self.get_block(branch)
                .mut_instrs()
                .push(Instr::OneArg(Jmp, Arg::Some(curr)));
            self.get_block(curr).push_parent(branch);
        }
        if !process_else {
            branches.push(self.curr_block().parents()[0]);
        }
        self.generate_phis(before_if_index);
    }

    fn generate_phis(&mut self, extra_lookup_block: usize) {
        let mut phis: HashMap<&'a str, BTreeSet<usize>> = HashMap::new();
        {
            let curr_func = &self.functions[self.curr_func];
            let curr_block_index = self.curr_block;
            let curr_block = curr_func.get_block(curr_block_index);
            for &p in curr_block
                .parents()
                .iter()
                .chain(vec![curr_block_index, extra_lookup_block].iter())
            {
                for (name, ref reg) in curr_func.get_block(p).non_locals() {
                    if self.is_local(name) {
                        phis.entry(name)
                            .and_modify(|args| {
                                args.insert(*reg.last().unwrap());
                            })
                            .or_insert_with(|| {
                                let mut new_set = BTreeSet::new();
                                new_set.insert(*reg.last().unwrap());
                                new_set
                            });
                    }
                }
            }
        }
        for (name, mut args) in phis {
            args.insert(
                self.get_reg_from_block(name, self.curr_func, self.curr_block)
                    .expect("Non-local found in branch, but not in parent blocks!"),
            );
            let mut args: Vec<Arg> = args.iter().map(|v| Arg::Reg(*v)).collect();
            let local_decl = self.is_locally_declared_in_doms(name);
            self.set_reg_name(args.first().unwrap().get_reg(), name, local_decl);
            self.instrs().push(Instr::NArg(Phi, args));
        }
    }

    /// Compile an <elselist> or <elselistopt>.
    fn get_elselist(&mut self, elselistopt: &'a Node<u8>) -> Vec<(&'a Node<u8>, &'a Node<u8>)> {
        let mut blocks = vec![];
        match *elselistopt {
            Nonterm {
                ridx: RIdx(ridx),
                ref nodes,
            } if ridx == lua5_3_y::R_ELSELISTOPT => {
                if nodes.len() > 0 {
                    blocks.extend(self.get_elselist(&nodes[0]));
                }
            }
            Nonterm {
                ridx: RIdx(ridx),
                ref nodes,
            } if ridx == lua5_3_y::R_ELSELIST => {
                // nodes = [<elselist>, <ELSEIF>, <exp>, <THEN>, <block>]
                if nodes.len() > 4 {
                    blocks.extend(self.get_elselist(&nodes[0]));
                    blocks.push((&nodes[2], &nodes[4]));
                } else {
                    // nodes = [<ELSEIF>, <exp>, <THEN>, <block>]
                    blocks.push((&nodes[1], &nodes[3]));
                }
            }
            _ => panic!(
                "Expected an <elselist> or <elselistopt>, but got {:#?}",
                elselistopt
            ),
        }
        blocks
    }

    fn compile_while(&mut self, expr: &'a Node<u8>, block: &'a Node<u8>) {
        // the block before the while loop
        let parent = self.curr_block;
        self.generate_phis_for_bb(parent);
        // compile expr in a new block, and create a branching instruction to it
        self.curr_block()
            .mut_instrs()
            .push(Instr::OneArg(Jmp, Arg::Some(parent + 1)));
        // the block in which the condition is compiled
        let cond_start = self.curr_func().create_block_with_parents(vec![parent]);
        self.get_block(cond_start).push_dominator(parent);
        self.curr_block = cond_start;
        let expr_reg = self.compile_expr(expr);
        // the expression might generate multiple basic blocks, thus we have to make sure
        // to save the last block of the whole expression
        let cond_end = self.curr_func().blocks().len() - 1;
        self.curr_block = cond_end;
        // compile the while loop block
        self.compile_while_body(
            parent,
            cond_start,
            cond_end,
            expr_reg,
            block,
            vec![],
            vec![],
        );
    }

    fn compile_while_body(
        &mut self,
        parent_of_while: usize,
        while_cond_start: usize,
        while_cond_end: usize,
        expr_reg: usize,
        block: &'a Node<u8>,
        additional_instrs: Vec<Instr>,
        reg_map_updates: Vec<(usize, &'a str)>,
    ) {
        // compile the body of the while loop
        let while_block = self.compile_block(block);
        // again, the block might create multiple blocks; save the last one
        let last_block = self.curr_func().blocks().len() - 1;
        // generate a jump from the condition block to `while_block` if the condition
        // is true, or to `last_block`, which is the block right after the while loop
        self.get_block(while_cond_end)
            .mut_instrs()
            .push(Instr::ThreeArg(
                JmpNE,
                Arg::Reg(expr_reg),
                Arg::Some(while_block),
                Arg::Some(last_block + 1),
            ));
        self.curr_block = last_block;
        // add any additional instructions
        self.instrs().extend(additional_instrs);
        for (reg, name) in reg_map_updates {
            self.set_reg_name(reg, name, false);
        }
        self.generate_phis_for_bb(last_block);
        // self.generate_phis(parent_of_while);
        // jump back to the start of the expression evaluation
        self.curr_block()
            .mut_instrs()
            .push(Instr::OneArg(Jmp, Arg::Some(while_cond_start)));
        // generate the block after the while loop
        let after_block = self
            .curr_func()
            .create_block_with_parents(vec![while_cond_end]);
        self.curr_block = after_block;
        self.curr_block().push_dominator(parent_of_while);
        self.generate_phis(last_block);
    }

    fn compile_for_count(
        &mut self,
        name: VarType<'a>,
        expr: &'a Node<u8>,
        exprs: &'a Node<u8>,
        block: &'a Node<u8>,
    ) {
        let block_before_for = self.create_child_block();
        let start_reg = self
            .compile_assignment(name, expr, AssignmentType::LocalDecl)
            .get_reg();
        let exprs = self.get_underlying_exprs(exprs);
        // [end_reg, step_reg]
        let mut regs: Vec<usize> = exprs.iter().map(|e| self.compile_expr(e)).collect();
        if regs.len() > 2 {
            panic!("Too many expression in for-loop.");
        } else if regs.len() == 1 {
            let new_reg = self.curr_func().get_new_reg();
            self.curr_block()
                .mut_instrs()
                .push(Instr::TwoArg(MOV, Arg::Reg(new_reg), Arg::Int(1)));
            regs.push(new_reg);
        }
        // compile while loop condition
        let while_condition_start = self.create_child_block();
        let zero_reg = self.curr_func().get_new_reg();
        self.instrs()
            .push(Instr::TwoArg(MOV, Arg::Reg(zero_reg), Arg::Int(0)));

        let left_reg =
            self.get_operand_of_for_count_condition(zero_reg, start_reg, regs[0], regs[1], true);
        let parent = self.curr_block;
        self.create_child_block();
        let right_reg =
            self.get_operand_of_for_count_condition(zero_reg, start_reg, regs[0], regs[1], false);
        let condition_reg = self.compile_or_short_circuit2(left_reg, right_reg, parent);
        let new_reg = self.curr_func().get_new_reg();
        let additional_instrs = vec![Instr::ThreeArg(
            ADD,
            Arg::Reg(new_reg),
            Arg::Reg(start_reg),
            Arg::Reg(regs[1]),
        )];
        let while_condition_end = self.curr_func().blocks().len() - 1;
        let reg_map_updates = vec![(new_reg, name.get_str())];
        self.compile_while_body(
            block_before_for,
            while_condition_start,
            while_condition_end,
            condition_reg,
            block,
            additional_instrs,
            reg_map_updates,
        );
    }

    fn get_operand_of_for_count_condition(
        &mut self,
        zero_reg: usize,
        start_reg: usize,
        end_reg: usize,
        step_reg: usize,
        is_left: bool,
    ) -> usize {
        let zero_cmp_reg = self.curr_func().get_new_reg();
        self.instrs().push(Instr::ThreeArg(
            if is_left { GE } else { LT },
            Arg::Reg(zero_cmp_reg),
            Arg::Reg(step_reg),
            Arg::Reg(zero_reg),
        ));
        let end_cmp_reg = self.curr_func().get_new_reg();
        let right_instrs = vec![Instr::ThreeArg(
            if is_left { LE } else { GE },
            Arg::Reg(end_cmp_reg),
            Arg::Reg(start_reg),
            Arg::Reg(end_reg),
        )];
        self.compile_and_short_circuit2(zero_cmp_reg, right_instrs)
    }

    fn compile_table_cons(&mut self, node: &'a Node<u8>) -> usize {
        match *node {
            Nonterm {
                ridx: RIdx(ridx),
                ref nodes,
            } if ridx == lua5_3_y::R_TABLECONSTRUCTOR => {
                let table_reg = self.curr_func().get_new_reg();
                self.instrs()
                    .push(Instr::TwoArg(MOV, Arg::Reg(table_reg), Arg::Table));
                let fields = self.compile_field_list(&nodes[1]);
                self.compile_fields(table_reg, fields);
                table_reg
            }
            _ => panic!("Expected <tableconstructor>; got {:?}", node),
        }
    }

    fn compile_field_list(&self, node: &'a Node<u8>) -> Vec<&'a Node<u8>> {
        let mut fields = vec![];
        match *node {
            Nonterm {
                ridx: RIdx(ridx),
                ref nodes,
            } if ridx == lua5_3_y::R_FIELDLISTOPT => {
                if nodes.len() > 0 {
                    fields.extend(self.compile_field_list(&nodes[0]));
                }
            }
            Nonterm {
                ridx: RIdx(ridx),
                ref nodes,
            } if ridx == lua5_3_y::R_FIELDLIST => {
                if nodes.len() > 1 {
                    fields.extend(self.compile_field_list(&nodes[0]));
                    fields.push(&nodes[2]);
                } else {
                    fields.push(&nodes[0]);
                }
            }
            _ => panic!("Expected <fieldlistopt> or <fieldlist>; got {:?}", node),
        }
        fields
    }

    fn compile_fields(&mut self, table_reg: usize, fields: Vec<&'a Node<u8>>) {
        let mut int_index = 1;
        for field in fields {
            match *field {
                Nonterm {
                    ridx: RIdx(ridx),
                    ref nodes,
                } if ridx == lua5_3_y::R_FIELD => {
                    // nodes = [<LSQUARE>, <exp>, <RSQUARE>, <EQ>, <exp>]
                    let (attr_reg, val_reg) = if nodes.len() == 5 {
                        let attr_reg = self.compile_expr(&nodes[1]);
                        let val_reg = self.compile_expr(&nodes[4]);
                        (attr_reg, val_reg)
                    } else if nodes.len() == 3 {
                        let attr_reg = self.curr_func().get_new_reg();
                        let string = self.get_str(&nodes[0]).to_string();
                        self.instrs().push(Instr::TwoArg(
                            MOV,
                            Arg::Reg(attr_reg),
                            Arg::Str(string),
                        ));
                        let val_reg = self.compile_expr(&nodes[2]);
                        (attr_reg, val_reg)
                    } else {
                        let attr_reg = self.curr_func().get_new_reg();
                        self.instrs().push(Instr::TwoArg(
                            MOV,
                            Arg::Reg(attr_reg),
                            Arg::Int(int_index),
                        ));
                        int_index += 1;
                        let val_reg = self.compile_expr(&nodes[0]);
                        (attr_reg, val_reg)
                    };
                    self.instrs().push(Instr::ThreeArg(
                        SetAttr,
                        Arg::Reg(table_reg),
                        Arg::Reg(attr_reg),
                        Arg::Reg(val_reg),
                    ));
                }
                _ => panic!("Expected <field>; got {:?}", field),
            }
        }
    }

    fn compile_do_block(&mut self, node: &'a Node<u8>) {
        match *node {
            Nonterm {
                ridx: RIdx(ridx),
                nodes: _,
            } if ridx == lua5_3_y::R_BLOCK => {
                // the block before the `do` block
                let main_block = self.curr_block;
                self.generate_phis_for_bb(main_block);
                // the `do` block
                let child_block = self.create_child_block();
                self.compile_block_in_basic_block(node, child_block);
                // last block of the `do` block
                let last_block = self.curr_func().blocks().len() - 1;
                self.generate_phis_for_bb(last_block);
                // merge block between last_block and main_block
                let new_curr = self.curr_func().create_block_with_parents(vec![last_block]);
                self.curr_func()
                    .get_mut_block(new_curr)
                    .push_dominator(main_block);
                self.curr_block = new_curr;
                self.generate_phis(main_block);
                self.curr_block = new_curr;
            }
            _ => panic!("Expected <block>; found {:?}", node),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::instr::Arg::*;
    use super::instr::Instr;
    use super::*;
    use std::collections::BTreeMap;
    use std::fmt::Debug;

    fn check_eq<T: Debug + PartialEq>(output: &Vec<T>, expected: &Vec<T>) {
        assert_eq!(output.len(), expected.len());
        for (lhs, rhs) in output.iter().zip(expected.iter()) {
            assert_eq!(lhs, rhs, "{:?} != {:?}", lhs, rhs);
        }
    }

    fn check_instrs_and_parents(
        ir: &LuaIR,
        num_of_funcs: usize,
        expected_instrs: &Vec<Vec<Instr>>,
        expected_parents: &Vec<Vec<usize>>,
        expected_dominators: &Vec<Vec<usize>>,
    ) {
        assert!(ir.functions.len() == num_of_funcs);
        for i in 0..ir.functions.len() {
            let blocks = ir.functions[i].blocks();
            assert!(
                blocks.len() == expected_instrs.len(),
                "len: {}; expected: {}",
                blocks.len(),
                expected_instrs.len()
            );
            for i in 0..blocks.len() {
                let block = &blocks[i];
                check_eq(block.parents(), &expected_parents[i]);
                check_eq(block.dominators(), &expected_dominators[i]);
                check_eq(block.instrs(), &expected_instrs[i]);
            }
        }
    }

    #[test]
    fn simple_math() {
        let pt = &LuaParseTree::from_str(String::from("x = 1 + 2 * 3 / 2 ^ 2.0 // 1 - 2")).unwrap();
        let ir = compile_to_ir(pt);
        let expected_instrs = vec![
            Instr::TwoArg(MOV, Reg(0), Int(1)),
            Instr::TwoArg(MOV, Reg(1), Int(2)),
            Instr::TwoArg(MOV, Reg(2), Int(3)),
            Instr::ThreeArg(MUL, Reg(3), Reg(1), Reg(2)),
            Instr::TwoArg(MOV, Reg(4), Int(2)),
            Instr::TwoArg(MOV, Reg(5), Float(2.0)),
            Instr::ThreeArg(EXP, Reg(6), Reg(4), Reg(5)),
            Instr::ThreeArg(DIV, Reg(7), Reg(3), Reg(6)),
            Instr::TwoArg(MOV, Reg(8), Int(1)),
            Instr::ThreeArg(FDIV, Reg(9), Reg(7), Reg(8)),
            Instr::ThreeArg(ADD, Reg(10), Reg(0), Reg(9)),
            Instr::TwoArg(MOV, Reg(11), Int(2)),
            Instr::ThreeArg(SUB, Reg(12), Reg(10), Reg(11)),
            Instr::ThreeArg(SetUpAttr, Some(0), Str("x".to_string()), Reg(12)),
        ];
        assert!(ir.functions.len() == 1);
        let blocks = &ir.functions[0].blocks();
        assert!(blocks.len() == 1);
        check_eq(blocks[0].instrs(), &expected_instrs);
    }

    #[test]
    fn global_assignment() {
        let pt = &LuaParseTree::from_str(String::from(
            "x = 1
             y = x",
        ))
        .unwrap();
        let ir = compile_to_ir(pt);
        let expected_instrs = vec![
            Instr::TwoArg(MOV, Reg(0), Int(1)),
            Instr::ThreeArg(SetUpAttr, Some(0), Str("x".to_string()), Reg(0)),
            Instr::ThreeArg(GetUpAttr, Reg(1), Some(0), Str("x".to_string())),
            Instr::ThreeArg(SetUpAttr, Some(0), Str("y".to_string()), Reg(1)),
        ];
        assert!(ir.functions.len() == 1);
        let blocks = &ir.functions[ir.main_func].blocks();
        assert!(blocks.len() == 1);
        check_eq(blocks[0].instrs(), &expected_instrs);
    }

    #[test]
    fn locals_and_globals() {
        let pt = &LuaParseTree::from_str(String::from(
            "local x = 2
             y = x",
        ))
        .unwrap();
        let ir = compile_to_ir(pt);
        let expected_instrs = vec![
            Instr::TwoArg(MOV, Reg(0), Int(2)),
            Instr::ThreeArg(SetUpAttr, Some(0), Str("y".to_string()), Reg(0)),
        ];
        assert!(ir.functions.len() == 1);
        let blocks = &ir.functions[0].blocks();
        assert!(blocks.len() == 1);
        check_eq(blocks[0].instrs(), &expected_instrs);
    }

    #[test]
    fn generate_closure() {
        let pt = &LuaParseTree::from_str(String::from(
            "function f()
                 x = 3
             end",
        ))
        .unwrap();
        let ir = compile_to_ir(pt);
        let expected_instrs = vec![
            vec![
                Instr::TwoArg(CLOSURE, Reg(0), Func(1)),
                Instr::ThreeArg(SetUpAttr, Some(0), Str("f".to_string()), Reg(0)),
            ],
            vec![
                Instr::TwoArg(MOV, Reg(0), Int(3)),
                Instr::ThreeArg(SetUpAttr, Some(0), Str("x".to_string()), Reg(0)),
            ],
        ];
        assert!(ir.functions.len() == 2);
        for i in 0..ir.functions.len() {
            let blocks = &ir.functions[i].blocks();
            assert!(blocks.len() == 1);
            check_eq(blocks[0].instrs(), &expected_instrs[i])
        }
    }

    #[test]
    fn generate_call() {
        let pt = &LuaParseTree::from_str(String::from(
            "function f()
                 x = 3
             end
             f()
             f()",
        ))
        .unwrap();
        let ir = compile_to_ir(pt);
        let expected_instrs = vec![
            vec![
                Instr::TwoArg(CLOSURE, Reg(0), Func(1)),
                Instr::ThreeArg(SetUpAttr, Some(0), Str("f".to_string()), Reg(0)),
                Instr::ThreeArg(GetUpAttr, Reg(1), Some(0), Str("f".to_string())),
                Instr::OneArg(SetTop, Reg(1)),
                Instr::OneArg(CALL, Reg(1)),
                Instr::ThreeArg(GetUpAttr, Reg(2), Some(0), Str("f".to_string())),
                Instr::OneArg(SetTop, Reg(2)),
                Instr::OneArg(CALL, Reg(2)),
            ],
            vec![
                Instr::TwoArg(MOV, Reg(0), Int(3)),
                Instr::ThreeArg(SetUpAttr, Some(0), Str("x".to_string()), Reg(0)),
            ],
        ];
        assert!(ir.functions.len() == 2);
        for i in 0..ir.functions.len() {
            let blocks = &ir.functions[i].blocks();
            assert!(blocks.len() == 1);
            check_eq(blocks[0].instrs(), &expected_instrs[i])
        }
    }

    #[test]
    fn generate_functions_with_args() {
        let pt = &LuaParseTree::from_str(String::from(
            "function f(a)
                 x = a
             end
             f(2)
             f(x)",
        ))
        .unwrap();
        let ir = compile_to_ir(pt);
        let expected_instrs = vec![
            vec![
                Instr::TwoArg(CLOSURE, Reg(0), Func(1)),
                Instr::ThreeArg(SetUpAttr, Some(0), Str("f".to_string()), Reg(0)),
                Instr::ThreeArg(GetUpAttr, Reg(1), Some(0), Str("f".to_string())),
                Instr::OneArg(SetTop, Reg(1)),
                Instr::TwoArg(MOV, Reg(2), Int(2)),
                Instr::OneArg(PUSH, Reg(2)),
                Instr::OneArg(CALL, Reg(1)),
                Instr::ThreeArg(GetUpAttr, Reg(3), Some(0), Str("f".to_string())),
                Instr::OneArg(SetTop, Reg(3)),
                Instr::ThreeArg(GetUpAttr, Reg(4), Some(0), Str("x".to_string())),
                Instr::OneArg(PUSH, Reg(4)),
                Instr::OneArg(CALL, Reg(3)),
            ],
            vec![Instr::ThreeArg(
                SetUpAttr,
                Some(0),
                Str("x".to_string()),
                Reg(0),
            )],
        ];
        assert!(ir.functions.len() == 2);
        for i in 0..ir.functions.len() {
            let blocks = &ir.functions[i].blocks();
            assert!(blocks.len() == 1);
            check_eq(blocks[0].instrs(), &expected_instrs[i])
        }
    }

    #[test]
    fn generate_multi_assignments() {
        let pt = &LuaParseTree::from_str(String::from(
            "local x, y, z, z2 = 1, 3
             x, y, z = 1, 4, 5, 6
             a, b = 1",
        ))
        .unwrap();
        let ir = compile_to_ir(pt);
        let expected_instrs = vec![
            Instr::TwoArg(MOV, Reg(0), Int(1)),
            Instr::TwoArg(MOV, Reg(1), Int(3)),
            Instr::TwoArg(MOV, Reg(2), Nil),
            Instr::TwoArg(MOV, Reg(3), Nil),
            Instr::TwoArg(MOV, Reg(4), Int(1)),
            Instr::TwoArg(MOV, Reg(5), Int(4)),
            Instr::TwoArg(MOV, Reg(6), Int(5)),
            Instr::TwoArg(MOV, Reg(7), Int(6)),
            Instr::TwoArg(MOV, Reg(8), Int(1)),
            Instr::TwoArg(MOV, Reg(9), Nil),
            Instr::ThreeArg(SetUpAttr, Some(0), Str("a".to_string()), Reg(8)),
            Instr::ThreeArg(SetUpAttr, Some(0), Str("b".to_string()), Reg(9)),
            Instr::NArg(Phi, vec![Reg(0), Reg(4)]),
            Instr::NArg(Phi, vec![Reg(1), Reg(5)]),
            Instr::NArg(Phi, vec![Reg(2), Reg(6)]),
        ];
        assert!(ir.functions.len() == 1);
        let blocks = &ir.functions[0].blocks();
        assert!(blocks.len() == 1);
        check_eq(blocks[0].instrs(), &expected_instrs);
    }

    #[test]
    fn generate_vararg() {
        let pt = &LuaParseTree::from_str(String::from(
            "function f(a, b, ...)
                 local x, y, z = a, ...
                 f(...)
             end
             f(1, 2, 3, 4)",
        ))
        .unwrap();
        let ir = compile_to_ir(pt);
        let expected_instrs = vec![
            vec![
                Instr::TwoArg(CLOSURE, Reg(0), Func(1)),
                Instr::ThreeArg(SetUpAttr, Some(0), Str("f".to_string()), Reg(0)),
                Instr::ThreeArg(GetUpAttr, Reg(1), Some(0), Str("f".to_string())),
                Instr::OneArg(SetTop, Reg(1)),
                Instr::TwoArg(MOV, Reg(2), Int(1)),
                Instr::OneArg(PUSH, Reg(2)),
                Instr::TwoArg(MOV, Reg(3), Int(2)),
                Instr::OneArg(PUSH, Reg(3)),
                Instr::TwoArg(MOV, Reg(4), Int(3)),
                Instr::OneArg(PUSH, Reg(4)),
                Instr::TwoArg(MOV, Reg(5), Int(4)),
                Instr::OneArg(PUSH, Reg(5)),
                Instr::OneArg(CALL, Reg(1)),
            ],
            vec![
                Instr::TwoArg(MOV, Reg(2), Reg(0)),
                Instr::TwoArg(VarArg, Reg(3), Some(0)),
                Instr::TwoArg(VarArg, Reg(4), Some(1)),
                Instr::ThreeArg(GetUpAttr, Reg(5), Some(0), Str("f".to_string())),
                Instr::OneArg(SetTop, Reg(5)),
                Instr::OneArg(VarArg, Some(1)),
                Instr::OneArg(CALL, Reg(5)),
            ],
        ];
        assert!(ir.functions.len() == 2);
        for i in 0..ir.functions.len() {
            let blocks = &ir.functions[i].blocks();
            assert!(blocks.len() == 1);
            check_eq(blocks[0].instrs(), &expected_instrs[i])
        }
    }

    #[test]
    fn generate_global_vararg() {
        let pt = &LuaParseTree::from_str(String::from(
            "function f(a, b, ...)
                 x, y, z = a, ...
             end
             f(1, 2, 3, 4)",
        ))
        .unwrap();
        let ir = compile_to_ir(pt);
        let expected_instrs = vec![
            vec![
                Instr::TwoArg(CLOSURE, Reg(0), Func(1)),
                Instr::ThreeArg(SetUpAttr, Some(0), Str("f".to_string()), Reg(0)),
                Instr::ThreeArg(GetUpAttr, Reg(1), Some(0), Str("f".to_string())),
                Instr::OneArg(SetTop, Reg(1)),
                Instr::TwoArg(MOV, Reg(2), Int(1)),
                Instr::OneArg(PUSH, Reg(2)),
                Instr::TwoArg(MOV, Reg(3), Int(2)),
                Instr::OneArg(PUSH, Reg(3)),
                Instr::TwoArg(MOV, Reg(4), Int(3)),
                Instr::OneArg(PUSH, Reg(4)),
                Instr::TwoArg(MOV, Reg(5), Int(4)),
                Instr::OneArg(PUSH, Reg(5)),
                Instr::OneArg(CALL, Reg(1)),
            ],
            vec![
                Instr::TwoArg(VarArg, Reg(2), Some(0)),
                Instr::TwoArg(VarArg, Reg(3), Some(1)),
                Instr::ThreeArg(SetUpAttr, Some(0), Str("x".to_string()), Reg(0)),
                Instr::ThreeArg(SetUpAttr, Some(0), Str("y".to_string()), Reg(2)),
                Instr::ThreeArg(SetUpAttr, Some(0), Str("z".to_string()), Reg(3)),
            ],
        ];
        assert!(ir.functions.len() == 2);
        for i in 0..ir.functions.len() {
            let blocks = &ir.functions[i].blocks();
            assert!(blocks.len() == 1);
            check_eq(blocks[0].instrs(), &expected_instrs[i])
        }
    }

    #[test]
    fn generate_return() {
        let pt = &LuaParseTree::from_str(String::from(
            "function f(a, b, ...)
                 return a, ...
             end
             f(1, f(5))",
        ))
        .unwrap();
        let ir = compile_to_ir(pt);
        let expected_instrs = vec![
            vec![
                Instr::TwoArg(CLOSURE, Reg(0), Func(1)),
                Instr::ThreeArg(SetUpAttr, Some(0), Str("f".to_string()), Reg(0)),
                Instr::ThreeArg(GetUpAttr, Reg(1), Some(0), Str("f".to_string())),
                Instr::OneArg(SetTop, Reg(1)),
                Instr::TwoArg(MOV, Reg(2), Int(1)),
                Instr::OneArg(PUSH, Reg(2)),
                Instr::ThreeArg(GetUpAttr, Reg(3), Some(0), Str("f".to_string())),
                Instr::OneArg(SetTop, Reg(3)),
                Instr::TwoArg(MOV, Reg(4), Int(5)),
                Instr::OneArg(PUSH, Reg(4)),
                Instr::OneArg(CALL, Reg(3)),
                Instr::OneArg(MOVR, Some(1)),
                Instr::OneArg(CALL, Reg(1)),
            ],
            vec![
                Instr::ThreeArg(PUSH, Reg(0), Some(0), Some(1)),
                Instr::OneArg(VarArg, Some(2)),
                Instr::ZeroArg(RET),
            ],
        ];
        assert!(ir.functions.len() == 2);
        for i in 0..ir.functions.len() {
            let blocks = &ir.functions[i].blocks();
            assert!(blocks.len() == 1);
            check_eq(blocks[0].instrs(), &expected_instrs[i])
        }
    }

    #[test]
    fn simple_if_else() {
        let pt = &LuaParseTree::from_str(String::from(
            "local a = 1
             if a then
               local b = 2
             else
               local c = 2
             end",
        ))
        .unwrap();
        let ir = compile_to_ir(pt);
        let expected_instrs = vec![
            vec![
                Instr::TwoArg(MOV, Reg(0), Int(1)),
                Instr::ThreeArg(JmpNE, Reg(0), Some(1), Some(2)),
            ],
            vec![
                Instr::TwoArg(MOV, Reg(1), Int(2)),
                Instr::OneArg(Jmp, Some(3)),
            ],
            vec![
                Instr::TwoArg(MOV, Reg(2), Int(2)),
                Instr::OneArg(Jmp, Some(3)),
            ],
            vec![],
        ];
        let expected_parents = vec![vec![], vec![0], vec![0], vec![1, 2]];
        let expected_dominators = vec![vec![], vec![0], vec![0], vec![0]];
        check_instrs_and_parents(
            &ir,
            1,
            &expected_instrs,
            &expected_parents,
            &expected_dominators,
        );
    }

    #[test]
    fn simple_if() {
        let pt = &LuaParseTree::from_str(String::from(
            "local a = 1
             if a then
               local b = 2
             end",
        ))
        .unwrap();
        let ir = compile_to_ir(pt);
        let expected_instrs = vec![
            vec![
                Instr::TwoArg(MOV, Reg(0), Int(1)),
                Instr::ThreeArg(JmpNE, Reg(0), Some(1), Some(2)),
            ],
            vec![
                Instr::TwoArg(MOV, Reg(1), Int(2)),
                Instr::OneArg(Jmp, Some(2)),
            ],
            vec![],
        ];
        let expected_parents = vec![vec![], vec![0], vec![0, 1]];
        let expected_dominators = vec![vec![], vec![0], vec![0]];
        check_instrs_and_parents(
            &ir,
            1,
            &expected_instrs,
            &expected_parents,
            &expected_dominators,
        );
    }

    #[test]
    fn multiple_ifelse() {
        let pt = &LuaParseTree::from_str(String::from(
            "local a, b, c
             if a then
               b = 2
             elseif b then
               b = 3
             elseif c then
               b = 4
             else
               b = 5
             end",
        ))
        .unwrap();
        let ir = compile_to_ir(pt);
        let expected_instrs = vec![
            vec![
                Instr::TwoArg(MOV, Reg(0), Nil),
                Instr::TwoArg(MOV, Reg(1), Nil),
                Instr::TwoArg(MOV, Reg(2), Nil),
                Instr::ThreeArg(JmpNE, Reg(0), Some(1), Some(2)),
            ],
            vec![
                Instr::TwoArg(MOV, Reg(3), Int(2)),
                Instr::OneArg(Jmp, Some(7)),
            ],
            vec![Instr::ThreeArg(JmpNE, Reg(1), Some(3), Some(4))],
            vec![
                Instr::TwoArg(MOV, Reg(4), Int(3)),
                Instr::OneArg(Jmp, Some(7)),
            ],
            vec![Instr::ThreeArg(JmpNE, Reg(2), Some(5), Some(6))],
            vec![
                Instr::TwoArg(MOV, Reg(5), Int(4)),
                Instr::OneArg(Jmp, Some(7)),
            ],
            vec![
                Instr::TwoArg(MOV, Reg(6), Int(5)),
                Instr::OneArg(Jmp, Some(7)),
            ],
            vec![Instr::NArg(
                Phi,
                vec![Reg(1), Reg(3), Reg(4), Reg(5), Reg(6)],
            )],
        ];
        let expected_parents = vec![
            vec![],
            vec![0],
            vec![0],
            vec![2],
            vec![2],
            vec![4],
            vec![4],
            vec![1, 3, 5, 6],
        ];
        let expected_dominators = vec![
            vec![],
            vec![0],
            vec![0],
            vec![2],
            vec![2],
            vec![4],
            vec![4],
            vec![0],
        ];
        check_instrs_and_parents(
            &ir,
            1,
            &expected_instrs,
            &expected_parents,
            &expected_dominators,
        );
    }

    #[test]
    fn ifelse_without_else() {
        let pt = &LuaParseTree::from_str(String::from(
            "local a, b
             if a then
               b = 2
             elseif b then
               b = 3
             end",
        ))
        .unwrap();
        let ir = compile_to_ir(pt);
        let expected_instrs = vec![
            vec![
                Instr::TwoArg(MOV, Reg(0), Nil),
                Instr::TwoArg(MOV, Reg(1), Nil),
                Instr::ThreeArg(JmpNE, Reg(0), Some(1), Some(2)),
            ],
            vec![
                Instr::TwoArg(MOV, Reg(2), Int(2)),
                Instr::OneArg(Jmp, Some(4)),
            ],
            vec![Instr::ThreeArg(JmpNE, Reg(1), Some(3), Some(4))],
            vec![
                Instr::TwoArg(MOV, Reg(3), Int(3)),
                Instr::OneArg(Jmp, Some(4)),
            ],
            vec![Instr::NArg(Phi, vec![Reg(1), Reg(2), Reg(3)])],
        ];
        let expected_parents = vec![vec![], vec![0], vec![0], vec![2], vec![2, 1, 3]];
        let expected_dominators = vec![vec![], vec![0], vec![0], vec![2], vec![2]];
        check_instrs_and_parents(
            &ir,
            1,
            &expected_instrs,
            &expected_parents,
            &expected_dominators,
        );
    }

    #[test]
    fn nested_ifs() {
        let pt = &LuaParseTree::from_str(String::from(
            "local a, b
             if a then
               b = 2
               if b then
                 local c = 3
               end
             end",
        ))
        .unwrap();
        let ir = compile_to_ir(pt);
        let expected_instrs = vec![
            vec![
                Instr::TwoArg(MOV, Reg(0), Nil),
                Instr::TwoArg(MOV, Reg(1), Nil),
                Instr::ThreeArg(JmpNE, Reg(0), Some(1), Some(4)),
            ],
            vec![
                Instr::TwoArg(MOV, Reg(2), Int(2)),
                Instr::ThreeArg(JmpNE, Reg(2), Some(2), Some(3)),
            ],
            vec![
                Instr::TwoArg(MOV, Reg(3), Int(3)),
                Instr::OneArg(Jmp, Some(3)),
            ],
            vec![Instr::NArg(Phi, vec![Reg(2)]), Instr::OneArg(Jmp, Some(4))],
            vec![Instr::NArg(Phi, vec![Reg(1), Reg(2)])],
        ];
        let expected_parents = vec![vec![], vec![0], vec![1], vec![1, 2], vec![0, 3]];
        let expected_dominators = vec![vec![], vec![0], vec![1], vec![1], vec![0]];
        check_instrs_and_parents(
            &ir,
            1,
            &expected_instrs,
            &expected_parents,
            &expected_dominators,
        );
    }

    #[test]
    fn nested_ifs_with_else() {
        let pt = &LuaParseTree::from_str(String::from(
            "local a, b
             if a then
               b = 2
               if b then
                 local c = 3
               else
                 local d = 4
               end
             end",
        ))
        .unwrap();
        let ir = compile_to_ir(pt);
        let expected_instrs = vec![
            vec![
                Instr::TwoArg(MOV, Reg(0), Nil),
                Instr::TwoArg(MOV, Reg(1), Nil),
                Instr::ThreeArg(JmpNE, Reg(0), Some(1), Some(5)),
            ],
            vec![
                Instr::TwoArg(MOV, Reg(2), Int(2)),
                Instr::ThreeArg(JmpNE, Reg(2), Some(2), Some(3)),
            ],
            vec![
                Instr::TwoArg(MOV, Reg(3), Int(3)),
                Instr::OneArg(Jmp, Some(4)),
            ],
            vec![
                Instr::TwoArg(MOV, Reg(4), Int(4)),
                Instr::OneArg(Jmp, Some(4)),
            ],
            vec![Instr::NArg(Phi, vec![Reg(2)]), Instr::OneArg(Jmp, Some(5))],
            vec![Instr::NArg(Phi, vec![Reg(1), Reg(2)])],
        ];
        let expected_parents = vec![vec![], vec![0], vec![1], vec![1], vec![2, 3], vec![0, 4]];
        let expected_dominators = vec![vec![], vec![0], vec![1], vec![1], vec![1], vec![0]];
        check_instrs_and_parents(
            &ir,
            1,
            &expected_instrs,
            &expected_parents,
            &expected_dominators,
        );
    }

    #[test]
    fn while_loops() {
        let pt = &LuaParseTree::from_str(String::from(
            "local a, b = 2, 1
             while a do
                 b = b + 1
             end",
        ))
        .unwrap();
        let ir = compile_to_ir(pt);
        let expected_instrs = vec![
            vec![
                Instr::TwoArg(MOV, Reg(0), Int(2)),
                Instr::TwoArg(MOV, Reg(1), Int(1)),
                Instr::OneArg(Jmp, Some(1)),
            ],
            vec![Instr::ThreeArg(JmpNE, Reg(0), Some(2), Some(3))],
            vec![
                Instr::TwoArg(MOV, Reg(2), Int(1)),
                Instr::ThreeArg(ADD, Reg(3), Reg(1), Reg(2)),
                Instr::OneArg(Jmp, Some(1)),
            ],
            vec![Instr::NArg(Phi, vec![Reg(1), Reg(3)])],
        ];
        let expected_parents = vec![vec![], vec![0], vec![1], vec![1]];
        let expected_dominators = vec![vec![], vec![0], vec![1], vec![0]];
        check_instrs_and_parents(
            &ir,
            1,
            &expected_instrs,
            &expected_parents,
            &expected_dominators,
        );
    }

    #[test]
    fn or_short_circuit() {
        let pt = &LuaParseTree::from_str(String::from("local a = 0 or 1")).unwrap();
        let ir = compile_to_ir(pt);
        let expected_instrs = vec![
            vec![
                Instr::TwoArg(MOV, Reg(0), Int(0)),
                Instr::ThreeArg(JmpEQ, Reg(0), Some(2), Some(1)),
            ],
            vec![Instr::TwoArg(MOV, Reg(1), Int(1))],
            vec![Instr::NArg(Phi, vec![Reg(2), Reg(0), Reg(1)])],
        ];
        let expected_parents = vec![vec![], vec![0], vec![0]];
        let expected_dominators = vec![vec![], vec![0], vec![0]];
        check_instrs_and_parents(
            &ir,
            1,
            &expected_instrs,
            &expected_parents,
            &expected_dominators,
        );
    }

    #[test]
    fn and_short_circuit() {
        let pt = &LuaParseTree::from_str(String::from("local a = 0 and 1")).unwrap();
        let ir = compile_to_ir(pt);
        let expected_instrs = vec![
            vec![
                Instr::TwoArg(MOV, Reg(0), Int(0)),
                Instr::ThreeArg(JmpNE, Reg(0), Some(1), Some(2)),
            ],
            vec![Instr::TwoArg(MOV, Reg(1), Int(1))],
            vec![Instr::NArg(Phi, vec![Reg(2), Reg(0), Reg(1)])],
        ];
        let expected_parents = vec![vec![], vec![0], vec![0]];
        let expected_dominators = vec![vec![], vec![0], vec![0]];
        check_instrs_and_parents(
            &ir,
            1,
            &expected_instrs,
            &expected_parents,
            &expected_dominators,
        );
    }

    #[test]
    fn multiple_prefix_assignments() {
        let pt = &LuaParseTree::from_str(String::from("a[1][2], b, c[3].d = 5, 6")).unwrap();
        let ir = compile_to_ir(pt);
        let expected_instrs = vec![vec![
            Instr::ThreeArg(GetUpAttr, Reg(0), Some(0), Str("a".to_string())),
            Instr::TwoArg(MOV, Reg(1), Int(1)),
            Instr::ThreeArg(GetAttr, Reg(2), Reg(0), Reg(1)),
            Instr::TwoArg(MOV, Reg(3), Int(2)),
            Instr::TwoArg(MOV, Reg(4), Int(5)),
            Instr::TwoArg(MOV, Reg(5), Int(6)),
            Instr::ThreeArg(GetUpAttr, Reg(6), Some(0), Str("c".to_string())),
            Instr::TwoArg(MOV, Reg(7), Int(3)),
            Instr::ThreeArg(GetAttr, Reg(8), Reg(6), Reg(7)),
            Instr::TwoArg(MOV, Reg(9), Str("d".to_string())),
            Instr::TwoArg(MOV, Reg(10), Nil),
            Instr::ThreeArg(SetAttr, Reg(2), Reg(3), Reg(4)),
            Instr::ThreeArg(SetUpAttr, Some(0), Str("b".to_string()), Reg(5)),
            Instr::ThreeArg(SetAttr, Reg(8), Reg(9), Reg(10)),
        ]];
        let expected_parents = vec![vec![]];
        let expected_dominators = vec![vec![]];
        check_instrs_and_parents(
            &ir,
            1,
            &expected_instrs,
            &expected_parents,
            &expected_dominators,
        );
    }

    #[test]
    fn upvals() {
        let pt = &LuaParseTree::from_str(String::from(
            "local a = 2
             function f()
               a = 3
               x = 2
             end",
        ))
        .unwrap();
        let ir = compile_to_ir(pt);
        let expected_instrs = vec![
            vec![
                Instr::TwoArg(MOV, Reg(0), Int(2)),
                Instr::TwoArg(CLOSURE, Reg(1), Func(1)),
                Instr::ThreeArg(SetUpAttr, Some(0), Str("f".to_string()), Reg(1)),
            ],
            vec![
                Instr::TwoArg(MOV, Reg(0), Int(3)),
                Instr::TwoArg(SetUpVal, Some(1), Reg(0)),
                Instr::TwoArg(MOV, Reg(1), Int(2)),
                Instr::ThreeArg(SetUpAttr, Some(0), Str("x".to_string()), Reg(1)),
            ],
        ];
        let expected_provides = vec![
            {
                let mut map = HashMap::new();
                let mut tree = BTreeMap::new();
                tree.insert(1, ProviderType::Reg(0));
                map.insert(1, tree);
                map
            },
            HashMap::new(),
        ];
        for (i, f) in ir.functions.iter().enumerate() {
            assert_eq!(f.provides(), &expected_provides[i]);
            check_eq(f.get_block(0).instrs(), &expected_instrs[i])
        }
    }

    #[test]
    fn upvals2() {
        let pt = &LuaParseTree::from_str(String::from(
            "local a = 2
             function f()
               local b = 3
               function g()
                 local c = a + b
               end
             end",
        ))
        .unwrap();
        let ir = compile_to_ir(pt);
        let expected_instrs = vec![
            vec![
                Instr::TwoArg(MOV, Reg(0), Int(2)),
                Instr::TwoArg(CLOSURE, Reg(1), Func(1)),
                Instr::ThreeArg(SetUpAttr, Some(0), Str("f".to_string()), Reg(1)),
            ],
            vec![
                Instr::TwoArg(MOV, Reg(0), Int(3)),
                Instr::TwoArg(CLOSURE, Reg(1), Func(2)),
                Instr::ThreeArg(SetUpAttr, Some(0), Str("g".to_string()), Reg(1)),
            ],
            vec![
                Instr::TwoArg(GetUpVal, Reg(0), Some(1)),
                Instr::TwoArg(GetUpVal, Reg(1), Some(2)),
                Instr::ThreeArg(ADD, Reg(2), Reg(0), Reg(1)),
            ],
        ];
        let expected_provides = vec![
            {
                let mut map = HashMap::new();
                let mut tree = BTreeMap::new();
                tree.insert(1, ProviderType::Reg(0));
                map.insert(1, tree);
                let mut tree = BTreeMap::new();
                tree.insert(1, ProviderType::Reg(0));
                map.insert(2, tree);
                map
            },
            {
                let mut map = HashMap::new();
                let mut tree = BTreeMap::new();
                tree.insert(2, ProviderType::Reg(0));
                tree.insert(1, ProviderType::Upval(1));
                map.insert(2, tree);
                map
            },
            HashMap::new(),
        ];
        for (i, f) in ir.functions.iter().enumerate() {
            assert_eq!(f.provides(), &expected_provides[i]);
            check_eq(f.get_block(0).instrs(), &expected_instrs[i])
        }
    }

    #[test]
    fn correct_phis_emitted() {
        let pt = &LuaParseTree::from_str(String::from(
            "local a = 2
             a = 3
             if a then
               a = 4
               a = 5
               local a = 6
               a = 7
               local a = 8
               a = 9
             end
             a = 10",
        ))
        .unwrap();
        let ir = compile_to_ir(pt);
        let expected_instrs = vec![
            vec![
                Instr::TwoArg(MOV, Reg(0), Int(2)),
                Instr::TwoArg(MOV, Reg(1), Int(3)),
                Instr::NArg(Phi, vec![Reg(0), Reg(1)]),
                Instr::ThreeArg(JmpNE, Reg(0), Some(1), Some(2)),
            ],
            vec![
                Instr::TwoArg(MOV, Reg(2), Int(4)),
                Instr::TwoArg(MOV, Reg(3), Int(5)),
                Instr::TwoArg(MOV, Reg(4), Int(6)),
                Instr::NArg(Phi, vec![Reg(2), Reg(3)]),
                Instr::TwoArg(MOV, Reg(5), Int(7)),
                Instr::TwoArg(MOV, Reg(6), Int(8)),
                Instr::NArg(Phi, vec![Reg(4), Reg(5)]),
                Instr::TwoArg(MOV, Reg(7), Int(9)),
                Instr::NArg(Phi, vec![Reg(6), Reg(7)]),
                Instr::OneArg(Jmp, Some(2)),
            ],
            vec![
                Instr::NArg(Phi, vec![Reg(0), Reg(2)]),
                Instr::TwoArg(MOV, Reg(8), Int(10)),
                Instr::NArg(Phi, vec![Reg(0), Reg(8)]),
            ],
        ];
        for (i, block) in ir.functions[0].blocks().iter().enumerate() {
            check_eq(block.instrs(), &expected_instrs[i]);
        }
    }

    #[test]
    fn nested_while_loops() {
        let pt = &LuaParseTree::from_str(String::from(
            "local i = 1
             while i < 10 do
               local j = 1
               while j < 5 do
                 j = j + 1
               end
               i = i + 1
             end",
        ))
        .unwrap();
        let ir = compile_to_ir(pt);
        let expected_instrs = vec![
            vec![
                Instr::TwoArg(MOV, Reg(0), Int(1)),
                Instr::OneArg(Jmp, Some(1)),
            ],
            vec![
                Instr::TwoArg(MOV, Reg(1), Int(10)),
                Instr::ThreeArg(LT, Reg(2), Reg(0), Reg(1)),
                Instr::ThreeArg(JmpNE, Reg(2), Some(2), Some(6)),
            ],
            vec![
                Instr::TwoArg(MOV, Reg(3), Int(1)),
                Instr::OneArg(Jmp, Some(3)),
            ],
            vec![
                Instr::TwoArg(MOV, Reg(4), Int(5)),
                Instr::ThreeArg(LT, Reg(5), Reg(3), Reg(4)),
                Instr::ThreeArg(JmpNE, Reg(5), Some(4), Some(5)),
            ],
            vec![
                Instr::TwoArg(MOV, Reg(6), Int(1)),
                Instr::ThreeArg(ADD, Reg(7), Reg(3), Reg(6)),
                Instr::OneArg(Jmp, Some(3)),
            ],
            vec![
                Instr::NArg(Phi, vec![Reg(3), Reg(7)]),
                Instr::TwoArg(MOV, Reg(8), Int(1)),
                Instr::ThreeArg(ADD, Reg(9), Reg(0), Reg(8)),
                Instr::OneArg(Jmp, Some(1)),
            ],
            vec![Instr::NArg(Phi, vec![Reg(0), Reg(9)])],
        ];
        let expected_parents = vec![vec![], vec![0], vec![1], vec![2], vec![3], vec![3], vec![1]];
        let expected_dominators =
            vec![vec![], vec![0], vec![1], vec![2], vec![3], vec![2], vec![0]];
        check_instrs_and_parents(
            &ir,
            1,
            &expected_instrs,
            &expected_parents,
            &expected_dominators,
        );
    }
}
