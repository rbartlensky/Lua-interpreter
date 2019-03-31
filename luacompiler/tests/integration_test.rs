extern crate luacompiler;
extern crate walkdir;

use luacompiler::{
    bytecode::instructions::{make_instr, Opcode},
    bytecodegen::compile_to_bytecode,
    irgen::compile_to_ir,
    LuaParseTree,
};
use std::fs::File;
use std::io::Read;
use std::ops::Add;
use walkdir::WalkDir;

#[test]
fn ldi_generation() {
    let pt = LuaParseTree::from_str(String::from("x = 1")).unwrap();
    let bc = compile_to_bytecode(compile_to_ir(&pt));
    assert_eq!(bc.get_int(0), 1);
    assert_eq!(bc.get_string(0), "x");
    let expected_instrs = vec![
        make_instr(Opcode::Ldi, 0, 0, 0),
        make_instr(Opcode::Lds, 1, 0, 0),
        make_instr(Opcode::SetUpAttr, 0, 1, 0),
    ];
    let function = bc.get_function(bc.get_main_function());
    assert_eq!(function.reg_count(), 2);
    assert_eq!(function.instrs_len(), expected_instrs.len());
    for i in 0..expected_instrs.len() {
        assert_eq!(function.get_instr(i), expected_instrs[i]);
    }
}

#[test]
fn ldf_generation() {
    let pt = LuaParseTree::from_str(String::from("x = 2.0")).unwrap();
    let bc = compile_to_bytecode(compile_to_ir(&pt));
    assert_eq!(bc.get_float(0).to_string(), "2");
    assert_eq!(bc.get_string(0), "x");
    let expected_instrs = vec![
        make_instr(Opcode::Ldf, 0, 0, 0),
        make_instr(Opcode::Lds, 1, 0, 0),
        make_instr(Opcode::SetUpAttr, 0, 1, 0),
    ];
    let function = bc.get_function(bc.get_main_function());
    assert_eq!(function.reg_count(), 2);
    assert_eq!(function.instrs_len(), expected_instrs.len());
    for i in 0..expected_instrs.len() {
        assert_eq!(function.get_instr(i), expected_instrs[i]);
    }
}

#[test]
fn lds_generation() {
    let pt = LuaParseTree::from_str(String::from("x = \"1.2\"")).unwrap();
    let bc = compile_to_bytecode(compile_to_ir(&pt));
    assert_eq!(bc.get_string(0), "1.2");
    assert_eq!(bc.get_string(1), "x");
    let expected_instrs = vec![
        make_instr(Opcode::Lds, 0, 0, 0),
        make_instr(Opcode::Lds, 1, 1, 0),
        make_instr(Opcode::SetUpAttr, 0, 1, 0),
    ];
    let function = bc.get_function(bc.get_main_function());
    assert_eq!(function.reg_count(), 2);
    assert_eq!(function.instrs_len(), expected_instrs.len());
    for i in 0..expected_instrs.len() {
        assert_eq!(function.get_instr(i), expected_instrs[i]);
    }
}

fn assert_bytecode(opcode: Opcode, operation: &str) {
    let pt = LuaParseTree::from_str(String::from(format!("x = 1 {} 2", operation))).unwrap();
    let bc = compile_to_bytecode(compile_to_ir(&pt));
    assert_eq!(bc.get_int(0), 1);
    assert_eq!(bc.get_int(1), 2);
    assert_eq!(bc.get_string(0), "x");
    let expected_instrs = vec![
        make_instr(Opcode::Ldi, 0, 0, 0),
        make_instr(Opcode::Ldi, 1, 1, 0),
        make_instr(opcode, 2, 0, 1),
        make_instr(Opcode::Lds, 3, 0, 0),
        make_instr(Opcode::SetUpAttr, 0, 3, 2),
    ];
    let function = bc.get_function(bc.get_main_function());
    assert_eq!(function.reg_count(), 4);
    assert_eq!(function.instrs_len(), expected_instrs.len());
    for i in 0..expected_instrs.len() {
        assert_eq!(function.get_instr(i), expected_instrs[i]);
    }
}

#[test]
fn add_generation() {
    assert_bytecode(Opcode::Add, "+");
}

#[test]
fn sub_generation() {
    assert_bytecode(Opcode::Sub, "-");
}

#[test]
fn mul_generation() {
    assert_bytecode(Opcode::Mul, "*");
}

#[test]
fn div_generation() {
    assert_bytecode(Opcode::Div, "/");
}

#[test]
fn mod_generation() {
    assert_bytecode(Opcode::Mod, "%");
}

#[test]
fn fdiv_generation() {
    assert_bytecode(Opcode::FDiv, "//");
}

#[test]
fn exp_generation() {
    assert_bytecode(Opcode::Exp, "^");
}

const LUAVM_TEST_DIR: &'static str = "../luavm/tests/lua_sources/";
const LUACOMP_OUT_DIR: &'static str = "./tests/bc_out/";

#[test]
fn bytecode_output() {
    for entry in WalkDir::new(LUACOMP_OUT_DIR)
        .into_iter()
        .filter(|e| match e {
            Ok(f) => !f.file_type().is_dir(),
            _ => false,
        })
    {
        let entry = entry.unwrap();
        let file = String::from(LUAVM_TEST_DIR)
            .add(entry.file_name().to_str().unwrap())
            .add(".lua");
        let pt = LuaParseTree::new(&file).unwrap();
        let bc = compile_to_bytecode(compile_to_ir(&pt));
        let mut contents = String::new();
        File::open(entry.path())
            .unwrap()
            .read_to_string(&mut contents)
            .unwrap();
        assert_eq!(format!("{}", bc), contents);
    }
}
