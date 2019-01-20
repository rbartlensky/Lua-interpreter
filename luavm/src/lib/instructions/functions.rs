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

pub fn push(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    vm.stack
        .push(vm.registers[first_arg(instr) as usize].clone());
    Ok(())
}

pub fn call(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    // The closure_index method gives us an index of the bytecode.functions vector
    // where we have to "jump" in order to find the instructions of the callee.
    let index = vm.registers[first_arg(instr) as usize].closure_index()?;
    let old_func = vm.curr_func;
    vm.curr_func = index;
    // push the first `reg_num` registers to the stack, as the function will modify these
    let reg_num = vm.bytecode.get_function(vm.curr_func).reg_count();
    for i in 1..reg_num {
        vm.stack.push(vm.registers[i].clone());
    }
    // the compiler might have pushed some arguments, but the exact number is encoded
    // in the second operand of the call instruction
    // we have to make sure that those arguments are copied where the function expects
    // its parameters to be located at
    let num_of_args = second_arg(instr) as usize;
    let mut index_of_arg = vm.stack.len() - (reg_num - 1) - num_of_args;
    let num_of_params = vm.bytecode.get_function(vm.curr_func).param_count();
    for i in 0..num_of_params {
        // if the caller didn't push enough arguments, we have to set the remaining
        // parameter registers to nil, so that we don't use some old values
        vm.registers[i + 1] = if i < num_of_args {
            vm.stack[index_of_arg].clone()
        } else {
            LuaVal::new()
        };
        index_of_arg += 1;
    }
    vm.eval();
    // restore the state of the caller
    for i in (1..reg_num).rev() {
        vm.registers[i] = vm.stack.pop().unwrap();
    }
    // pop the arguments
    for _ in 0..num_of_args {
        vm.stack.pop();
    }
    vm.curr_func = old_func;
    Ok(())
}
