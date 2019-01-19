pub mod compiled_func;
pub mod constants_map;
pub mod lua_ir;
pub mod register_map;
mod utils;

use self::utils::{find_term, get_nodes, is_term};
use self::{
    compiled_func::CompiledFunc, constants_map::ConstantsMap, lua_ir::LuaIR,
    register_map::RegisterMap,
};
use bytecode::instructions::{HLInstr, Opcode};
use cfgrammar::RIdx;
use lrpar::Node::{self, *};
use lua5_3_l;
use lua5_3_y;
use LuaParseTree;

/// Compile the given parse tree into an SSA IR.
pub fn compile_to_ir(pt: &LuaParseTree) -> LuaIR {
    LuaToIR::new(pt).to_lua_ir()
}

/// Represents a compiler which translates a given Lua parse tree to an SSA IR.
/// The compiler assumes that the `_ENV` variable is always stored in register 0!
struct LuaToIR<'a> {
    pt: &'a LuaParseTree,
    const_map: ConstantsMap,
    functions: Vec<CompiledFunc<'a>>,
    curr_function: usize,
}

impl<'a> LuaToIR<'a> {
    fn new(pt: &'a LuaParseTree) -> LuaToIR {
        LuaToIR {
            pt,
            const_map: ConstantsMap::new(),
            functions: vec![CompiledFunc::new(0)],
            curr_function: 0,
        }
    }

    /// Compile and return the intermediate representation of the given lua parse tree.
    pub fn to_lua_ir(mut self) -> LuaIR<'a> {
        self.compile_block(&self.pt.tree);
        LuaIR::new(self.functions, self.curr_function, self.const_map)
    }

    fn curr_reg_map(&mut self) -> &mut RegisterMap<'a> {
        self.functions[self.curr_function].mut_reg_map()
    }

    /// Compile a <block> without recursively compiling its <retstatopt> child.
    fn compile_block(&mut self, node: &Node<u8>) {
        self.curr_reg_map().push_scope();
        self.compile_block_without_scope(node);
        self.curr_reg_map().pop_scope();
    }

    fn compile_block_without_scope(&mut self, node: &Node<u8>) {
        // nodes = [<statlistopt>, <retstatopt>]
        let nodes = get_nodes(node, lua5_3_y::R_BLOCK);
        self.compile_stat_list(&nodes[0]);
    }

    /// Compile a <statlist> or a <statlistopt>.
    fn compile_stat_list(&mut self, node: &Node<u8>) {
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
    fn compile_stat(&mut self, stat_nodes: &Vec<Node<u8>>) {
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
                        // left hand-side = <namelist> and right hand-side = <explist>
                        self.compile_assignment(&stat_nodes[1], &nodes[1], true);
                    }
                    _ => {}
                }
            } else {
                match (&stat_nodes[0], &stat_nodes[1]) {
                    // stat_nodes = [<function>, <funcname>, <funcbody>]
                    (Term { lexeme }, _) if lexeme.tok_id() == lua5_3_l::T_FUNCTION => {
                        self.compile_assignment(&stat_nodes[1], &stat_nodes[2], false);
                    }
                    // stat_nodes = [<varlist>, <eq>, <explist>]
                    (_, Term { lexeme }) if lexeme.tok_id() == lua5_3_l::T_EQ => {
                        self.compile_assignment(&stat_nodes[0], &stat_nodes[2], false);
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

    /// Compile an assignment by compiling <right> and then storing the result in <left>.
    /// * `left` - The variable in which the result is stored
    /// * `right` - The expression that is evaluated
    /// * `is_local_decl` - Whether the assignment is local or not.
    fn compile_assignment(&mut self, left: &Node<u8>, right: &Node<u8>, is_local_decl: bool) {
        let old_len = self.functions[self.curr_function].instrs().len();
        let mut value = self.compile_expr(right);
        let name = self.compile_variable(left);
        // the register map only keeps track of local variables
        // if we are compiling: `x = 3`, then we also have to check if x is in `reg_map`
        // if it is, then it is a local assignment (because `reg_map` only stores
        // mappings of local variable to registers), if it isn't then we have to look
        // it up in _ENV
        if is_local_decl || self.curr_reg_map().get_reg(name).is_some() {
            // No new instructions were added, which means that <right> has already been
            // computed and stored in some register. Because we are compiling an
            // assignment, we will create a copy of this result and store it in <left>.
            // See test `load_string_multiple_times`.
            if self.functions[self.curr_function].instrs().len() == old_len {
                let new_reg = self.curr_reg_map().get_new_reg();
                self.functions[self.curr_function].push_instr(HLInstr(
                    Opcode::MOV,
                    new_reg,
                    value,
                    0,
                ));
                value = new_reg;
            }
            // if a variable is assigned a value multiple times, we have to make sure
            // that the map knows the new register which holds the new value
            self.curr_reg_map().set_reg(name, value);
        } else {
            // we would like to generate code for the following statement: _ENV[name] = value
            // load a reference to _ENV
            let env_reg = self.curr_reg_map().get_reg("_ENV").unwrap();
            // prepare the attribute for _ENV which is the name of the variable
            let name_index = self.const_map.get_str(name.to_string());
            let attr_reg = self.get_const_str_reg(name_index);
            self.functions[self.curr_function].push_instr(HLInstr(
                Opcode::SetAttr,
                env_reg,
                attr_reg,
                value,
            ));
        }
    }

    /// Get the register which contains the constant string <name_index>.
    /// If no register holds this value, then this method creates the necessary
    /// instruction to load the string.
    fn get_const_str_reg(&mut self, name_index: usize) -> usize {
        match self.curr_reg_map().get_str_reg(name_index) {
            Some(i) => i,
            None => {
                let reg = self.curr_reg_map().get_new_reg();
                self.functions[self.curr_function].push_instr(HLInstr(
                    Opcode::LDS,
                    reg,
                    name_index,
                    0,
                ));
                self.curr_reg_map().set_str_reg(name_index, reg);
                reg
            }
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
    fn compile_expr(&mut self, node: &Node<u8>) -> usize {
        match *node {
            Nonterm {
                ridx: RIdx(ridx),
                ref nodes,
            } if ridx == lua5_3_y::R_FUNCBODY => {
                let old_curr_function = self.curr_function;
                // create a new compiledfunc for this function, and add it as a child
                // of the enclosing function
                let new_function_id = self.functions.len();
                let (new_function, func_num) = {
                    let mut curr_function = &mut self.functions[self.curr_function];
                    curr_function.push_func(new_function_id);
                    (
                        CompiledFunc::new(new_function_id),
                        curr_function.funcs_len() - 1,
                    )
                };
                self.functions.push(new_function);
                self.curr_function = new_function_id;
                self.curr_reg_map().push_scope();
                // make the first N registers point to the first N parameters
                self.compile_param_list(&nodes[1]);
                self.compile_block_without_scope(&nodes[3]);
                self.curr_reg_map().pop_scope();
                // restore the old state so that we can create a closure instruction
                // in the outer function
                self.curr_function = old_curr_function;
                let reg = self.curr_reg_map().get_new_reg();
                self.functions[self.curr_function].push_instr(HLInstr(
                    Opcode::CLOSURE,
                    reg,
                    func_num,
                    0,
                ));
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
                    let instr = self.get_instr(&nodes[1], new_var, left, right);
                    self.functions[self.curr_function].push_instr(instr);
                    new_var
                }
            }
            Term { lexeme } => {
                let value = self
                    .pt
                    .get_string(lexeme.start(), lexeme.end().unwrap_or(lexeme.start()));
                match lexeme.tok_id() {
                    lua5_3_l::T_NUMERAL => {
                        let reg = self.curr_reg_map().get_new_reg();
                        if value.contains(".") {
                            let fl = self.const_map.get_float(value.to_string());
                            self.functions[self.curr_function].push_instr(HLInstr(
                                Opcode::LDF,
                                reg,
                                fl,
                                0,
                            ));
                        } else {
                            let int = self.const_map.get_int(value.parse().unwrap());
                            self.functions[self.curr_function].push_instr(HLInstr(
                                Opcode::LDI,
                                reg,
                                int,
                                0,
                            ));
                        }
                        reg
                    }
                    lua5_3_l::T_SHORT_STR => {
                        let len = value.len();
                        // make sure that the quotes are not included!
                        let short_str = self.const_map.get_str(value[1..(len - 1)].to_string());
                        self.get_const_str_reg(short_str)
                    }
                    lua5_3_l::T_NAME => {
                        // if the variable is in a register, then we can return reg number
                        // otherwise we have to generate code for `_ENV[<name>]`
                        self.curr_reg_map().get_reg(value).unwrap_or_else(|| {
                            let env_reg = self.curr_reg_map().get_reg("_ENV").unwrap();
                            let name_index = self.const_map.get_str(value.to_string());
                            let attr_reg = self.get_const_str_reg(name_index);
                            let reg = self.curr_reg_map().get_new_reg();
                            self.functions[self.curr_function].push_instr(HLInstr(
                                Opcode::GetAttr,
                                reg,
                                env_reg,
                                attr_reg,
                            ));
                            reg
                        })
                    }
                    _ => panic!(
                        "Cannot compile terminals that are not variable names, numbers or strings."
                    ),
                }
            }
        }
    }

    /// Compile an <explist> or <explistopt> and return the registers in which the
    /// result of each expression is stored.
    fn compile_exprs(&mut self, exprs: &Node<u8>) -> Vec<usize> {
        match *exprs {
            Nonterm {
                ridx: RIdx(ridx),
                ref nodes,
            } if ridx == lua5_3_y::R_EXPLIST => {
                let mut regs = vec![];
                // nodes = <exp>
                if nodes.len() == 1 {
                    regs.push(self.compile_expr(&nodes[0]));
                } else {
                    // nodes = [<explist>, <COMMA>,  <exp>]
                    regs.extend(self.compile_exprs(&nodes[0]));
                    regs.push(self.compile_expr(&nodes[2]));
                }
                regs
            }
            Nonterm {
                ridx: RIdx(ridx),
                ref nodes,
            } if ridx == lua5_3_y::R_EXPLISTOPT => {
                // nodes = <explist>
                if nodes.len() > 0 {
                    self.compile_exprs(&nodes[0])
                } else {
                    vec![]
                }
            }
            _ => panic!("Root node was not an <explist> or <explistopt>"),
        }
    }

    /// Compile a <namelist> into a vector of names.
    fn compile_namelist(&mut self, names: &Node<u8>) -> Vec<&'a str> {
        match *names {
            Nonterm {
                ridx: RIdx(ridx),
                ref nodes,
            } if ridx == lua5_3_y::R_NAMELIST => {
                let mut names = vec![];
                // nodes = <NAME>
                if nodes.len() == 1 {
                    names.push(self.compile_variable(&nodes[0]));
                } else {
                    // nodes = [<namelist>, <COMMA>, <NAME>]
                    names.extend(self.compile_namelist(&nodes[0]));
                    names.push(self.compile_variable(&nodes[2]));
                }
                names
            }
            _ => panic!("Root node is not a <namelist>"),
        }
    }

    /// Compile a <parlist> node, and assign each name a register in the current
    /// register map.
    /// The first parameter of a function is assigned to register 1, and so on.
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
                    names.extend(self.compile_namelist(&nodes[0]));
                    names.push("...");
                } else {
                    // either nodes = <...> or <parlist>
                    match nodes[0] {
                        Term { lexeme: _ } => names.push("..."),
                        _ => names.extend(self.compile_namelist(&nodes[0])),
                    }
                }
                self.functions[self.curr_function].set_param_count(names.len());
                let mut reg_map = self.curr_reg_map();
                for name in names {
                    reg_map.create_reg(name);
                }
            }
            _ => panic!("Root node was not a <parlist>"),
        }
    }

    /// Compile a <functioncall>.
    fn compile_call(&mut self, func: &Node<u8>, params: &Node<u8>) {
        let func_reg = self.compile_expr(find_term(func, lua5_3_l::T_NAME).unwrap());
        let params = match *params {
            Nonterm {
                ridx: RIdx(ridx),
                ref nodes,
            } if ridx == lua5_3_y::R_ARGS => &nodes[1],
            _ => panic!("Missing node <args> from <functioncall>"),
        };
        let compiled_exprs = self.compile_exprs(params);
        let func = &mut self.functions[self.curr_function];
        // move each expression into the paramter list of the function
        for reg in &compiled_exprs {
            func.push_instr(HLInstr(Opcode::PUSH, *reg, 0, 0));
        }
        func.push_instr(HLInstr(Opcode::CALL, func_reg, compiled_exprs.len(), 0));
    }

    /// Get the appropriate instruction for a given Node::Term.
    fn get_instr(&self, node: &Node<u8>, reg: usize, lreg: usize, rreg: usize) -> HLInstr {
        if let Term { lexeme } = node {
            let opcode = match lexeme.tok_id() {
                lua5_3_l::T_PLUS => Opcode::ADD,
                lua5_3_l::T_MINUS => Opcode::SUB,
                lua5_3_l::T_STAR => Opcode::MUL,
                lua5_3_l::T_FSLASH => Opcode::DIV,
                lua5_3_l::T_MOD => Opcode::MOD,
                lua5_3_l::T_FSFS => Opcode::FDIV,
                lua5_3_l::T_CARET => Opcode::EXP,
                _ => unimplemented!("Instruction {}", lexeme.tok_id()),
            };
            HLInstr(opcode, reg, lreg, rreg)
        } else {
            panic!("Expected a Node::Term!");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use irgen::register_map::Lifetime;
    use std::fmt::Debug;

    fn check_eq<T: Debug + PartialEq>(output: &Vec<T>, expected: &Vec<T>) {
        assert_eq!(output.len(), expected.len());
        for (lhs, rhs) in output.iter().zip(expected.iter()) {
            assert_eq!(lhs, rhs);
        }
    }

    #[test]
    fn correctness_of_ssa_ir() {
        let pt = &LuaParseTree::from_str(String::from("x = 1 + 2 * 3 / 2 ^ 2.0 // 1 - 2")).unwrap();
        let ir = compile_to_ir(pt);
        let expected_instrs = vec![
            HLInstr(Opcode::LDI, 1, 0, 0),
            HLInstr(Opcode::LDI, 2, 1, 0),
            HLInstr(Opcode::LDI, 3, 2, 0),
            HLInstr(Opcode::MUL, 4, 2, 3),
            HLInstr(Opcode::LDI, 5, 1, 0),
            HLInstr(Opcode::LDF, 6, 0, 0),
            HLInstr(Opcode::EXP, 7, 5, 6),
            HLInstr(Opcode::DIV, 8, 4, 7),
            HLInstr(Opcode::LDI, 9, 0, 0),
            HLInstr(Opcode::FDIV, 10, 8, 9),
            HLInstr(Opcode::ADD, 11, 1, 10),
            HLInstr(Opcode::LDI, 12, 1, 0),
            HLInstr(Opcode::SUB, 13, 11, 12),
            HLInstr(Opcode::LDS, 14, 0, 0),
            HLInstr(Opcode::SetAttr, 0, 14, 13),
        ];
        let function = &ir.functions[0];
        check_eq(function.instrs(), &expected_instrs);
        // check that the IR is in SSA form
        let mut regs = Vec::with_capacity(function.instrs().len());
        regs.resize(function.instrs().len(), false);
        for i in function.instrs() {
            regs[i.1] = !regs[i.1];
            // if at any point this assertion fails, it means that a register has been
            // assigned a value multiple times
            // SetAttr only updates the state of a register, so it doesn't mess up the
            // correctness of the SSA
            if i.0 != Opcode::SetAttr {
                assert!(regs[i.1]);
            }
        }
        // check lifetimes
        let expected_lifetimes = vec![
            Lifetime::with_end_point(0, 15),
            Lifetime::with_end_point(1, 2),
            Lifetime::with_end_point(2, 3),
            Lifetime::with_end_point(3, 4),
            Lifetime::with_end_point(4, 5),
            Lifetime::with_end_point(5, 6),
            Lifetime::with_end_point(6, 7),
            Lifetime::with_end_point(7, 8),
            Lifetime::with_end_point(8, 9),
            Lifetime::with_end_point(9, 10),
            Lifetime::with_end_point(10, 11),
            Lifetime::with_end_point(11, 12),
            Lifetime::with_end_point(12, 13),
            Lifetime::with_end_point(13, 14),
            Lifetime::with_end_point(14, 15),
        ];
        check_eq(function.lifetimes(), &expected_lifetimes);
        // check constats map
        let expected_ints = vec![1, 2, 3];
        let ints = ir.const_map.get_ints();
        check_eq(&ints, &expected_ints);
        let expected_floats = vec![2.0];
        let floats = ir.const_map.get_floats();
        check_eq(&floats, &expected_floats);
        let expected_strings = vec!["x".to_string()];
        let strings = ir.const_map.get_strings();
        check_eq(&strings, &expected_strings);
    }

    #[test]
    fn correctness_of_ssa_ir2() {
        let pt = &LuaParseTree::from_str(String::from(
            "x = 1
             y = x",
        ))
        .unwrap();
        let ir = compile_to_ir(pt);
        let expected_instrs = vec![
            HLInstr(Opcode::LDI, 1, 0, 0),     // R(1) = INT(0) == 1
            HLInstr(Opcode::LDS, 2, 0, 0),     // R(2) = STR(0) == "x"
            HLInstr(Opcode::SetAttr, 0, 2, 1), // _ENV["x"] = R(1); x = 1
            HLInstr(Opcode::GetAttr, 3, 0, 2), // R(3) = _ENV["x"]
            HLInstr(Opcode::LDS, 4, 1, 0),     // R(4) = STR(1) == "y"
            HLInstr(Opcode::SetAttr, 0, 4, 3), // _ENV["y"] = R(4); y = x
        ];
        let function = &ir.functions[ir.main_func];
        check_eq(function.instrs(), &expected_instrs);
        // check that the IR is in SSA form
        let mut regs = Vec::with_capacity(function.instrs().len());
        regs.resize(function.instrs().len(), false);
        for i in function.instrs() {
            regs[i.1] = !regs[i.1];
            // if at any point this assertion fails, it means that a register has been
            // assigned a value multiple times
            if i.0 != Opcode::SetAttr {
                assert!(regs[i.1]);
            }
        }
        // check lifetimes
        let expected_lifetimes = vec![
            Lifetime::with_end_point(0, 5),
            Lifetime::with_end_point(1, 2),
            Lifetime::with_end_point(2, 4),
            Lifetime::with_end_point(3, 4),
            Lifetime::with_end_point(4, 5),
        ];
        check_eq(function.lifetimes(), &expected_lifetimes);
        // check constats map
        let expected_ints = vec![1];
        let ints = ir.const_map.get_ints();
        check_eq(&ints, &expected_ints);
        assert!(ir.const_map.get_floats().is_empty());
        let expected_strings = vec!["x".to_string(), "y".to_string()];
        let strings = ir.const_map.get_strings();
        check_eq(&strings, &expected_strings);
    }

    #[test]
    fn generates_get_attr_instr() {
        let pt = &LuaParseTree::from_str(String::from("x = y")).unwrap();
        let ir = compile_to_ir(pt);
        let expected_instrs = vec![
            HLInstr(Opcode::LDS, 1, 0, 0),
            HLInstr(Opcode::GetAttr, 2, 0, 1),
            HLInstr(Opcode::LDS, 3, 1, 0),
            HLInstr(Opcode::SetAttr, 0, 3, 2),
        ];
        let function = &ir.functions[0];
        check_eq(function.instrs(), &expected_instrs);
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
            HLInstr(Opcode::LDI, 1, 0, 0),
            HLInstr(Opcode::LDS, 2, 0, 0),
            HLInstr(Opcode::SetAttr, 0, 2, 1),
        ];
        let function = &ir.functions[0];
        check_eq(function.instrs(), &expected_instrs);
    }

    #[test]
    fn load_string_multiple_times() {
        let pt = &LuaParseTree::from_str(String::from(
            "local x = \"1\"
             local y = \"1\"",
        ))
        .unwrap();
        let ir = compile_to_ir(pt);
        let expected_instrs = vec![HLInstr(Opcode::LDS, 1, 0, 0), HLInstr(Opcode::MOV, 2, 1, 0)];
        let function = &ir.functions[ir.main_func];
        check_eq(function.instrs(), &expected_instrs);
        let pt = &LuaParseTree::from_str(String::from(
            "x = \"1\"
             y = \"x\"",
        ))
        .unwrap();
        let ir = compile_to_ir(pt);
        let expected_instrs = vec![
            HLInstr(Opcode::LDS, 1, 0, 0),     // R(1) = "1"
            HLInstr(Opcode::LDS, 2, 1, 0),     // R(2) = "x"
            HLInstr(Opcode::SetAttr, 0, 2, 1), // _ENV["x"] = "1"
            HLInstr(Opcode::LDS, 3, 2, 0),     // R(3) = "y"
            // notice that it is reusing register 2!
            HLInstr(Opcode::SetAttr, 0, 3, 2), // _ENV["y"] = "x"
        ];
        let function = &ir.functions[ir.main_func];
        check_eq(function.instrs(), &expected_instrs);
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
                HLInstr(Opcode::CLOSURE, 1, 0, 0),
                HLInstr(Opcode::LDS, 2, 1, 0),
                HLInstr(Opcode::SetAttr, 0, 2, 1),
            ],
            vec![
                HLInstr(Opcode::LDI, 1, 0, 0),
                HLInstr(Opcode::LDS, 2, 0, 0),
                HLInstr(Opcode::SetAttr, 0, 2, 1),
            ],
        ];
        for i in 0..ir.functions.len() {
            check_eq(ir.functions[i].instrs(), &expected_instrs[i])
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
                HLInstr(Opcode::CLOSURE, 1, 0, 0),
                HLInstr(Opcode::LDS, 2, 1, 0),
                HLInstr(Opcode::SetAttr, 0, 2, 1),
                HLInstr(Opcode::GetAttr, 3, 0, 2),
                HLInstr(Opcode::CALL, 3, 0, 0),
                HLInstr(Opcode::GetAttr, 4, 0, 2),
                HLInstr(Opcode::CALL, 4, 0, 0),
            ],
            vec![
                HLInstr(Opcode::LDI, 1, 0, 0),
                HLInstr(Opcode::LDS, 2, 0, 0),
                HLInstr(Opcode::SetAttr, 0, 2, 1),
            ],
        ];
        for i in 0..ir.functions.len() {
            check_eq(ir.functions[i].instrs(), &expected_instrs[i])
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
                HLInstr(Opcode::CLOSURE, 1, 0, 0),
                HLInstr(Opcode::LDS, 2, 1, 0),
                HLInstr(Opcode::SetAttr, 0, 2, 1),
                HLInstr(Opcode::GetAttr, 3, 0, 2),
                HLInstr(Opcode::LDI, 4, 0, 0),
                HLInstr(Opcode::PUSH, 4, 0, 0),
                HLInstr(Opcode::CALL, 3, 1, 0),
                HLInstr(Opcode::GetAttr, 5, 0, 2),
                HLInstr(Opcode::LDS, 6, 0, 0),
                HLInstr(Opcode::GetAttr, 7, 0, 6),
                HLInstr(Opcode::PUSH, 7, 0, 0),
                HLInstr(Opcode::CALL, 5, 1, 0),
            ],
            vec![
                HLInstr(Opcode::LDS, 2, 0, 0),
                HLInstr(Opcode::SetAttr, 0, 2, 1),
            ],
        ];
        for i in 0..ir.functions.len() {
            check_eq(ir.functions[i].instrs(), &expected_instrs[i])
        }
    }
}
