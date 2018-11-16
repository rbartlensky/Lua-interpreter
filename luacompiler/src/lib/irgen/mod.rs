pub mod constants_map;
pub mod lua_ir;
pub mod register_map;

use self::{constants_map::ConstantsMap, lua_ir::LuaIR, register_map::RegisterMap};
use bytecode::instructions::{HLInstr, Opcode};
use cfgrammar::RIdx;
use lrpar::Node::{self, *};
use lua5_3_l;
use lua5_3_y;
use LuaParseTree;

/// Compile the given parse tree into an SSA IR.
pub fn compile_to_ir(pt: &LuaParseTree) -> LuaIR {
    LuaToIR::new(pt).compile()
}

/// Represents a compiler which translates a given Lua parse tree to an SSA IR.
struct LuaToIR<'a> {
    pt: &'a LuaParseTree,
    reg_map: RegisterMap<'a>,
    const_map: ConstantsMap,
    instrs: Vec<HLInstr>,
}

impl<'a> LuaToIR<'a> {
    fn new(pt: &'a LuaParseTree) -> LuaToIR {
        LuaToIR {
            pt,
            reg_map: RegisterMap::new(),
            const_map: ConstantsMap::new(),
            instrs: vec![],
        }
    }

    /// Compile the parse tree to an intermediate representation.
    pub fn compile(mut self) -> LuaIR {
        let mut pt_nodes = vec![&self.pt.tree];
        while !pt_nodes.is_empty() {
            let node = pt_nodes.pop().unwrap();
            match *node {
                Nonterm {
                    ridx: RIdx(ridx),
                    ref nodes,
                } if ridx == lua5_3_y::R_STAT => {
                    debug_assert!(nodes.len() == 3);
                    match nodes[1] {
                        Term { lexeme } if lexeme.tok_id() == lua5_3_l::T_EQ => {
                            let value = self.compile_expr(&nodes[2]);
                            let name = self.compile_variable(&nodes[0]);
                            // because we are creating an IR which is in SSA form, it
                            // means that each assignment creates a new register
                            let reg = self.reg_map.get_new_reg();
                            // if a variable is assigned a value multiple times, we have
                            // to make sure that the map knows the new register which
                            // holds the new value
                            self.reg_map.set_reg(name, reg);
                            self.instrs.push(HLInstr(Opcode::MOV, reg, value, 0));
                        }
                        _ => {}
                    }
                }
                Nonterm { ridx: _, ref nodes } => {
                    for i in (0..nodes.len()).rev() {
                        pt_nodes.push(&nodes[i]);
                    }
                }
                _ => {
                    continue;
                }
            }
        }
        LuaIR::new(self.instrs, self.const_map, self.reg_map.get_lifetimes())
    }

    /// Jumps to the first child of <node> which denotes a variable name.
    fn compile_variable(&self, node: &Node<u8>) -> &'a str {
        let name = LuaToIR::find_term(node, lua5_3_l::T_NAME);
        match name {
            Some(Term { lexeme }) => self.pt.get_string(lexeme.start(), lexeme.end()),
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
                ridx: RIdx(_ridx),
                ref nodes,
            } => {
                if nodes.len() == 1 {
                    self.compile_expr(&nodes[0])
                } else {
                    debug_assert!(nodes.len() == 3);
                    let left = self.compile_expr(&nodes[0]);
                    let right = self.compile_expr(&nodes[2]);
                    let new_var = self.reg_map.get_new_reg();
                    let instr = self.get_instr(&nodes[1], new_var, left, right);
                    self.instrs.push(instr);
                    new_var
                }
            }
            Term { lexeme } => {
                let value = self.pt.get_string(lexeme.start(), lexeme.end());
                match lexeme.tok_id() {
                    lua5_3_l::T_NUMERAL => {
                        let reg = self.reg_map.get_new_reg();
                        if value.contains(".") {
                            let fl = self.const_map.get_float(value.to_string());
                            self.instrs.push(HLInstr(Opcode::LDF, reg, fl, 0));
                        } else {
                            let int = self.const_map.get_int(value.parse().unwrap());
                            self.instrs.push(HLInstr(Opcode::LDI, reg, int, 0));
                        }
                        reg
                    }
                    lua5_3_l::T_SHORT_STR => {
                        let reg = self.reg_map.get_new_reg();
                        let len = value.len();
                        // make sure that the quotes are not included!
                        let short_str = self.const_map.get_str(value[1..(len - 1)].to_string());
                        self.instrs.push(HLInstr(Opcode::LDS, reg, short_str, 0));
                        reg
                    }
                    _ => self.reg_map.get_reg(value),
                }
            }
        }
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

    /// Find the first Node::Term with the given id.
    fn find_term(start: &Node<u8>, id: u8) -> Option<&Node<u8>> {
        let mut pt_nodes: Vec<&Node<u8>> = vec![start];
        while !pt_nodes.is_empty() {
            let node = pt_nodes.pop().unwrap(); // always checked if it is empty
            match node {
                Nonterm { ridx: _, ref nodes } => {
                    for ref node in nodes {
                        pt_nodes.push(node);
                    }
                }
                Term { lexeme } => {
                    if lexeme.tok_id() == id {
                        return Some(node);
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use irgen::register_map::Lifetime;

    #[test]
    fn correctness_of_ssa_ir() {
        let pt = LuaParseTree::from_str(String::from("x = 1 + 2 * 3 / 2 ^ 2.0 // 1 - 2"));
        let ir = compile_to_ir(&pt.unwrap());
        let expected_instrs = vec![
            HLInstr(Opcode::LDI, 0, 0, 0),
            HLInstr(Opcode::LDI, 1, 1, 0),
            HLInstr(Opcode::LDI, 2, 2, 0),
            HLInstr(Opcode::MUL, 3, 1, 2),
            HLInstr(Opcode::LDI, 4, 1, 0),
            HLInstr(Opcode::LDF, 5, 0, 0),
            HLInstr(Opcode::EXP, 6, 4, 5),
            HLInstr(Opcode::DIV, 7, 3, 6),
            HLInstr(Opcode::LDI, 8, 0, 0),
            HLInstr(Opcode::FDIV, 9, 7, 8),
            HLInstr(Opcode::ADD, 10, 0, 9),
            HLInstr(Opcode::LDI, 11, 1, 0),
            HLInstr(Opcode::SUB, 12, 10, 11),
            HLInstr(Opcode::MOV, 13, 12, 0),
        ];
        assert_eq!(ir.instrs.len(), expected_instrs.len());
        for (lhs, rhs) in ir.instrs.iter().zip(expected_instrs.iter()) {
            assert_eq!(lhs, rhs);
        }
        // check that the IR is in SSA form
        let mut regs = Vec::with_capacity(ir.instrs.len());
        regs.resize(ir.instrs.len(), false);
        for i in &ir.instrs {
            regs[i.1] = !regs[i.1];
            // if at any point this assertion fails, it means that a register has been
            // assigned a value multiple times
            assert!(regs[i.1]);
        }
        // check lifetimes
        let expected_lifetimes = vec![
            Lifetime::with_end_point(0, 1),
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
        ];
        assert_eq!(ir.lifetimes.len(), expected_lifetimes.len());
        for (lhs, rhs) in ir.lifetimes.iter().zip(expected_lifetimes.iter()) {
            assert_eq!(lhs, rhs);
        }
        // check constats map
        let expected_ints = vec![1, 2, 3];
        let ints = ir.const_map.get_ints();
        assert_eq!(ints.len(), expected_ints.len());
        for (lhs, rhs) in ints.iter().zip(expected_ints.iter()) {
            assert_eq!(lhs, rhs);
        }
        let expected_floats = vec![2.0];
        let floats = ir.const_map.get_floats();
        assert_eq!(floats.len(), expected_floats.len());
        for (lhs, rhs) in floats.iter().zip(expected_floats.iter()) {
            assert_eq!(lhs, rhs);
        }
        assert_eq!(ir.const_map.get_strings().len(), 0);
    }
}
