use irgen::{compiled_func::CompiledFunc, instr::*, opcodes::*};

/// Represents an IR in which all instructions are in SSA form.
pub struct LuaIR<'a> {
    pub functions: Vec<CompiledFunc<'a>>,
    pub main_func: usize,
}

impl<'a> LuaIR<'a> {
    pub fn new(functions: Vec<CompiledFunc<'a>>, main_func: usize) -> LuaIR<'a> {
        LuaIR {
            functions,
            main_func,
        }
    }

    pub fn substitute_phis(&mut self) {
        for f in 0..self.functions.len() {
            let mut points = vec![];
            let len = self.functions[f].blocks().len();
            for bb in 0..len {
                for i in 0..self.functions[f].get_block(bb).instrs().len() {
                    let instr = self.functions[f].get_block(bb).get(i);
                    if let Instr {
                        opcode: IROpcode::Phi,
                        ref args,
                    } = instr
                    {
                        points.push((bb, args.clone()));
                    }
                }
            }
            for (bb, args) in points {
                for block in &mut self.functions[f].get_mut_blocks()[..bb] {
                    block.replace_regs_with(&args[1..], &args[0])
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytecode::instructions::Opcode;

    #[test]
    fn substitute_phis() {
        let mut func = CompiledFunc::new(0, false);
        func.create_block();
        func.create_block();
        func.create_block();
        func.get_mut_block(0).push_instr(
            IROpcode::from(Opcode::MOV),
            vec![Arg::Reg(1), Arg::Reg(2), Arg::Reg(5)],
        );
        func.get_mut_block(0).push_instr(
            IROpcode::from(Opcode::MOV),
            vec![Arg::Reg(3), Arg::Reg(2), Arg::Reg(3)],
        );
        func.get_mut_block(1)
            .push_instr(IROpcode::Phi, vec![Arg::Reg(4), Arg::Reg(1), Arg::Reg(2)]);
        func.get_mut_block(2)
            .push_instr(IROpcode::Phi, vec![Arg::Reg(6), Arg::Reg(5)]);
        let mut ir = LuaIR::new(vec![func], 0);
        ir.substitute_phis();
        let expected = vec![
            vec![
                Instr::new(
                    IROpcode::from(Opcode::MOV),
                    vec![Arg::Reg(4), Arg::Reg(4), Arg::Reg(6)],
                ),
                Instr::new(
                    IROpcode::from(Opcode::MOV),
                    vec![Arg::Reg(3), Arg::Reg(4), Arg::Reg(3)],
                ),
            ],
            vec![Instr::new(
                IROpcode::Phi,
                vec![Arg::Reg(4), Arg::Reg(1), Arg::Reg(2)],
            )],
            vec![Instr::new(
                IROpcode::Phi,
                vec![Arg::Reg(6), Arg::Reg(5)],
            )],
        ];
        for (i, bb) in ir.functions[0].blocks().iter().enumerate() {
            assert_eq!(bb.instrs(), &expected[i]);
        }
    }
}
