pub mod constants_map;

use self::constants_map::ConstantsMap;
use bytecode::{
    instructions::{make_instr, Opcode::*},
    Function, LuaBytecode,
};
use irgen::instr::Arg;
use irgen::lua_ir::LuaIR;
use irgen::opcodes::IROpcode;

pub fn compile_to_bytecode(ir: LuaIR) -> LuaBytecode {
    LuaIRToLuaBc::new(ir).compile()
}

struct LuaIRToLuaBc<'a> {
    ir: LuaIR<'a>,
    const_map: ConstantsMap,
}

impl<'a> LuaIRToLuaBc<'a> {
    /// Compile the given LuaIR to LuaBytecode.
    fn new(ir: LuaIR) -> LuaIRToLuaBc {
        LuaIRToLuaBc {
            ir,
            const_map: ConstantsMap::new(),
        }
    }

    fn compile(mut self) -> LuaBytecode {
        let mut functions = vec![];
        for i in 0..self.ir.functions.len() {
            assert!(self.ir.functions[i].reg_count() < 255);
            functions.push(self.compile_function(i));
        }
        LuaBytecode::new(functions, self.ir.main_func, self.const_map)
    }

    fn compile_function(&mut self, i: usize) -> Function {
        let mut instrs = vec![];
        for bb in 0..self.ir.functions[i].blocks().len() {
            for j in 0..self.ir.functions[i].get_block(bb).instrs().len() {
                instrs.extend(self.compile_instr(i, bb, j));
            }
        }
        let func = &self.ir.functions[i];
        Function::new(i, func.reg_count() + 1, func.param_count(), instrs)
    }

    fn compile_instr(&mut self, f: usize, bb: usize, i: usize) -> Vec<u32> {
        let last_reg = self.ir.functions[f].reg_count() as u8;
        let instr = &self.ir.functions[f].get_block(bb).instrs()[i];
        let args = &instr.args;
        let opcode = match instr.opcode {
            IROpcode::Opcode(o) => o,
            _ => unreachable!(""),
        };
        match opcode {
            MOV => {
                let (opcode, arg2) = match instr.args[1] {
                    Arg::Reg(reg) => (MOV, reg),
                    Arg::Int(i) => (LDI, self.const_map.get_int(i)),
                    Arg::Float(f) => (LDF, self.const_map.get_float(f.to_string())),
                    Arg::Str(ref s) => (LDS, self.const_map.get_str(s.clone())),
                    _ => (MOV, 0),
                };
                vec![make_instr(opcode, args[0].get_reg() as u8, arg2 as u8, 0)]
            }
            ADD | SUB | MUL | DIV | MOD | FDIV | EXP | EQ => vec![make_instr(
                opcode,
                args[0].get_reg() as u8,
                args[1].get_reg() as u8,
                args[2].get_reg() as u8,
            )],
            CLOSURE => vec![make_instr(
                opcode,
                args[0].get_reg() as u8,
                args[1].get_func() as u8,
                0,
            )],
            CALL | SetTop => vec![make_instr(opcode, args[0].get_reg() as u8, 0, 0)],
            PUSH => vec![if instr.args.len() == 1 {
                make_instr(opcode, args[0].get_reg() as u8, 0, 0)
            } else {
                make_instr(
                    opcode,
                    args[0].get_reg() as u8,
                    args[1].get_some() as u8,
                    args[2].get_some() as u8,
                )
            }],
            VarArg | MOVR => vec![if instr.args.len() == 2 {
                make_instr(opcode, args[0].get_reg() as u8, args[1].get_some() as u8, 0)
            } else {
                make_instr(opcode, 0, 0, args[2].get_some() as u8)
            }],
            RET => vec![make_instr(opcode, 0, 0, 0)],
            GetUpAttr => {
                let reg = args[0].get_reg() as u8;
                vec![
                    make_instr(LDS, reg, self.const_map.get_str(args[2].get_str()) as u8, 0),
                    make_instr(opcode, reg as u8, args[1].get_some() as u8, reg),
                ]
            }
            SetUpAttr => {
                let reg = args[2].get_reg() as u8;
                vec![
                    make_instr(
                        LDS,
                        last_reg,
                        self.const_map.get_str(args[1].get_str()) as u8,
                        0,
                    ),
                    make_instr(opcode, args[0].get_some() as u8, last_reg, reg),
                ]
            }
            _ => panic!("Opcode {:?} cannot be compiled at the moment!", opcode),
        }
    }
}
