use errors::LuaError;
use lua_values::{lua_closure::LuaClosure, LuaVal};
use luacompiler::bytecode::instructions::{first_arg, second_arg};
use Vm;

// R(1) = Closure(curr_function.child(R(2)).index())
pub fn closure(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    // Take the index of the function which is the child of the current function
    let index = vm
        .bytecode
        .get_function(vm.curr_func)
        .get_func_index(second_arg(instr) as usize);
    vm.registers[first_arg(instr) as usize] = LuaVal::from(LuaClosure::new(index));
    Ok(())
}
