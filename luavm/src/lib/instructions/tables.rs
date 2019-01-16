use errors::LuaError;
use luacompiler::bytecode::instructions::{first_arg, second_arg, third_arg};
use luacompiler::irgen::register_map::ENV_REG;
use Vm;

/// R(1) = R(2)[R(3)]
pub fn get_attr(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    let val = {
        let arg2 = second_arg(instr) as usize;
        let from = &vm.registers[arg2];
        let attr = &vm.registers[third_arg(instr) as usize];
        match attr.get_constant_index() {
            Some(i) => {
                if arg2 == ENV_REG {
                    vm.env_attrs[i].clone()
                } else {
                    from.get_attr(attr)?
                }
            }
            _ => from.get_attr(attr)?,
        }
    };
    vm.registers[first_arg(instr) as usize] = val;
    Ok(())
}

/// R(1)[R(2)] = R(3)
pub fn set_attr(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    let attr = vm.registers[second_arg(instr) as usize].clone();
    let val = vm.registers[third_arg(instr) as usize].clone();
    let arg1 = first_arg(instr) as usize;
    match attr.get_constant_index() {
        Some(i) => {
            if arg1 == ENV_REG {
                vm.env_attrs[i] = val
            } else {
                vm.registers[arg1].set_attr(attr, val)?
            }
        }
        _ => vm.registers[arg1].set_attr(attr, val)?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lua_values::LuaVal;
    use luacompiler::{
        bytecode::instructions::{make_instr, Opcode},
        bytecodegen::compile_to_bytecode,
        irgen::compile_to_ir,
        LuaParseTree,
    };

    fn get_vm_for(p: String) -> Vm {
        let pt = LuaParseTree::from_str(p).unwrap();
        let ir = compile_to_ir(&pt);
        let bc = compile_to_bytecode(ir);
        Vm::new(bc)
    }

    #[test]
    fn get_attr_works() {
        // this should generate:
        // LDI     1 0 0
        // LDS     2 0 0
        // SetAttr 0 2 1
        let mut vm = get_vm_for("x = 2".to_string());
        vm.eval(); // so that the registers are updated based on the supplied program
        assert!(get_attr(&mut vm, make_instr(Opcode::GetAttr, 1, ENV_REG as u8, 2)).is_ok());
        assert_eq!(vm.registers[1], LuaVal::from(2));
    }

    #[test]
    fn set_attr_works() {
        // this should generate:
        // LDI     1 0 0
        // LDS     2 0 0
        // SetAttr 0 2 1
        let mut vm = get_vm_for("x = 2".to_string());
        vm.eval(); // so that the registers are updated based on the supplied program
        assert!(set_attr(&mut vm, make_instr(Opcode::SetAttr, ENV_REG as u8, 2, 1)).is_ok());
        let index_of_x = 0;
        assert_eq!(
            vm.registers[ENV_REG]
                .get_attr(&LuaVal::from((String::from("x"), index_of_x)))
                .unwrap(),
            LuaVal::new()
        );
        assert_eq!(vm.env_attrs[index_of_x], LuaVal::from(2));
    }
}
