use irgen::{
    compiled_func::{CompiledFunc, ProviderType},
    instr::*,
    opcodes::IROpcode::*,
};
use std::collections::BTreeMap;

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
            // we use a treemap in order to apply phis in order
            let mut substs: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
            let len = self.functions[f].blocks().len();
            for bb in 0..len {
                for i in 0..self.functions[f].get_block(bb).instrs().len() {
                    let instr = self.functions[f].get_mut_block(bb).get_mut(i);
                    if let Instr::NArg(Phi, ref mut args) = instr {
                        let mut new_args: Vec<Arg> = vec![];
                        std::mem::swap(args, &mut new_args);
                        let mut new_regs: Vec<usize> =
                            new_args.into_iter().map(|a| a.get_reg()).collect();
                        let mut args = new_regs.split_off(1);
                        let k = new_regs[0];
                        substs
                            .entry(k)
                            .and_modify(|vec| vec.append(&mut args))
                            .or_insert(args);
                    }
                }
            }
            for (&k, v) in &substs {
                for bb in 0..len {
                    let block = self.functions[f].get_mut_block(bb);
                    block.replace_regs_with(v, k);
                }
                // also update the provides
                for (_, provides) in self.functions[f].provides_mut().iter_mut() {
                    for (_, ty) in provides.iter_mut() {
                        if let ProviderType::Reg(ref mut reg) = ty {
                            if v.contains(&reg) {
                                *reg = k;
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitute_phis() {
        let mut func = CompiledFunc::new(0, false);
        func.create_block();
        func.create_block();
        func.create_block();
        func.get_mut_block(0).mut_instrs().push(Instr::ThreeArg(
            MOV,
            Arg::Reg(1),
            Arg::Reg(2),
            Arg::Reg(5),
        ));
        func.get_mut_block(0).mut_instrs().push(Instr::ThreeArg(
            MOV,
            Arg::Reg(3),
            Arg::Reg(2),
            Arg::Reg(3),
        ));
        func.get_mut_block(1).mut_instrs().push(Instr::NArg(
            Phi,
            vec![Arg::Reg(4), Arg::Reg(1), Arg::Reg(2)],
        ));
        func.get_mut_block(2)
            .mut_instrs()
            .push(Instr::NArg(Phi, vec![Arg::Reg(6), Arg::Reg(5)]));
        let mut ir = LuaIR::new(vec![func], 0);
        ir.substitute_phis();
        let expected = vec![
            vec![
                Instr::ThreeArg(MOV, Arg::Reg(4), Arg::Reg(4), Arg::Reg(6)),
                Instr::ThreeArg(MOV, Arg::Reg(3), Arg::Reg(4), Arg::Reg(3)),
            ],
            vec![Instr::NArg(Phi, vec![])],
            vec![Instr::NArg(Phi, vec![])],
        ];
        for (i, bb) in ir.functions[0].blocks().iter().enumerate() {
            assert_eq!(bb.instrs(), &expected[i]);
        }
    }
}
