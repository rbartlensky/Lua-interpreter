extern crate cfgrammar;
#[macro_use] extern crate lrlex;
#[macro_use] extern crate lrpar;
extern crate lrtable;
#[macro_use] extern crate serde_derive;
extern crate bincode;

pub mod bytecode;
pub mod errors;
mod register_map;
mod constants_map;

use cfgrammar::RIdx;
use lrpar::Node;
use lrpar::Node::*;
use std::fs::File;
use std::io::prelude::*;
use register_map::RegisterMap;
use bytecode::LuaBytecode;
use bytecode::instructions::{ Opcode, make_instr };
use constants_map::ConstantsMap;
use errors::CliError;

lrlex_mod!(lua5_3_l); // lua lexer
lrpar_mod!(lua5_3_y); // lua parser

/// Holds the parse tree of a Lua file.
pub struct LuaParseTree {
    /// The original Lua code
    pub contents: String,
    /// The root of the parse tree
    pub tree: Node<u8>
}

impl LuaParseTree {
    /// Create a new LuaParseTree out of the contents found in <file>.
    pub fn new(file: &str) -> Result<LuaParseTree, CliError> {
        let mut pt = LuaParseTree {
            contents: String::new(),
            tree: Node::Nonterm {ridx: cfgrammar::RIdx(0), nodes: vec![]}
        };
        // read contents of the file
        let mut file = File::open(file).map_err(CliError::Io)?;
        file.read_to_string(&mut pt.contents).map_err(CliError::Io)?;

        // try to parse the contents
        {
            let lexerdef = lua5_3_l::lexerdef();
            let mut lexer = lexerdef.lexer(&mut pt.contents);
            let tree = lua5_3_y::parse(&mut lexer)?;
            pt.tree = tree;
        }
        Ok(pt)
    }

    /// Create a new LuaParseTree from the given string.
    pub fn from_str(code: String) -> Result<LuaParseTree, CliError> {
        let mut pt = LuaParseTree {
            contents: code,
            tree: Node::Nonterm {ridx: cfgrammar::RIdx(0), nodes: vec![]}
        };
        {
            let lexerdef = lua5_3_l::lexerdef();
            let mut lexer = lexerdef.lexer(&mut pt.contents);
            let tree = lua5_3_y::parse(&mut lexer)?;
            pt.tree = tree;
        }
        Ok(pt)
    }

    /// Compile the parse tree to an intermmediate representation.
    pub fn compile_to_ir(&self) -> LuaBytecode {
        let mut instrs = vec![];
        let mut pt_nodes: Vec<&Node<u8>> = vec![&self.tree];
        let mut reg_map = RegisterMap::new();
        let mut const_map = ConstantsMap::new();
        while !pt_nodes.is_empty() {
            let node = pt_nodes.pop().unwrap(); // always checked if it is empty
            match *node {
                Nonterm{ridx: RIdx(ridx), ref nodes} if ridx == lua5_3_y::R_STAT => {
                    debug_assert!(nodes.len() == 3);
                    match nodes[1] {
                        Term{lexeme} if lexeme.tok_id() == lua5_3_l::T_EQ => {
                            let id = self.compile_variable(&nodes[0]);
                            let value = self.compile_expr(&nodes[2], &mut instrs,
                                                          &mut reg_map, &mut const_map);
                            let reg = reg_map.get_reg(&id);
                            instrs.push(make_instr(Opcode::MOV, reg, value, 0));
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
        LuaBytecode::new(instrs, const_map, reg_map.reg_count())
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
    fn compile_expr(&self, node: &Node<u8>, instrs: &mut Vec<u32>,
                    reg_map: &mut RegisterMap, const_map: &mut ConstantsMap) -> u8 {
        match *node {
            Nonterm{ridx: RIdx(_ridx), ref nodes} => {
                if nodes.len() == 1 {
                    self.compile_expr(&nodes[0], instrs, reg_map, const_map)
                } else {
                    assert!(nodes.len() == 3);
                    let left = self.compile_expr(&nodes[0], instrs, reg_map, const_map);
                    let right = self.compile_expr(&nodes[2], instrs, reg_map, const_map);
                    let new_var = reg_map.new_reg();
                    instrs.push(self.get_instr(&nodes[1], new_var, left, right));
                    new_var
                }
            },
            Term{lexeme} => {
                let value = self.get_string(lexeme.start(), lexeme.end());
                match lexeme.tok_id() {
                    lua5_3_l::T_NUMERAL => {
                        let reg = reg_map.new_reg();
                        if value.contains(".") {
                            let fl = const_map.get_float(value.to_string());
                            instrs.push(make_instr(Opcode::LDF, reg, fl, 0));
                        } else {
                            let int = const_map.get_int(value.parse().unwrap());
                            instrs.push(make_instr(Opcode::LDI, reg, int, 0));
                        }
                        reg
                    },
                    lua5_3_l::T_SHORT_STR => {
                        let reg = reg_map.new_reg();
                        let len = value.len();
                        // make sure that the quotes are not included!
                        let short_str = const_map.get_str(value[1..(len-1)].to_string());
                        instrs.push(make_instr(Opcode::LDS, reg, short_str, 0));
                        reg
                    }
                    _ => reg_map.get_reg(value)
                }
            }
        }
    }

    /// Get the appropriate instruction for a given Node::Term.
    fn get_instr(&self, node: &Node<u8>, reg: u8, lreg: u8, rreg: u8) -> u32 {
        if let Term{lexeme} = node {
            let opcode = match lexeme.tok_id() {
                lua5_3_l::T_PLUS => Opcode::ADD,
                lua5_3_l::T_MINUS => Opcode::SUB,
                lua5_3_l::T_STAR => Opcode::MUL,
                lua5_3_l::T_FSLASH => Opcode::DIV,
                lua5_3_l::T_MOD => Opcode::MOD,
                lua5_3_l::T_FSFS => Opcode::FDIV,
                lua5_3_l::T_CARET => Opcode::EXP,
                _ => unimplemented!("Instruction {}", lexeme.tok_id())
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
        &self.contents[start..end]
    }
}
