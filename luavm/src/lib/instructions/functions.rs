use errors::LuaError;
use lua_values::LuaVal;
use luacompiler::bytecode::instructions::{first_arg, second_arg};
use Vm;

// R(1) = Closure(curr_function.child(R(2)).index())
pub fn closure(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    // Take the index of the function which is the child of the current function
    let index = vm
        .bytecode
        .get_function(vm.closure.index())
        .get_func_index(second_arg(instr) as usize);
    vm.registers[first_arg(instr) as usize] = LuaVal::from(vm.bytecode.get_function(index));
    Ok(())
}

pub fn push(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    // push all the variable arguments of the current function to the stack
    if second_arg(instr) == 1 {
        let args_count = vm.closure.args_count();
        let args_start = vm.closure.args_start();
        // make sure to skip the arguments which are the actual parameters
        for i in (args_start + vm.closure.param_count())..(args_start + args_count) {
            let val = vm.stack[i].clone();
            vm.stack.push(val);
        }
    } else {
        vm.stack
            .push(vm.registers[first_arg(instr) as usize].clone());
    }
    Ok(())
}

pub fn call(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    let old_closure = vm.closure.clone();
    vm.closure = vm.registers[first_arg(instr) as usize].get_closure()?;
    // push the first `reg_num` registers to the stack, as the function will modify these
    let reg_num = vm.closure.reg_count();
    for i in 1..reg_num {
        vm.stack.push(vm.registers[i].clone());
    }
    // the compiler might have pushed some arguments, but the exact number is encoded
    // in the second operand of the call instruction
    // we have to make sure that those arguments are copied where the function expects
    // its parameters to be located at
    let num_of_args = second_arg(instr) as usize;
    let mut index_of_arg = vm.stack.len() - (reg_num - 1) - num_of_args;
    vm.closure.set_args_count(num_of_args);
    vm.closure.set_args_start(index_of_arg);
    let num_of_params = vm.closure.param_count();
    // copy arguments into registers [R(1)..R(num_of_params)]
    for i in 0..num_of_params {
        // if the caller didn't push enough arguments, we have to set the remaining
        // parameter registers to nil, so that we don't use some value from the old frame
        vm.registers[i + 1] = if i < num_of_args {
            vm.stack[index_of_arg].clone()
        } else {
            LuaVal::new()
        };
        index_of_arg += 1;
    }
    vm.closure.clone().call(vm);
    // restore the state of the caller
    for i in (1..reg_num).rev() {
        vm.registers[i] = vm.stack.pop().unwrap();
    }
    // pop the arguments
    for _ in 0..num_of_args {
        vm.stack.pop();
    }
    vm.closure = old_closure;
    Ok(())
}

pub fn vararg(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    let param_count = vm.bytecode.get_function(vm.closure.index()).param_count();
    // where they start on the stack
    let args_start = vm.closure.args_start();
    let mut var_args_start = args_start + param_count;
    let var_args_end = args_start + vm.closure.args_count();
    // The first register which receives a cloned value from varargs
    let start_reg = first_arg(instr) as usize;
    // second operand tells us how many registers we have to assign
    for r in start_reg..(start_reg + second_arg(instr) as usize) {
        // in the case where we don't have enough arguments to unpack generate Nils
        vm.registers[r] = if var_args_start < var_args_end {
            vm.stack[var_args_start].clone()
        } else {
            LuaVal::new()
        };
        var_args_start += 1;
    }
    Ok(())
}
