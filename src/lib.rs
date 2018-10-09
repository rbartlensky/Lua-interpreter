extern crate cfgrammar;
#[macro_use] extern crate lrlex;
#[macro_use] extern crate lrpar;
extern crate lrtable;

mod bytecode;
mod errors;
mod register_map;
pub mod interpreter;

use cfgrammar::RIdx;
use lrpar::Node;
use lrpar::Node::*;
use std::fs::File;
use std::io::prelude::*;
use register_map::RegisterMap;
use bytecode::LuaBytecode;
use bytecode::instructions::Val::*;
use bytecode::instructions::*;
use errors::CliError;

lrlex_mod!(lua5_3_l); // lua lexer
lrpar_mod!(lua5_3_y); // lua parser

/// Holds the parse tree of a Lua file.
pub struct LuaParseTree {
    /// The original Lua code
    pub original_code: String,
    /// The root of the parse tree
    pub tree: Node<u8>
}

impl LuaParseTree {
    /// Create a new LuaParseTree out of the contents found in <file>.
    pub fn new(file: &str) -> Result<LuaParseTree, CliError> {
        // read contents of the file
        let mut file = File::open(file).map_err(CliError::Io)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(CliError::Io)?;

        // try to parse the contents
        let lexerdef = lua5_3_l::lexerdef();
        let mut lexer = lexerdef.lexer(&contents);
        let tree = lua5_3_y::parse(&mut lexer)?;

        Ok(LuaParseTree {
            original_code: contents.clone(),
            tree
        })
    }

    /// Compile the parse tree to an intermmediate representation.
    pub fn compile_to_ir(&self) -> LuaBytecode {
        let mut instrs = vec![];
        let mut pt_nodes: Vec<&Node<u8>> = vec![&self.tree];
        let mut reg_map = RegisterMap::new();
        while !pt_nodes.is_empty() {
            let node = pt_nodes.pop().unwrap(); // always checked if it is empty
            match *node {
                Nonterm{ridx: RIdx(ridx), ref nodes} if ridx == lua5_3_y::R_STAT => {
                    debug_assert!(nodes.len() == 3);
                    match nodes[1] {
                        Term{lexeme} if lexeme.tok_id() == lua5_3_l::T_EQ => {
                            let id = self.compile_variable(&nodes[0]);
                            let value = self.compile_expr(&nodes[2], &mut instrs, &mut reg_map);
                            let reg = reg_map.get_reg(&id);
                            instrs.push(Instr::Mov(reg, value));
                        },
                        _ => {}
                    }
                },
                Nonterm{ridx: _, ref nodes} => {
                    for i in (0..nodes.len()).rev() {
                        pt_nodes.push(&nodes[i]);
                    }
                },
                _ => { continue; }
            }
        }
        LuaBytecode::new(instrs, reg_map.get_registers())
    }

    /// Jumps to the first child of <node> which denotes a variable name.
    fn compile_variable<'a>(&'a self, node: &Node<u8>) -> &'a str {
        let name = LuaParseTree::find_term(node, lua5_3_l::T_NAME);
        match name {
            Some(Term{lexeme}) =>
                self.get_string(lexeme.start(), lexeme.end()),
            _ => { panic!("Must have assignments of form: var = expr!"); }
        }
    }

    /// Compile the expression rooted at <node>. Any instructions that are created are
    /// simply added to the bytecode that is being generated.
    fn compile_expr(&self, node: &Node<u8>, instrs: &mut Vec<Instr>,
                    reg_map: &mut RegisterMap) -> Val {
        match *node {
            Nonterm{ridx: RIdx(_ridx), ref nodes} => {
                if nodes.len() == 1 {
                    self.compile_expr(&nodes[0], instrs, reg_map)
                } else {
                    assert!(nodes.len() == 3);
                    let left = self.compile_expr(&nodes[0], instrs, reg_map);
                    let right = self.compile_expr(&nodes[2], instrs, reg_map);
                    let new_var = reg_map.new_reg();
                    instrs.push(self.get_instr(&nodes[1], Reg::new(new_var), left, right));
                    Register(new_var)
                }
            },
            Term{lexeme} => {
                let value = self.get_string(lexeme.start(), lexeme.end());
                if lexeme.tok_id() == lua5_3_l::T_NUMERAL {
                    LuaValue(Value::Number(value.parse().unwrap()))
                } else {
                    let reg = reg_map.get_reg(value);
                    Register(reg)
                }
            }
        }
    }

    /// Get the appropriate instruction for a given Node::Term.
    fn get_instr(&self, node: &Node<u8>, reg: Reg, lhs: Val, rhs: Val) -> Instr {
        if let Term{lexeme} = node {
            match lexeme.tok_id() {
                lua5_3_l::T_PLUS => Instr::Add(reg.id(), lhs, rhs),
                lua5_3_l::T_MINUS => Instr::Sub(reg.id(), lhs, rhs),
                lua5_3_l::T_STAR => Instr::Mul(reg.id(), lhs, rhs),
                lua5_3_l::T_FSLASH => Instr::Div(reg.id(), lhs, rhs),
                lua5_3_l::T_MOD => Instr::Mod(reg.id(), lhs, rhs),
                _ => unimplemented!("Instruction {}", lexeme.tok_id())
            }
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
                Nonterm{ridx: _, ref nodes} => {
                    for ref node in nodes {
                        pt_nodes.push(node);
                    }
                },
                Term{lexeme} => {
                    if lexeme.tok_id() == id {
                        return Some(node);
                    } else {
                        // continue the dfs
                        continue;
                    }
                }
            }
        }
        None
    }

    /// Get a slice from the original file.
    fn get_string(&self, start: usize, end: usize) -> &str {
        &self.original_code[start..end]
    }
}
