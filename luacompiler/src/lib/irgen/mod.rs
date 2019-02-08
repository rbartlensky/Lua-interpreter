pub mod compiled_func;
pub mod instr;
pub mod lua_ir;
pub mod register_map;
mod utils;

use self::compiled_func::{BasicBlock, CompiledFunc};
use self::instr::Arg;
use self::lua_ir::LuaIR;
use self::utils::{find_term, get_nodes, is_term};
use bytecode::instructions::Opcode;
use cfgrammar::RIdx;
use irgen::register_map::RegisterMap;
use lrpar::Node::{self, *};
use lua5_3_l;
use lua5_3_y;
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

enum VariableType {
    Local(usize),
    Global(usize),
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
        self.compile_block(&self.pt.tree);
        LuaIR::new(self.functions, 0)
    }

    fn curr_func(&mut self) -> &mut CompiledFunc<'a> {
        &mut self.functions[self.curr_func]
    }

    fn curr_reg_map(&mut self) -> &mut RegisterMap<'a> {
        self.functions[self.curr_func].reg_map()
    }

    fn curr_block(&mut self) -> &mut BasicBlock {
        let i = self.curr_block;
        self.functions[self.curr_func].get_block(i)
    }

    fn push_instr(&mut self, op: Opcode, args: Vec<Arg>) {
        self.curr_block().push_instr(op, args);
    }

    /// Compile a <block>.
    fn compile_block(&mut self, node: &'a Node<u8>) {
        self.curr_reg_map().push_block();
        let block_num = self.compile_block_without_scope(node);
        self.curr_reg_map().pop_block(block_num);
    }

    /// Compiles a block in the current scope. This means that the user must
    /// manully push/pop a new scope, if necessary.
    fn compile_block_without_scope(&mut self, node: &'a Node<u8>) -> usize {
        let old_block = self.curr_block;
        let index = self.curr_func().create_block();
        self.curr_block = index;
        // nodes = [<statlistopt>, <retstatopt>]
        let nodes = get_nodes(node, lua5_3_y::R_BLOCK);
        self.compile_stat_list(&nodes[0]);
        self.compile_retstat(&nodes[1]);
        self.curr_block = old_block;
        index
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
                        self.push_instr(
                            Opcode::PUSH,
                            vec![Arg::Reg(reg), Arg::Some(0), Arg::Some(1)],
                        );
                    }
                    self.unpack_to_stack(&exprs.last().unwrap(), true);
                    self.push_instr(Opcode::RET, vec![]);
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
                debug_assert!(
                    last_instr.opcode == Opcode::MOVR || last_instr.opcode == Opcode::VarArg
                );
                // check bytecode/instructions.rs for more info on why we set the third
                // argument to 1 or 2
                last_instr.args = vec![
                    Arg::Some(0),
                    Arg::Some(0),
                    Arg::Some(1 + increment_ret_vals as usize),
                ];
            }
            // compile_expr will generate (VarArg/MOVR <new_reg> <op2> <op3>)
            // but because we are modifying the last instruction, there is
            // no need to keep the previously allocated register
            self.curr_reg_map().pop_last_reg();
        } else {
            if increment_ret_vals {
                self.push_instr(
                    Opcode::PUSH,
                    vec![Arg::Reg(reg), Arg::Some(0), Arg::Some(1)],
                );
            } else {
                self.push_instr(Opcode::PUSH, vec![Arg::Reg(reg)]);
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
                        let exprs = self.get_underlying_exprs(&nodes[1]);
                        self.compile_local_assignments(names, exprs);
                    }
                    _ => {}
                }
            } else {
                match (&stat_nodes[0], &stat_nodes[1]) {
                    // stat_nodes = [<function>, <funcname>, <funcbody>]
                    (Term { lexeme }, _) if lexeme.tok_id() == lua5_3_l::T_FUNCTION => {
                        let name = self.compile_variable(&stat_nodes[1]);
                        self.compile_assignment(name, &stat_nodes[2], AssignmentType::Regular);
                    }
                    // stat_nodes = [<varlist>, <eq>, <explist>]
                    (_, Term { lexeme }) if lexeme.tok_id() == lua5_3_l::T_EQ => {
                        // x, y, z = 1, 2
                        let names = self.compile_names(&stat_nodes[0]);
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
                } if ridx == lua5_3_y::R_FUNCTIONCALL => self.compile_call(&nodes[0], &nodes[1]),
                _ => {}
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
            self.compile_assignment(names[i], exprs[i], AssignmentType::LocalDecl);
        }
        // for all the remaining names (c, d), create a new empty register, because the
        // user might access the variable later
        if names.len() > exprs.len() {
            let mut regs = vec![];
            for i in exprs.len()..names.len() {
                self.curr_reg_map().create_reg(names[i]);
                let reg = self.curr_reg_map().reg_count() - 1;
                regs.push(reg);
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
                    self.push_instr(Opcode::MOV, vec![Arg::Reg(reg), Arg::Nil]);
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
    fn compile_assignments(&mut self, names: Vec<&'a str>, exprs: Vec<&'a Node<u8>>) {
        // we want to emit _ENV[<name>] = <reg> only after we assign all expressions into
        // registers. This is because of how vararg expects registers to be ordered.
        // For instance `a, b = ...`, will generate `VarArg 3, 2, 0` meaning that the vm
        // will copy two variable arguments into registers 3 and 4. We have to make sure
        // that a, and b point to consecutive registers, but a global assignment will
        // generate additional instructions, which we try to postpone
        let mut postponed_envs: Vec<(&str, usize)> = vec![];
        // example: x, y, z, w = 1, 2
        // compile x = 1, y = 2
        for (name, expr) in names.iter().zip(exprs.iter()) {
            let res = self.compile_assignment(name, expr, AssignmentType::Postponed);
            if let VariableType::Global(reg) = res {
                postponed_envs.push((name, reg));
            }
        }
        // for all the remaining names (z, w), create a new empty register, and update
        // _ENV if the variable has not been declared as local in some outer scope
        // names.len() == exprs.len() is intentionally left out because that case is
        // handled by the loop above
        if names.len() > exprs.len() {
            let mut regs = vec![];
            for i in exprs.len()..names.len() {
                let reg = self.curr_reg_map().get_new_reg();
                if !self.curr_reg_map().is_local(names[i]) {
                    postponed_envs.push((names[i], reg));
                } else {
                    self.curr_reg_map().set_reg(names[i], reg);
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
                    self.push_instr(Opcode::MOV, vec![Arg::Reg(reg), Arg::Nil]);
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
        for (name, reg) in postponed_envs {
            self.push_instr(
                Opcode::SetUpAttr,
                vec![Arg::Some(0), Arg::Str(name.to_string()), Arg::Reg(reg)],
            );
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
        let opcode = if self.is_vararg(expr) {
            Opcode::VarArg
        } else {
            Opcode::MOVR
        };
        for (i, reg) in regs.iter().enumerate() {
            self.push_instr(opcode, vec![Arg::Reg(*reg), Arg::Some(i + 1)]);
        }
    }

    /// Compile an assignment by compiling <right> and then storing the result in <left>.
    /// * `left` - The name of the variable in which the result is stored
    /// * `right` - The expression that is evaluated
    /// * `action` - How the compiler should behave, see @AssignmentType for more info.
    /// Returns whether the assignment was local or global.
    fn compile_assignment(
        &mut self,
        name: &'a str,
        right: &'a Node<u8>,
        action: AssignmentType,
    ) -> VariableType {
        let old_len = self.curr_block().instrs().len();
        let mut value = self.compile_expr(right);
        // the register map only keeps track of local variables
        // if we are compiling: `x = 3`, then we also have to check if x is in `reg_map`
        // if it is, then it is a local assignment (because `reg_map` only stores
        // mappings of local variable to registers), if it isn't then we load it from
        // the global mapping
        if action == AssignmentType::LocalDecl || self.curr_reg_map().get_reg(name).is_some() {
            // No new instructions were added, which means that <right> has already been
            // computed and stored in some register. Because we are compiling an
            // assignment, we will create a copy of this result and store it in <left>.
            // See test `load_string_multiple_times`.
            if self.curr_block().instrs().len() == old_len {
                let new_reg = self.curr_reg_map().get_new_reg();
                self.push_instr(Opcode::MOV, vec![Arg::Reg(new_reg), Arg::Reg(value)]);
                value = new_reg;
            }
            // if a variable is assigned a value multiple times, we have to make sure
            // that the map knows the new register which holds the new value
            self.curr_reg_map().set_reg(name, value);
            VariableType::Local(value)
        } else {
            if action != AssignmentType::Postponed {
                self.push_instr(
                    Opcode::SetUpAttr,
                    vec![Arg::Some(0), Arg::Str(name.to_string()), Arg::Reg(value)],
                );
            }
            VariableType::Global(value)
        }
    }

    /// Jumps to the first child of <node> which denotes a variable name.
    fn compile_variable(&self, node: &Node<u8>) -> &'a str {
        let name = find_term(node, lua5_3_l::T_NAME);
        match name {
            Some(Term { lexeme }) => self
                .pt
                .get_string(lexeme.start(), lexeme.end().unwrap_or(lexeme.start())),
            _ => {
                panic!("Must have assignments of form: var = expr!");
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
            } if ridx == lua5_3_y::R_FUNCBODY => {
                let old_curr_func = self.curr_func;
                // create a new `CompiledFunc` for this function
                let new_func_id = self.functions.len();
                let param_nodes = get_nodes(&nodes[1], lua5_3_y::R_PARLIST);
                let mut param_count = param_nodes.len();
                let is_vararg =
                    param_count > 0 && is_term(param_nodes.last().unwrap(), lua5_3_l::T_DOTDOTDOT);
                let new_func = CompiledFunc::new(0, is_vararg);
                self.functions.push(new_func);
                self.curr_func = new_func_id;
                self.curr_reg_map().push_block();
                // make the first N registers point to the first N parameters
                self.compile_param_list(&nodes[1]);
                let block = self.compile_block_without_scope(&nodes[3]);
                self.curr_reg_map().pop_block(block);
                // restore the old state so that we can create a closure instruction
                // in the outer function
                self.curr_func = old_curr_func;
                let reg = self.curr_reg_map().get_new_reg();
                self.push_instr(Opcode::CLOSURE, vec![Arg::Reg(reg), Arg::Func(new_func_id)]);
                reg
            }
            Nonterm {
                ridx: RIdx(ridx),
                ref nodes,
            } if ridx == lua5_3_y::R_FUNCTIONCALL => {
                self.compile_call(&nodes[0], &nodes[1]);
                let reg = self.curr_reg_map().get_new_reg();
                self.push_instr(Opcode::MOVR, vec![Arg::Reg(reg), Arg::Some(0)]);
                reg
            }
            Nonterm {
                ridx: RIdx(_ridx),
                ref nodes,
            } => {
                if nodes.len() == 1 {
                    self.compile_expr(&nodes[0])
                } else {
                    debug_assert!(nodes.len() == 3);
                    let left = self.compile_expr(&nodes[0]);
                    let right = self.compile_expr(&nodes[2]);
                    let new_var = self.curr_reg_map().get_new_reg();
                    let (opcode, args) = self.get_instr(&nodes[1], new_var, left, right);
                    self.push_instr(opcode, args);
                    new_var
                }
            }
            Term { lexeme } => {
                let value = self
                    .pt
                    .get_string(lexeme.start(), lexeme.end().unwrap_or(lexeme.start()));
                match lexeme.tok_id() {
                    lua5_3_l::T_NUMERAL => {
                        let new_reg = self.curr_reg_map().get_new_reg();
                        let arg = if value.contains(".") {
                            Arg::Float(value.parse().unwrap())
                        } else {
                            Arg::Int(value.parse().unwrap())
                        };
                        self.push_instr(Opcode::MOV, vec![Arg::Reg(new_reg), arg]);
                        new_reg
                    }
                    lua5_3_l::T_SHORT_STR => {
                        let new_reg = self.curr_reg_map().get_new_reg();
                        self.push_instr(
                            Opcode::MOV,
                            vec![
                                Arg::Reg(new_reg),
                                Arg::Str(value[1..(value.len() - 1)].to_string()),
                            ],
                        );
                        new_reg
                    }
                    lua5_3_l::T_NAME => match self.curr_reg_map().get_reg(value) {
                        Some(reg) => reg,
                        None => {
                            let reg = self.curr_reg_map().get_new_reg();
                            self.push_instr(
                                Opcode::GetUpAttr,
                                vec![Arg::Reg(reg), Arg::Some(0), Arg::Str(value.to_string())],
                            );
                            reg
                        }
                    },
                    lua5_3_l::T_DOTDOTDOT => {
                        if self.curr_func().is_vararg() {
                            let reg = self.curr_reg_map().get_new_reg();
                            self.push_instr(Opcode::VarArg, vec![Arg::Reg(reg), Arg::Some(0)]);
                            reg
                        } else {
                            panic!("Cannot use '...' outside of a vararg function.")
                        }
                    }
                    _ => panic!(
                        "Cannot compile terminals that are not variable names, numbers or strings."
                    ),
                }
            }
        }
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

    /// Compile a <namelist> or a <varlist> into a vector of names.
    fn compile_names(&mut self, names: &Node<u8>) -> Vec<&'a str> {
        match *names {
            Nonterm {
                ridx: RIdx(ridx),
                ref nodes,
            } if ridx == lua5_3_y::R_NAMELIST || ridx == lua5_3_y::R_VARLIST => {
                let mut names = vec![];
                // nodes = <NAME>
                if nodes.len() == 1 {
                    names.push(self.compile_variable(&nodes[0]));
                } else {
                    // nodes = [<name/varlist>, <COMMA>, <NAME>]
                    names.extend(self.compile_names(&nodes[0]));
                    names.push(self.compile_variable(&nodes[2]));
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
    fn compile_param_list(&mut self, params: &Node<u8>) {
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
                let mut reg_map = self.curr_reg_map();
                for name in names {
                    reg_map.create_reg(name);
                }
            }
            _ => panic!("Root node was not a <parlist>"),
        }
    }

    /// Compile a <functioncall>.
    fn compile_call(&mut self, func: &'a Node<u8>, params: &'a Node<u8>) {
        let func_reg = self.compile_expr(find_term(func, lua5_3_l::T_NAME).unwrap());
        let params = match *params {
            Nonterm {
                ridx: RIdx(ridx),
                ref nodes,
            } if ridx == lua5_3_y::R_ARGS => &nodes[1],
            _ => panic!("Missing node <args> from <functioncall>"),
        };
        self.push_instr(Opcode::SetTop, vec![Arg::Reg(func_reg)]);
        let exprs = self.get_underlying_exprs(params);
        if exprs.len() > 0 {
            // push the arguments to the function
            for i in 0..(exprs.len() - 1) {
                let reg = self.compile_expr(exprs[i]);
                self.push_instr(Opcode::PUSH, vec![Arg::Reg(reg)]);
            }
            self.unpack_to_stack(&exprs.last().unwrap(), false);
        }
        self.push_instr(Opcode::CALL, vec![Arg::Reg(func_reg)])
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
    fn get_instr(
        &self,
        node: &'a Node<u8>,
        reg: usize,
        lreg: usize,
        rreg: usize,
    ) -> (Opcode, Vec<Arg>) {
        if let Term { lexeme } = node {
            let opcode = match lexeme.tok_id() {
                lua5_3_l::T_PLUS => Opcode::ADD,
                lua5_3_l::T_MINUS => Opcode::SUB,
                lua5_3_l::T_STAR => Opcode::MUL,
                lua5_3_l::T_FSLASH => Opcode::DIV,
                lua5_3_l::T_MOD => Opcode::MOD,
                lua5_3_l::T_FSFS => Opcode::FDIV,
                lua5_3_l::T_CARET => Opcode::EXP,
                lua5_3_l::T_EQEQ => Opcode::EQ,
                _ => unimplemented!("Instruction {:#?}", node),
            };
            (opcode, vec![Arg::Reg(reg), Arg::Reg(lreg), Arg::Reg(rreg)])
        } else {
            panic!("Expected a Node::Term!");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::instr::Arg::*;
    use super::instr::Instr;
    use super::*;
    use bytecode::instructions::Opcode::*;
    use std::fmt::Debug;

    fn check_eq<T: Debug + PartialEq>(output: &Vec<T>, expected: &Vec<T>) {
        assert_eq!(output.len(), expected.len());
        for (lhs, rhs) in output.iter().zip(expected.iter()) {
            assert_eq!(lhs, rhs);
        }
    }

    #[test]
    fn simple_math() {
        let pt = &LuaParseTree::from_str(String::from("x = 1 + 2 * 3 / 2 ^ 2.0 // 1 - 2")).unwrap();
        let ir = compile_to_ir(pt);
        let expected_instrs = vec![
            Instr::new(MOV, vec![Reg(0), Int(1)]),
            Instr::new(MOV, vec![Reg(1), Int(2)]),
            Instr::new(MOV, vec![Reg(2), Int(3)]),
            Instr::new(MUL, vec![Reg(3), Reg(1), Reg(2)]),
            Instr::new(MOV, vec![Reg(4), Int(2)]),
            Instr::new(MOV, vec![Reg(5), Float(2.0)]),
            Instr::new(EXP, vec![Reg(6), Reg(4), Reg(5)]),
            Instr::new(DIV, vec![Reg(7), Reg(3), Reg(6)]),
            Instr::new(MOV, vec![Reg(8), Int(1)]),
            Instr::new(FDIV, vec![Reg(9), Reg(7), Reg(8)]),
            Instr::new(ADD, vec![Reg(10), Reg(0), Reg(9)]),
            Instr::new(MOV, vec![Reg(11), Int(2)]),
            Instr::new(SUB, vec![Reg(12), Reg(10), Reg(11)]),
            Instr::new(SetUpAttr, vec![Some(0), Str("x".to_string()), Reg(12)]),
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
            Instr::new(MOV, vec![Reg(0), Int(1)]),
            Instr::new(SetUpAttr, vec![Some(0), Str("x".to_string()), Reg(0)]),
            Instr::new(GetUpAttr, vec![Reg(1), Some(0), Str("x".to_string())]),
            Instr::new(SetUpAttr, vec![Some(0), Str("y".to_string()), Reg(1)]),
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
            Instr::new(MOV, vec![Reg(0), Int(2)]),
            Instr::new(SetUpAttr, vec![Some(0), Str("y".to_string()), Reg(0)]),
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
                Instr::new(CLOSURE, vec![Reg(0), Func(1)]),
                Instr::new(SetUpAttr, vec![Some(0), Str("f".to_string()), Reg(0)]),
            ],
            vec![
                Instr::new(MOV, vec![Reg(0), Int(3)]),
                Instr::new(SetUpAttr, vec![Some(0), Str("x".to_string()), Reg(0)]),
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
                Instr::new(CLOSURE, vec![Reg(0), Func(1)]),
                Instr::new(SetUpAttr, vec![Some(0), Str("f".to_string()), Reg(0)]),
                Instr::new(GetUpAttr, vec![Reg(1), Some(0), Str("f".to_string())]),
                Instr::new(SetTop, vec![Reg(1)]),
                Instr::new(CALL, vec![Reg(1)]),
                Instr::new(GetUpAttr, vec![Reg(2), Some(0), Str("f".to_string())]),
                Instr::new(SetTop, vec![Reg(2)]),
                Instr::new(CALL, vec![Reg(2)]),
            ],
            vec![
                Instr::new(MOV, vec![Reg(0), Int(3)]),
                Instr::new(SetUpAttr, vec![Some(0), Str("x".to_string()), Reg(0)]),
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
                Instr::new(CLOSURE, vec![Reg(0), Func(1)]),
                Instr::new(SetUpAttr, vec![Some(0), Str("f".to_string()), Reg(0)]),
                Instr::new(GetUpAttr, vec![Reg(1), Some(0), Str("f".to_string())]),
                Instr::new(SetTop, vec![Reg(1)]),
                Instr::new(MOV, vec![Reg(2), Int(2)]),
                Instr::new(PUSH, vec![Reg(2)]),
                Instr::new(CALL, vec![Reg(1)]),
                Instr::new(GetUpAttr, vec![Reg(3), Some(0), Str("f".to_string())]),
                Instr::new(SetTop, vec![Reg(3)]),
                Instr::new(GetUpAttr, vec![Reg(4), Some(0), Str("x".to_string())]),
                Instr::new(PUSH, vec![Reg(4)]),
                Instr::new(CALL, vec![Reg(3)]),
            ],
            vec![Instr::new(
                SetUpAttr,
                vec![Some(0), Str("x".to_string()), Reg(0)],
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
            Instr::new(MOV, vec![Reg(0), Int(1)]),
            Instr::new(MOV, vec![Reg(1), Int(3)]),
            Instr::new(MOV, vec![Reg(2), Nil]),
            Instr::new(MOV, vec![Reg(3), Nil]),
            Instr::new(MOV, vec![Reg(4), Int(1)]),
            Instr::new(MOV, vec![Reg(5), Int(4)]),
            Instr::new(MOV, vec![Reg(6), Int(5)]),
            Instr::new(MOV, vec![Reg(7), Int(6)]),
            Instr::new(MOV, vec![Reg(8), Int(1)]),
            Instr::new(MOV, vec![Reg(9), Nil]),
            Instr::new(SetUpAttr, vec![Some(0), Str("a".to_string()), Reg(8)]),
            Instr::new(SetUpAttr, vec![Some(0), Str("b".to_string()), Reg(9)]),
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
                Instr::new(CLOSURE, vec![Reg(0), Func(1)]),
                Instr::new(SetUpAttr, vec![Some(0), Str("f".to_string()), Reg(0)]),
                Instr::new(GetUpAttr, vec![Reg(1), Some(0), Str("f".to_string())]),
                Instr::new(SetTop, vec![Reg(1)]),
                Instr::new(MOV, vec![Reg(2), Int(1)]),
                Instr::new(PUSH, vec![Reg(2)]),
                Instr::new(MOV, vec![Reg(3), Int(2)]),
                Instr::new(PUSH, vec![Reg(3)]),
                Instr::new(MOV, vec![Reg(4), Int(3)]),
                Instr::new(PUSH, vec![Reg(4)]),
                Instr::new(MOV, vec![Reg(5), Int(4)]),
                Instr::new(PUSH, vec![Reg(5)]),
                Instr::new(CALL, vec![Reg(1)]),
            ],
            vec![
                Instr::new(MOV, vec![Reg(2), Reg(0)]),
                Instr::new(VarArg, vec![Reg(3), Some(0)]),
                Instr::new(VarArg, vec![Reg(4), Some(1)]),
                Instr::new(GetUpAttr, vec![Reg(5), Some(0), Str("f".to_string())]),
                Instr::new(SetTop, vec![Reg(5)]),
                Instr::new(VarArg, vec![Some(0), Some(0), Some(1)]),
                Instr::new(CALL, vec![Reg(5)]),
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
                Instr::new(CLOSURE, vec![Reg(0), Func(1)]),
                Instr::new(SetUpAttr, vec![Some(0), Str("f".to_string()), Reg(0)]),
                Instr::new(GetUpAttr, vec![Reg(1), Some(0), Str("f".to_string())]),
                Instr::new(SetTop, vec![Reg(1)]),
                Instr::new(MOV, vec![Reg(2), Int(1)]),
                Instr::new(PUSH, vec![Reg(2)]),
                Instr::new(MOV, vec![Reg(3), Int(2)]),
                Instr::new(PUSH, vec![Reg(3)]),
                Instr::new(MOV, vec![Reg(4), Int(3)]),
                Instr::new(PUSH, vec![Reg(4)]),
                Instr::new(MOV, vec![Reg(5), Int(4)]),
                Instr::new(PUSH, vec![Reg(5)]),
                Instr::new(CALL, vec![Reg(1)]),
            ],
            vec![
                Instr::new(VarArg, vec![Reg(2), Some(0)]),
                Instr::new(VarArg, vec![Reg(3), Some(1)]),
                Instr::new(SetUpAttr, vec![Some(0), Str("x".to_string()), Reg(0)]),
                Instr::new(SetUpAttr, vec![Some(0), Str("y".to_string()), Reg(2)]),
                Instr::new(SetUpAttr, vec![Some(0), Str("z".to_string()), Reg(3)]),
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
                Instr::new(CLOSURE, vec![Reg(0), Func(1)]),
                Instr::new(SetUpAttr, vec![Some(0), Str("f".to_string()), Reg(0)]),
                Instr::new(GetUpAttr, vec![Reg(1), Some(0), Str("f".to_string())]),
                Instr::new(SetTop, vec![Reg(1)]),
                Instr::new(MOV, vec![Reg(2), Int(1)]),
                Instr::new(PUSH, vec![Reg(2)]),
                Instr::new(GetUpAttr, vec![Reg(3), Some(0), Str("f".to_string())]),
                Instr::new(SetTop, vec![Reg(3)]),
                Instr::new(MOV, vec![Reg(4), Int(5)]),
                Instr::new(PUSH, vec![Reg(4)]),
                Instr::new(CALL, vec![Reg(3)]),
                Instr::new(MOVR, vec![Some(0), Some(0), Some(1)]),
                Instr::new(CALL, vec![Reg(1)]),
            ],
            vec![
                Instr::new(PUSH, vec![Reg(0), Some(0), Some(1)]),
                Instr::new(VarArg, vec![Some(0), Some(0), Some(2)]),
                Instr::new(RET, vec![]),
            ],
        ];
        assert!(ir.functions.len() == 2);
        for i in 0..ir.functions.len() {
            let blocks = &ir.functions[i].blocks();
            assert!(blocks.len() == 1);
            check_eq(blocks[0].instrs(), &expected_instrs[i])
        }
    }
}
