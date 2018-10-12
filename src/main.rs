extern crate clap;
extern crate lrpar;
extern crate lua_interp;

use clap::{Arg, App};
use lua_interp::LuaParseTree;
use lua_interp::interpreter::Interpreter;

fn main() {
    let matches = App::new("Lua interpreter")
        .version("0.1")
        .author("Robert Bartlensky")
        .about("Interpret Lua files")
        .arg(Arg::with_name("INPUT")
             .help("File to interpret")
             .required(true)
             .index(1))
        .get_matches();
    // we can safely unwrap because INPUT is not an optional argument
    let file = matches.value_of("INPUT").unwrap();
    let parse_tree = LuaParseTree::new(file);
    match parse_tree {
        Ok(pt) => {
            let instrs = pt.compile_to_ir();
            let mut interp = Interpreter::new(instrs);
            interp.eval();

        },
        Err(err) => println!("{:#?}", err)
    }
}
