use errors::LuaError;
use luacompiler::bytecode::instructions::{first_arg, second_arg};
use Vm;

pub fn jmp_if(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    if !vm.registers[first_arg(instr) as usize].to_bool() {
        vm.pc += second_arg(instr) as usize;
    }
    Ok(())
}

pub fn jmp(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    vm.pc += first_arg(instr) as usize;
    Ok(())
}
