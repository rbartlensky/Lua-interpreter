pub mod constants_map;
pub mod register_map;
use self::{constants_map::ConstantsMap, register_map::RegisterMap};
use bytecode::{
    instructions::{make_instr, Opcode},
    LuaBytecode,
};
use cfgrammar::RIdx;
use lrpar::Node::{self, *};
use lua5_3_l;
use lua5_3_y;
use LuaParseTree;

/// Represents a compiler which translates a given Lua parse tree to some bytecode representation.
/// This compiler will be changed in the future to translate from lua to a higher-level
/// representation, which is easier to translate to the current bytecode.
pub struct LuaToBytecode<'a> {
    pt: &'a LuaParseTree,
    reg_map: RegisterMap,
    const_map: ConstantsMap,
    instrs: Vec<u32>,
}

impl<'a> LuaToBytecode<'a> {
    pub fn new(pt: &'a LuaParseTree) -> LuaToBytecode {
        LuaToBytecode {
            pt,
            reg_map: RegisterMap::new(),
            const_map: ConstantsMap::new(),
            instrs: vec![],
        }
    }

    /// Compile the parse tree to an intermediate representation.
    pub fn compile(mut self) -> LuaBytecode {
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
                            let reg = self.reg_map.get_reg(self.compile_variable(&nodes[0]));
                            self.instrs.push(make_instr(Opcode::MOV, reg, value, 0));
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
        LuaBytecode::new(self.instrs, self.const_map, self.reg_map.reg_count())
    }

    /// Jumps to the first child of <node> which denotes a variable name.
    fn compile_variable(&'a self, node: &Node<u8>) -> &'a str {
        let name = LuaToBytecode::find_term(node, lua5_3_l::T_NAME);
        match name {
            Some(Term { lexeme }) => self.pt.get_string(lexeme.start(), lexeme.end()),
            _ => {
                panic!("Must have assignments of form: var = expr!");
            }
        }
    }

    /// Compile the expression rooted at <node>. Any instructions that are created are
    /// simply added to the bytecode that is being generated.
    fn compile_expr(&mut self, node: &Node<u8>) -> u8 {
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
                    let new_var = self.reg_map.new_reg();
                    let instr = self.get_instr(&nodes[1], new_var, left, right);
                    self.instrs.push(instr);
                    new_var
                }
            }
            Term { lexeme } => {
                let value = self.pt.get_string(lexeme.start(), lexeme.end());
                match lexeme.tok_id() {
                    lua5_3_l::T_NUMERAL => {
                        let reg = self.reg_map.new_reg();
                        if value.contains(".") {
                            let fl = self.const_map.get_float(value.to_string());
                            self.instrs.push(make_instr(Opcode::LDF, reg, fl, 0));
                        } else {
                            let int = self.const_map.get_int(value.parse().unwrap());
                            self.instrs.push(make_instr(Opcode::LDI, reg, int, 0));
                        }
                        reg
                    }
                    lua5_3_l::T_SHORT_STR => {
                        let reg = self.reg_map.new_reg();
                        let len = value.len();
                        // make sure that the quotes are not included!
                        let short_str = self.const_map.get_str(value[1..(len - 1)].to_string());
                        self.instrs.push(make_instr(Opcode::LDS, reg, short_str, 0));
                        reg
                    }
                    _ => self.reg_map.get_reg(value),
                }
            }
        }
    }

    /// Get the appropriate instruction for a given Node::Term.
    fn get_instr(&self, node: &Node<u8>, reg: u8, lreg: u8, rreg: u8) -> u32 {
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
            make_instr(opcode, reg, lreg, rreg)
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
