use errors::LuaError;
use lua_values::LuaVal;
use luacompiler::bytecode::instructions::{first_arg, second_arg, third_arg};
use Vm;

pub fn eq(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    vm.registers[first_arg(instr) as usize] = LuaVal::from(
        &vm.registers[second_arg(instr) as usize] == &vm.registers[third_arg(instr) as usize],
    );
    Ok(())
}

pub fn lt(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    vm.registers[first_arg(instr) as usize] = LuaVal::from(
        &vm.registers[second_arg(instr) as usize] < &vm.registers[third_arg(instr) as usize],
    );
    Ok(())
}
