extern crate clap;
extern crate luacompiler;
extern crate luavm;

use clap::{App, Arg};
use luacompiler::{irgen::LuaToBytecode, LuaParseTree};
use luavm::Vm;

fn main() {
    let matches = App::new("Lua interpreter")
        .version("0.1")
        .author("Robert Bartlensky")
        .about("Interpret Lua files")
        .arg(
            Arg::with_name("INPUT")
                .help("File to interpret")
                .required(true)
                .index(1),
        )
        .get_matches();
    // we can safely unwrap because INPUT is not an optional argument
    let file = matches.value_of("INPUT").unwrap();
    let parse_tree = LuaParseTree::new(&file);
    match parse_tree {
        Ok(pt) => {
            let bc = LuaToBytecode::new(&pt).compile();
            let mut vm = Vm::new(bc);
            vm.eval();
        }
        Err(err) => println!("{:#?}", err),
    }
}
