extern crate luacompiler;

use luacompiler::{
    bytecode::instructions::{make_instr, Opcode},
    bytecodegen::compile_to_bytecode,
    irgen::compile_to_ir,
    LuaParseTree,
};

#[test]
fn ldi_generation() {
    let pt = LuaParseTree::from_str(String::from("x = 1")).unwrap();
    let bc = compile_to_bytecode(compile_to_ir(&pt));
    assert_eq!(bc.instrs_len(), 2);
    assert_eq!(bc.reg_count(), 2);
    assert_eq!(bc.get_int(0), 1);
    assert_eq!(bc.get_instr(0), make_instr(Opcode::LDI, 0, 0, 0));
    assert_eq!(bc.get_instr(1), make_instr(Opcode::MOV, 1, 0, 0));
}

#[test]
fn ldf_generation() {
    let pt = LuaParseTree::from_str(String::from("x = 2.0")).unwrap();
    let bc = compile_to_bytecode(compile_to_ir(&pt));
    assert_eq!(bc.instrs_len(), 2);
    assert_eq!(bc.reg_count(), 2);
    assert_eq!(bc.get_float(0).to_string(), "2");
    assert_eq!(bc.get_instr(0), make_instr(Opcode::LDF, 0, 0, 0));
    assert_eq!(bc.get_instr(1), make_instr(Opcode::MOV, 1, 0, 0));
}

#[test]
fn lds_generation() {
    let pt = LuaParseTree::from_str(String::from("x = \"1.2\"")).unwrap();
    let bc = compile_to_bytecode(compile_to_ir(&pt));
    assert_eq!(bc.instrs_len(), 2);
    assert_eq!(bc.reg_count(), 2);
    assert_eq!(bc.get_string(0), "1.2");
    assert_eq!(bc.get_instr(0), make_instr(Opcode::LDS, 0, 0, 0));
    assert_eq!(bc.get_instr(1), make_instr(Opcode::MOV, 1, 0, 0));
}

fn assert_bytecode(opcode: Opcode, operation: &str) {
    let pt = LuaParseTree::from_str(String::from(format!("x = 1 {} 2", operation))).unwrap();
    let bc = compile_to_bytecode(compile_to_ir(&pt));
    assert_eq!(bc.instrs_len(), 4);
    assert_eq!(bc.reg_count(), 4);
    assert_eq!(bc.get_int(0), 1);
    assert_eq!(bc.get_int(1), 2);
    assert_eq!(bc.get_instr(0), make_instr(Opcode::LDI, 0, 0, 0));
    assert_eq!(bc.get_instr(1), make_instr(Opcode::LDI, 1, 1, 0));
    assert_eq!(bc.get_instr(2), make_instr(opcode, 2, 0, 1));
    assert_eq!(bc.get_instr(3), make_instr(Opcode::MOV, 3, 2, 0));
}

#[test]
fn add_generation() {
    assert_bytecode(Opcode::ADD, "+");
}

#[test]
fn sub_generation() {
    assert_bytecode(Opcode::SUB, "-");
}

#[test]
fn mul_generation() {
    assert_bytecode(Opcode::MUL, "*");
}

#[test]
fn div_generation() {
    assert_bytecode(Opcode::DIV, "/");
}

#[test]
fn mod_generation() {
    assert_bytecode(Opcode::MOD, "%");
}

#[test]
fn fdiv_generation() {
    assert_bytecode(Opcode::FDIV, "//");
}

#[test]
fn exp_generation() {
    assert_bytecode(Opcode::EXP, "^");
}
