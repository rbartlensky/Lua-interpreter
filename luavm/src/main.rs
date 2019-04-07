extern crate clap;
extern crate luacompiler;
extern crate luavm;

use clap::{App, Arg};
use luacompiler::{
    bytecode::LuaBytecode, bytecodegen::compile_to_bytecode, irgen::compile_to_ir, LuaParseTree,
};
use luavm::Vm;
use std::fs::File;
use std::io::Read;
use std::path::Path;

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
    let file = Path::new(script_args.nth(0).unwrap());
    let file_str = file.to_str().unwrap();
    let bc = match file
        .extension()
        .expect("Input file has no extension!")
        .to_str()
        .unwrap()
    {
        "lua" => {
            let pt = LuaParseTree::new(&file_str);
            match pt {
                Ok(pt) => compile_to_bytecode(compile_to_ir(&pt)),
                Err(err) => panic!("{:#?}", err),
            }
        }
        "luabc" => {
            let mut contents = vec![];
            File::open(file)
                .unwrap()
                .read_to_end(&mut contents)
                .unwrap();
            LuaBytecode::new_from_bytes(contents)
        }
        _ => panic!("Expected a .lua or .luabc file!"),
    };
    if matches.is_present("bytecode") {
        println!("{}", &bc);
    }
    let mut all_args: Vec<&str> = vec![file_str];
    let script_args: Vec<&str> = script_args.map(|v| v).collect();
    all_args.extend(script_args);
    let mut vm = Vm::new(bc, all_args);
    vm.eval().unwrap();
}
