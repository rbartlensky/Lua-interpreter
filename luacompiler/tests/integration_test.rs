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
    assert_eq!(bc.get_int(0), 1);
    assert_eq!(bc.get_string(0), "x");
    let expected_instrs = vec![
        make_instr(Opcode::LDI, 0, 0, 0),
        make_instr(Opcode::LDS, 1, 0, 0),
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
        make_instr(Opcode::LDF, 0, 0, 0),
        make_instr(Opcode::LDS, 1, 0, 0),
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
        make_instr(Opcode::LDS, 0, 0, 0),
        make_instr(Opcode::LDS, 1, 1, 0),
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
        make_instr(Opcode::LDI, 0, 0, 0),
        make_instr(Opcode::LDI, 1, 1, 0),
        make_instr(opcode, 2, 0, 1),
        make_instr(Opcode::LDS, 3, 0, 0),
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
