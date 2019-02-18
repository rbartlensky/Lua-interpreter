use errors::LuaError;
use luacompiler::bytecode::instructions::{extended_arg, first_arg};
use Vm;

pub fn jmp_if(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    if !vm.registers[first_arg(instr) as usize].to_bool() {
        let jmp: isize = vm.pc as isize + extended_arg(instr) as isize;
        vm.pc = jmp as usize;
    }
    Ok(())
}

pub fn jmp(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    let jmp: isize = vm.pc as isize + extended_arg(instr) as isize;
    vm.pc = jmp as usize;
    Ok(())
}
