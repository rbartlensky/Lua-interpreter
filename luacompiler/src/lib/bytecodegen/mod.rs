pub mod constants_map;

use self::constants_map::ConstantsMap;
use bytecode::{
    instructions::{Opcode::*, *},
    Function, LuaBytecode,
};
use irgen::instr::Arg;
use irgen::lua_ir::LuaIR;
use irgen::opcodes::IROpcode::*;
use std::collections::HashMap;

pub fn compile_to_bytecode(ir: LuaIR) -> LuaBytecode {
    LuaIRToLuaBc::new(ir).compile()
}

struct LuaIRToLuaBc<'a> {
    ir: LuaIR<'a>,
    const_map: ConstantsMap,
    branches: Vec<(usize, usize)>,
    blocks: HashMap<usize, usize>,
}

impl<'a> LuaIRToLuaBc<'a> {
    /// Compile the given LuaIR to LuaBytecode.
    fn new(ir: LuaIR<'a>) -> LuaIRToLuaBc<'a> {
        LuaIRToLuaBc {
            ir,
            const_map: ConstantsMap::new(),
            branches: vec![],
            blocks: HashMap::new(),
        }
    }

    fn compile(mut self) -> LuaBytecode {
        self.ir.substitute_phis();
        let mut functions = vec![];
        for i in 0..self.ir.functions.len() {
            assert!(self.ir.functions[i].reg_count() < 255);
            functions.push(self.compile_function(i));
        }
        LuaBytecode::new(functions, self.ir.main_func, self.const_map)
    }

    fn compile_function(&mut self, i: usize) -> Function {
        let reg_count = self.ir.functions[i].reg_count();
        let mut instrs = Vec::with_capacity(reg_count);
        for bb in 0..self.ir.functions[i].blocks().len() {
            self.blocks.insert(bb, instrs.len());
            self.compile_basic_block(i, bb, &mut instrs);
        }
        for (instr, bb) in &self.branches {
            if opcode(instrs[*instr]) == Jmp as u8 {
                set_first_arg(&mut instrs[*instr], (self.blocks[&bb] - instr - 1) as u8);
            } else if opcode(instrs[*instr]) == JmpIf as u8 {
                set_second_arg(&mut instrs[*instr], (self.blocks[&bb] - instr - 1) as u8);
            }
        }
        self.branches.clear();
        self.blocks.clear();
        let func = &self.ir.functions[i];
        Function::new(i, func.reg_count() + 1, func.param_count(), instrs)
    }

    fn compile_basic_block(&mut self, f: usize, bb: usize, instrs: &mut Vec<u32>) {
        for i in 0..self.ir.functions[f].get_block(bb).instrs().len() {
            self.compile_instr(f, bb, i, instrs);
        }
    }

    fn compile_instr(&mut self, f: usize, bb: usize, i: usize, instrs: &mut Vec<u32>) {
        let last_reg = self.ir.functions[f].reg_count() as u8;
        let instr = self.ir.functions[f].get_block(bb).get(i);
        let args = &instr.args;
        let opcode = instr.opcode;
        match opcode {
            Opcode(MOV) => {
                let (opcode, arg2) = match instr.args[1] {
                    Arg::Reg(reg) => (MOV, reg),
                    Arg::Int(i) => (LDI, self.const_map.get_int(i)),
                    Arg::Float(f) => (LDF, self.const_map.get_float(f.to_string())),
                    Arg::Str(ref s) => (LDS, self.const_map.get_str(s.clone())),
                    _ => (MOV, 0),
                };
                instrs.push(make_instr(opcode, args[0].get_reg() as u8, arg2 as u8, 0))
            }
            Opcode(ADD) | Opcode(SUB) | Opcode(MUL) | Opcode(DIV) | Opcode(MOD) | Opcode(FDIV)
            | Opcode(EXP) | Opcode(EQ) => instrs.push(make_instr(
                opcode.opcode(),
                args[0].get_reg() as u8,
                args[1].get_reg() as u8,
                args[2].get_reg() as u8,
            )),
            Opcode(CLOSURE) => instrs.push(make_instr(
                opcode.opcode(),
                args[0].get_reg() as u8,
                args[1].get_func() as u8,
                0,
            )),
            Opcode(CALL) | Opcode(SetTop) => {
                instrs.push(make_instr(opcode.opcode(), args[0].get_reg() as u8, 0, 0))
            }
            Opcode(PUSH) => instrs.push(if instr.args.len() == 1 {
                make_instr(opcode.opcode(), args[0].get_reg() as u8, 0, 0)
            } else {
                make_instr(
                    opcode.opcode(),
                    args[0].get_reg() as u8,
                    args[1].get_some() as u8,
                    args[2].get_some() as u8,
                )
            }),
            Opcode(VarArg) | Opcode(MOVR) => instrs.push(if instr.args.len() == 2 {
                make_instr(
                    opcode.opcode(),
                    args[0].get_reg() as u8,
                    args[1].get_some() as u8,
                    0,
                )
            } else {
                make_instr(opcode.opcode(), 0, 0, args[2].get_some() as u8)
            }),
            Opcode(RET) => instrs.push(make_instr(opcode.opcode(), 0, 0, 0)),
            Opcode(GetUpAttr) => {
                let reg = args[0].get_reg() as u8;
                instrs.push(make_instr(
                    LDS,
                    reg,
                    self.const_map.get_str(args[2].get_str()) as u8,
                    0,
                ));
                instrs.push(make_instr(
                    opcode.opcode(),
                    reg as u8,
                    args[1].get_some() as u8,
                    reg,
                ));
            }
            Opcode(SetUpAttr) => {
                let reg = args[2].get_reg() as u8;
                instrs.push(make_instr(
                    LDS,
                    last_reg,
                    self.const_map.get_str(args[1].get_str()) as u8,
                    0,
                ));
                instrs.push(make_instr(
                    opcode.opcode(),
                    args[0].get_some() as u8,
                    last_reg,
                    reg,
                ));
            }
            Branch => {
                let len = instrs.len();
                instrs.push(if let Arg::Reg(reg) = args[0] {
                    self.branches.push((len, args[2].get_some()));
                    make_instr(JmpIf, reg as u8, 0, 0)
                } else {
                    self.branches.push((len, args[0].get_some()));
                    make_instr(Jmp, 0, 0, 0)
                })
            }
            // ignore phis as we have already processed them
            Phi => {}
            _ => panic!("Opcode {:?} cannot be compiled at the moment!", opcode),
        }
    }
}
