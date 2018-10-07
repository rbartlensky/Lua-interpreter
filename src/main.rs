extern crate clap;
extern crate lrpar;
extern crate lua_interp;

use clap::{Arg, App};
use lua_interp::LuaParseTree;

fn main() {
    let matches = App::new("Lua interpreter")
        .version("0.1")
        .author("Robert Bartlensky")
        .about("Semi-interpret Lua files")
        .arg(Arg::with_name("INPUT")
             .help("File to interpret")
             .required(true)
             .index(1))
        .get_matches();
    // safe because INPUT is required
    let file = matches.value_of("INPUT").unwrap();
    let parse_tree = LuaParseTree::new(file);
    match parse_tree {
        Ok(pt) => {
            let instrs = pt.compile_to_ir();
            for ins in instrs.get_instrs()  {
                println!("{}", ins);
            }
        },
        Err(err) => println!("{:#?}", err)
    }
}
