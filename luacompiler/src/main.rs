extern crate clap;
extern crate lrpar;
extern crate luacompiler;

use clap::{App, Arg};
use luacompiler::{irgen::LuaToBytecode, LuaParseTree};
use std::path::PathBuf;

fn main() {
    let matches = App::new("Lua compiler")
        .version("0.1")
        .author("Robert Bartlensky")
        .about("Compile Lua files to IR")
        .arg(
            Arg::with_name("INPUT")
                .help("File to compile")
                .required(true)
                .index(1),
        )
        .get_matches();
    // we can safely unwrap because INPUT is not an optional argument
    let file = matches.value_of("INPUT").unwrap();
    let parse_tree = LuaParseTree::new(&file);
    match parse_tree {
        Ok(pt) => {
            let bc = LuaToBytecode::new(&pt).compile_to_ir();
            // create a luabc file next to the input file
            let mut path = PathBuf::from(file);
            path.set_extension("luabc");
            bc.serialize_to_file(path.to_str().unwrap()).unwrap();
        }
        Err(err) => println!("{:#?}", err),
    }
}
