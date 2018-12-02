use errors::LuaError;
use lua_values::LuaVal;
use luacompiler::bytecode::instructions::{first_arg, second_arg};
use Vm;

pub fn mov(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    let i = first_arg(instr) as usize;
    let j = second_arg(instr) as usize;
    vm.registers[i] = vm.registers[j].clone();
    Ok(())
}

pub fn ldi(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    let val = vm.bytecode.get_int(second_arg(instr));
    vm.registers[first_arg(instr) as usize] = LuaVal::from(val);
    Ok(())
}

pub fn ldf(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    let val = vm.bytecode.get_float(second_arg(instr));
    vm.registers[first_arg(instr) as usize] = LuaVal::from(val);
    Ok(())
}

pub fn lds(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    let arg2 = second_arg(instr);
    let val = vm.bytecode.get_string(arg2);
    // we also want to save the index of the string in the constant table in order to
    // speed up lookups in _ENV
    vm.registers[first_arg(instr) as usize] = LuaVal::from((val.to_string(), arg2 as usize));
    Ok(())
}
