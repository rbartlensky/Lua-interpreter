extern crate cfgrammar;
#[macro_use]
extern crate lrlex;
#[macro_use]
extern crate lrpar;
extern crate lrtable;
#[macro_use]
extern crate serde_derive;
extern crate bincode;

pub mod bytecode;
pub mod bytecodegen;
pub mod errors;
pub mod irgen;

use errors::CliError;
use lrpar::Node;
use std::{fs::File, io::prelude::*};

lrlex_mod!(lua5_3_l); // lua lexer
lrpar_mod!(lua5_3_y); // lua parser

/// Holds the parse tree of a Lua file.
pub struct LuaParseTree {
    /// The original Lua code
    pub contents: String,
    /// The root of the parse tree
    pub tree: Node<u8>,
}

impl LuaParseTree {
    /// Create a new LuaParseTree out of the contents found in <file>.
    pub fn new(file: &str) -> Result<LuaParseTree, CliError> {
        let mut pt = LuaParseTree {
            contents: String::new(),
            tree: Node::Nonterm {
                ridx: cfgrammar::RIdx(0),
                nodes: vec![],
            },
        };
        // read contents of the file
        let mut file = File::open(file).map_err(CliError::Io)?;
        file.read_to_string(&mut pt.contents)
            .map_err(CliError::Io)?;

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
            tree: Node::Nonterm {
                ridx: cfgrammar::RIdx(0),
                nodes: vec![],
            },
        };
        {
            let lexerdef = lua5_3_l::lexerdef();
            let mut lexer = lexerdef.lexer(&mut pt.contents);
            let tree = lua5_3_y::parse(&mut lexer)?;
            pt.tree = tree;
        }
        Ok(pt)
    }

    /// Get a slice from the original file.
    fn get_string(&self, start: usize, end: usize) -> &str {
        &self.contents[start..end]
    }
}
