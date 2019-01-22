extern crate luacompiler;
extern crate luavm;
extern crate walkdir;

use luacompiler::{bytecodegen::compile_to_bytecode, irgen::compile_to_ir, LuaParseTree};
use luavm::Vm;

use walkdir::WalkDir;

fn compile_and_run(file: &str) {
    println!("Parsing {}", file);
    let pt = LuaParseTree::new(file).unwrap();
    println!("Compiling {}", file);
    let bc = compile_to_bytecode(compile_to_ir(&pt));
    println!("Interpreting {}", file);
    let mut vm = Vm::new(bc);
    vm.eval();
}

#[test]
fn test_lua_sources() {
    for entry in WalkDir::new("./tests/lua_sources/")
        .into_iter()
        .filter(|e| match e {
            Ok(f) => !f.file_type().is_dir(),
            _ => false,
        })
    {
        compile_and_run(entry.unwrap().path().to_str().unwrap())
    }
}
