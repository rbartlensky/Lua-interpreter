use irgen::opcodes::IROpcode;

#[derive(PartialEq, Debug, Clone)]
pub enum Arg {
    Nil,
    Table,
    Int(i64),
    Float(f64),
    Str(String),
    Reg(usize),
    Func(usize),
    Some(usize),
}

impl Arg {
    pub fn is_reg(&self) -> bool {
        if let Arg::Reg(_) = self {
            true
        } else {
            false
        }
    }

    pub fn get_reg(&self) -> usize {
        if let Arg::Reg(reg) = self {
            *reg
        } else {
            panic!("Arg was not a Reg; received {:?}", self)
        }
    }

    pub fn get_some(&self) -> usize {
        if let Arg::Some(some) = self {
            *some
        } else {
            panic!("Arg was not a Some; received {:?}", self)
        }
    }

    pub fn get_str(&self) -> String {
        if let Arg::Str(s) = self {
            s.clone()
        } else {
            panic!("Arg was not a Str; received {:?}", self)
        }
    }

    pub fn get_func(&self) -> usize {
        if let Arg::Func(f) = self {
            *f
        } else {
            panic!("Arg was not a Func; received {:?}", self)
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum Instr {
    ZeroArg(IROpcode),
    OneArg(IROpcode, Arg),
    TwoArg(IROpcode, Arg, Arg),
    ThreeArg(IROpcode, Arg, Arg, Arg),
    NArg(IROpcode, Vec<Arg>),
}

impl Instr {
    pub fn opcode(&self) -> IROpcode {
        match *self {
            Instr::ZeroArg(o)
            | Instr::OneArg(o, _)
            | Instr::TwoArg(o, _, _)
            | Instr::ThreeArg(o, _, _, _)
            | Instr::NArg(o, _) => o,
        }
    }

    pub fn replace_regs_with(&mut self, regs: &[usize], with: usize) {
        match *self {
            Instr::OneArg(_, ref mut arg) => {
                if arg.is_reg() && regs.contains(&arg.get_reg()) {
                    *arg = Arg::Reg(with)
                }
            }
            Instr::TwoArg(_, ref mut arg1, ref mut arg2) => {
                if arg1.is_reg() && regs.contains(&arg1.get_reg()) {
                    *arg1 = Arg::Reg(with)
                }
                if arg2.is_reg() && regs.contains(&arg2.get_reg()) {
                    *arg2 = Arg::Reg(with)
                }
            }
            Instr::ThreeArg(_, ref mut arg1, ref mut arg2, ref mut arg3) => {
                if arg1.is_reg() && regs.contains(&arg1.get_reg()) {
                    *arg1 = Arg::Reg(with)
                }
                if arg2.is_reg() && regs.contains(&arg2.get_reg()) {
                    *arg2 = Arg::Reg(with)
                }
                if arg3.is_reg() && regs.contains(&arg3.get_reg()) {
                    *arg3 = Arg::Reg(with)
                }
            }
            Instr::NArg(_, ref mut args) => {
                for i in 0..args.len() {
                    if args[i].is_reg() && regs.contains(&args[i].get_reg()) {
                        args[i] = Arg::Reg(with);
                    }
                }
            }
            _ => {}
        }
    }
}
