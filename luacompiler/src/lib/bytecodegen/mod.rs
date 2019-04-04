pub mod constants_map;

use self::constants_map::ConstantsMap;
use bytecode::{instructions::*, BCProviderType, BcFunc, LuaBytecode};
use irgen::instr::{Arg, Instr};
use irgen::lua_ir::LuaIR;
use irgen::opcodes::IROpcode::*;
use std::collections::HashMap;

pub fn fit_in_u8(v: usize) -> u8 {
    if v <= u8::max_value() as usize {
        v as u8
    } else {
        panic!("Value {} does not fit in a u8!", v)
    }
}

pub fn compile_to_bytecode(ir: LuaIR) -> LuaBytecode {
    BcGen::new(ir).compile()
}

struct BcGen<'a> {
    ir: LuaIR<'a>,
    const_map: ConstantsMap,
    branches: Vec<(usize, usize)>,
    blocks: HashMap<usize, usize>,
}

impl<'a> BcGen<'a> {
    /// Compile the given LuaIR to LuaBytecode.
    fn new(ir: LuaIR<'a>) -> BcGen<'a> {
        BcGen {
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
            assert!(self.ir.functions[i].reg_count() <= u8::max_value() as usize);
            functions.push(self.compile_function(i));
        }
        LuaBytecode::new(functions, self.ir.main_func, self.const_map)
    }

    fn compile_function(&mut self, i: usize) -> BcFunc {
        let reg_count = self.ir.functions[i].reg_count();
        let mut instrs = Vec::with_capacity(reg_count);
        for bb in 0..self.ir.functions[i].blocks().len() {
            self.blocks.insert(bb, instrs.len());
            self.compile_basic_block(i, bb, &mut instrs);
        }
        // check which instructions need patching
        for (instr, bb) in &self.branches {
            if opcode(instrs[*instr]) == Opcode::Jmp as u8
                || opcode(instrs[*instr]) == Opcode::JmpEq as u8
                || opcode(instrs[*instr]) == Opcode::JmpNe as u8
            {
                let jmp: i16 = self.blocks[&bb] as i16 - *instr as i16 - 1;
                set_extended_arg(&mut instrs[*instr], jmp);
            }
        }
        self.branches.clear();
        self.blocks.clear();
        // generate the bytecode version of the provides mapping
        let provides: HashMap<u8, Vec<(BCProviderType, u8)>> = self.ir.functions[i]
            .provides()
            .iter()
            .map(|(k, v)| {
                let new_k = fit_in_u8(*k);
                let new_v = v
                    .iter()
                    .map(|(i, pt)| (BCProviderType::from(pt), fit_in_u8(*i)))
                    .collect();
                (new_k, new_v)
            })
            .collect();
        let func = &self.ir.functions[i];
        BcFunc::new(
            i,
            func.reg_count() + 1,
            func.param_count(),
            func.upvals().len(),
            provides,
            instrs,
        )
    }

    fn compile_basic_block(&mut self, f: usize, bb: usize, instrs: &mut Vec<u32>) {
        for i in 0..self.ir.functions[f].get_block(bb).instrs().len() {
            self.compile_instr(f, bb, i, instrs);
        }
    }

    fn compile_instr(&mut self, f: usize, bb: usize, i: usize, instrs: &mut Vec<u32>) {
        let last_reg = self.ir.functions[f].reg_count() as u8;
        let instr = self.ir.functions[f].get_block(bb).get(i);
        let opcode = instr.opcode();
        match opcode {
            Mov => {
                if let Instr::TwoArg(_, ref arg1, ref arg2) = instr {
                    let (opcode, arg2) = match *arg2 {
                        Arg::Reg(reg) => (Opcode::Mov, reg),
                        Arg::Int(i) => (Opcode::Ldi, self.const_map.get_int(i)),
                        Arg::Float(f) => (Opcode::Ldf, self.const_map.get_float(f.to_string())),
                        Arg::Str(ref s) => (Opcode::Lds, self.const_map.get_str(s.clone())),
                        Arg::Nil => (Opcode::Ldn, 0),
                        Arg::Table => (Opcode::Ldt, 0),
                        Arg::Bool(b) => (Opcode::Ldb, b as usize),
                        _ => panic!("Mov shouldn't have {:?} as an argument.", arg2),
                    };
                    instrs.push(make_instr(opcode, arg1.get_reg() as u8, arg2 as u8, 0))
                }
            }
            Add | Sub | Mul | Div | Mod | FDiv | Exp | Eq | Lt | Gt | Le | Ge | Ne => {
                if let Instr::ThreeArg(_, arg1, arg2, arg3) = instr {
                    instrs.push(make_instr(
                        opcode.to_opcode(),
                        arg1.get_reg() as u8,
                        arg2.get_reg() as u8,
                        arg3.get_reg() as u8,
                    ))
                }
            }
            Closure => {
                if let Instr::TwoArg(_, arg1, arg2) = instr {
                    instrs.push(make_instr(
                        opcode.to_opcode(),
                        arg1.get_reg() as u8,
                        arg2.get_func() as u8,
                        0,
                    ))
                }
            }
            Call | SetTop => {
                if let Instr::OneArg(_, arg1) = instr {
                    instrs.push(make_instr(opcode.to_opcode(), arg1.get_reg() as u8, 0, 0))
                }
            }
            Push => instrs.push(if let Instr::OneArg(_, arg1) = instr {
                make_instr(opcode.to_opcode(), arg1.get_reg() as u8, 0, 0)
            } else if let Instr::TwoArg(_, arg1, arg2) = instr {
                make_instr(
                    opcode.to_opcode(),
                    arg1.get_reg() as u8,
                    arg2.get_some() as u8,
                    0,
                )
            } else {
                panic!("Not enough arguments for {:?}!", opcode)
            }),
            VarArg | MovR => instrs.push(if let Instr::TwoArg(_, arg1, arg2) = instr {
                make_instr(
                    opcode.to_opcode(),
                    arg1.get_reg() as u8,
                    arg2.get_stackval() as u8,
                    0,
                )
            } else if let Instr::OneArg(_, arg1) = instr {
                make_instr(opcode.to_opcode(), 0, 0, arg1.get_some() as u8)
            } else {
                panic!("Not enough arguments for {:?}!", opcode)
            }),
            Ret => instrs.push(make_instr(opcode.to_opcode(), 0, 0, 0)),
            GetUpAttr => {
                if let Instr::ThreeArg(_, arg1, arg2, arg3) = instr {
                    let reg = arg1.get_reg() as u8;
                    instrs.push(make_instr(
                        Opcode::Lds,
                        reg,
                        self.const_map.get_str(arg3.get_str()) as u8,
                        0,
                    ));
                    instrs.push(make_instr(
                        opcode.to_opcode(),
                        reg as u8,
                        arg2.get_upval() as u8,
                        reg,
                    ));
                }
            }
            SetUpAttr => {
                if let Instr::ThreeArg(_, arg1, arg2, arg3) = instr {
                    let reg = arg3.get_reg() as u8;
                    instrs.push(make_instr(
                        Opcode::Lds,
                        last_reg,
                        self.const_map.get_str(arg2.get_str()) as u8,
                        0,
                    ));
                    instrs.push(make_instr(
                        opcode.to_opcode(),
                        arg1.get_upval() as u8,
                        last_reg,
                        reg,
                    ));
                }
            }
            Jmp => {
                let len = instrs.len();
                instrs.push(if let Instr::OneArg(_, arg1) = instr {
                    self.branches.push((len, arg1.get_block()));
                    make_instr(opcode.to_opcode(), 0, 0, 0)
                } else {
                    panic!("Not enough arguments for {:?}!", opcode)
                })
            }
            JmpNe => {
                let len = instrs.len();
                instrs.push(if let Instr::ThreeArg(_, arg1, _, arg3) = instr {
                    self.branches.push((len, arg3.get_block()));
                    make_instr(opcode.to_opcode(), arg1.get_reg() as u8, 0, 0)
                } else {
                    panic!("Not enough arguments for {:?}!", opcode)
                })
            }
            JmpEq => {
                let len = instrs.len();
                instrs.push(if let Instr::ThreeArg(_, arg1, arg2, _) = instr {
                    self.branches.push((len, arg2.get_block()));
                    make_instr(opcode.to_opcode(), arg1.get_reg() as u8, 0, 0)
                } else {
                    panic!("Not enough arguments for {:?}!", opcode)
                })
            }
            GetAttr | SetAttr => {
                if let Instr::ThreeArg(_, arg1, arg2, arg3) = instr {
                    instrs.push(make_instr(
                        opcode.to_opcode(),
                        arg1.get_reg() as u8,
                        arg2.get_reg() as u8,
                        arg3.get_reg() as u8,
                    ))
                } else {
                    panic!("GetAttr should be a Instr::ThreeArg instruction!")
                }
            }
            GetUpVal => {
                if let Instr::TwoArg(_, arg1, arg2) = instr {
                    instrs.push(make_instr(
                        opcode.to_opcode(),
                        arg1.get_reg() as u8,
                        arg2.get_upval() as u8,
                        0,
                    ))
                } else {
                    panic!("GetUpVal should be a Instr::TwoArg instruction!")
                }
            }
            SetUpVal => {
                if let Instr::TwoArg(_, arg1, arg2) = instr {
                    instrs.push(make_instr(
                        opcode.to_opcode(),
                        arg1.get_upval() as u8,
                        arg2.get_reg() as u8,
                        0,
                    ))
                } else {
                    panic!("SetUpVal should be a Instr::TwoArg instruction!")
                }
            }
            Umn => {
                if let Instr::TwoArg(_, arg1, arg2) = instr {
                    instrs.push(make_instr(
                        opcode.to_opcode(),
                        arg1.get_reg() as u8,
                        arg2.get_reg() as u8,
                        0,
                    ))
                } else {
                    panic!("Umn should be a Instr::TwoArg instruction!")
                }
            }
            // ignore phis as we have already processed them
            Phi => {}
        }
    }
}
