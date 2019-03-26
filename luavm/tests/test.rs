extern crate luacompiler;
extern crate luavm;
extern crate walkdir;

use luacompiler::{bytecodegen::compile_to_bytecode, irgen::compile_to_ir, LuaParseTree};
use luavm::Vm;
use std::ops::Add;
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

fn compile_and_run(file: &str) {
    let pt = LuaParseTree::new(file).expect(&format!("Failed to parse {}!", file));
    let bc = compile_to_bytecode(compile_to_ir(&pt));
    let mut vm = Vm::new(bc, vec![]);
    vm.eval().expect(&format!("Failed to execute {}!", file));
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

const LUAJIT_DIR: &'static str = "./tests/luajit2-test-suite/";
const LUAJIT_TESTS: [&'static str; 6] = [
    "ack.lua",
    "ack_notail.lua",
    "assign_tset_prevnil.lua",
    "assign_tset_tmp.lua",
    "tak.lua",
    "strcmp.lua",
];

#[test]
fn test_luajit_suite() {
    // test if luajit-test-suite is cloned
    if !Path::new(LUAJIT_DIR).exists() {
        // clone the repo
        Command::new("git")
            .arg("clone")
            .arg("https://github.com/openresty/luajit2-test-suite")
            .arg(LUAJIT_DIR)
            .status()
            .expect("'git clone' command failed to start");
    }
    for test in LUAJIT_TESTS.iter() {
        let source_file = String::from(LUAJIT_DIR).add("test/misc/").add(test);
        compile_and_run(&source_file);
    }
}
