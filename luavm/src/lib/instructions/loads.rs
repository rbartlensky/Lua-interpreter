use lua_value::*;
use luacompiler::bytecode::instructions::{first_arg, second_arg};
use Vm;

pub fn mov(vm: &mut Vm, instr: u32) {
    let i = first_arg(instr) as usize;
    let j = second_arg(instr) as usize;
    vm.registers[i] = vm.registers[j].clone();
}

pub fn ldi(vm: &mut Vm, instr: u32) {
    let val = vm.bytecode.get_int(second_arg(instr));
    vm.registers[first_arg(instr) as usize] = Box::new(LuaInt::new(val));
}

pub fn ldf(vm: &mut Vm, instr: u32) {
    let val = vm.bytecode.get_float(second_arg(instr));
    vm.registers[first_arg(instr) as usize] = Box::new(LuaFloat::new(val));
}

pub fn lds(vm: &mut Vm, instr: u32) {
    let val = vm.bytecode.get_string(second_arg(instr));
    vm.registers[first_arg(instr) as usize] = Box::new(LuaString::new(val.to_string()));
}
