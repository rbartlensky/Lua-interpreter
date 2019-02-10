use errors::LuaError;
use luacompiler::bytecode::instructions::{first_arg, second_arg, third_arg};
use Vm;

/// Up(1) = R(2)[R(3)]
pub fn get_up_attr(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    let val = {
        let arg2 = second_arg(instr) as usize;
        let from = vm.closure.get_upval(arg2)?;
        let attr = &vm.registers[third_arg(instr) as usize];
        from.get_attr(attr)?
    };
    vm.registers[first_arg(instr) as usize] = val;
    Ok(())
}

/// Up(1)[R(2)] = R(3)
pub fn set_up_attr(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    let attr = vm.registers[second_arg(instr) as usize].clone();
    let val = vm.registers[third_arg(instr) as usize].clone();
    let arg1 = first_arg(instr) as usize;
    vm.closure.get_upval(arg1)?.set_attr(attr, val)?;
    Ok(())
}
