extern crate clap;
extern crate luacompiler;
extern crate luavm;

use clap::{App, Arg};
use luacompiler::{bytecodegen::compile_to_bytecode, irgen::compile_to_ir, LuaParseTree};
use luavm::Vm;

fn main() {
    let matches = App::new("Lua interpreter")
        .version("0.1")
        .author("Robert Bartlensky")
        .about("Interpret Lua files")
        .arg(
            Arg::with_name("bytecode")
                .long("bytecode")
                .help("Print the bytecode produced by the compiler."),
        )
        .arg(
            Arg::with_name("INPUT")
                .help("File to interpret")
                .required(true)
                .index(1)
                .min_values(1)
                .max_values(u64::max_value()),
        )
        .get_matches();
    // we can safely unwrap because INPUT is not an optional argument
    let mut script_args = matches.values_of("INPUT").unwrap();
    let file = script_args.nth(0).unwrap();
    let parse_tree = LuaParseTree::new(&file);
    match parse_tree {
        Ok(pt) => {
            let bc = compile_to_bytecode(compile_to_ir(&pt));
            if matches.is_present("bytecode") {
                println!("{}", &bc);
            }
            let mut all_args: Vec<&str> = vec![file];
            let script_args: Vec<&str> = script_args.map(|v| v).collect();
            all_args.extend(script_args);
            let mut vm = Vm::new(bc, all_args);
            vm.eval().unwrap();
        }
        Err(err) => println!("{:#?}", err),
    }
}
