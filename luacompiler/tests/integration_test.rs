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
    assert_eq!(bc.reg_count(), 3);
    assert_eq!(bc.get_int(0), 1);
    assert_eq!(bc.get_string(0), "x");
    let expected_instrs = vec![
        make_instr(Opcode::LDI, 1, 0, 0),
        make_instr(Opcode::LDS, 2, 0, 0),
        make_instr(Opcode::SetAttr, 0, 2, 1),
    ];
    assert_eq!(bc.instrs_len(), expected_instrs.len());
    for i in 0..expected_instrs.len() {
        assert_eq!(bc.get_instr(i), expected_instrs[i]);
    }
}

#[test]
fn ldf_generation() {
    let pt = LuaParseTree::from_str(String::from("x = 2.0")).unwrap();
    let bc = compile_to_bytecode(compile_to_ir(&pt));
    assert_eq!(bc.reg_count(), 3);
    assert_eq!(bc.get_float(0).to_string(), "2");
    assert_eq!(bc.get_string(0), "x");
    let expected_instrs = vec![
        make_instr(Opcode::LDF, 1, 0, 0),
        make_instr(Opcode::LDS, 2, 0, 0),
        make_instr(Opcode::SetAttr, 0, 2, 1),
    ];
    assert_eq!(bc.instrs_len(), expected_instrs.len());
    for i in 0..expected_instrs.len() {
        assert_eq!(bc.get_instr(i), expected_instrs[i]);
    }
}

#[test]
fn lds_generation() {
    let pt = LuaParseTree::from_str(String::from("x = \"1.2\"")).unwrap();
    let bc = compile_to_bytecode(compile_to_ir(&pt));
    assert_eq!(bc.reg_count(), 3);
    assert_eq!(bc.get_string(0), "1.2");
    assert_eq!(bc.get_string(1), "x");
    let expected_instrs = vec![
        make_instr(Opcode::LDS, 1, 0, 0),
        make_instr(Opcode::LDS, 2, 1, 0),
        make_instr(Opcode::SetAttr, 0, 2, 1),
    ];
    assert_eq!(bc.instrs_len(), expected_instrs.len());
    for i in 0..expected_instrs.len() {
        assert_eq!(bc.get_instr(i), expected_instrs[i]);
    }
}

fn assert_bytecode(opcode: Opcode, operation: &str) {
    let pt = LuaParseTree::from_str(String::from(format!("x = 1 {} 2", operation))).unwrap();
    let bc = compile_to_bytecode(compile_to_ir(&pt));
    assert_eq!(bc.get_int(0), 1);
    assert_eq!(bc.get_int(1), 2);
    assert_eq!(bc.get_string(0), "x");
    let expected_instrs = vec![
        make_instr(Opcode::LDI, 1, 0, 0),
        make_instr(Opcode::LDI, 2, 1, 0),
        make_instr(opcode, 3, 1, 2),
        make_instr(Opcode::LDS, 4, 0, 0),
        make_instr(Opcode::SetAttr, 0, 4, 3),
    ];
    assert_eq!(bc.instrs_len(), expected_instrs.len());
    for i in 0..expected_instrs.len() {
        assert_eq!(bc.get_instr(i), expected_instrs[i]);
    }
    assert_eq!(bc.reg_count(), 5);
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
