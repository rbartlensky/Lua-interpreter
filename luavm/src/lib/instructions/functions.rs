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
            vm.push(val);
        }
    } else {
        let val = vm.registers[first_arg(instr) as usize].clone();
        vm.push(val);
    }
    Ok(())
}

pub fn call(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    let old_closure = vm.closure.clone();
    vm.closure = vm.registers[first_arg(instr) as usize].get_closure()?;
    // push the first `reg_count` registers to the stack, as the function will modify these
    let reg_count = vm.closure.reg_count();
    for i in 1..reg_count {
        let reg = vm.registers[i].clone();
        vm.push(reg);
    }
    // the compiler might have pushed some arguments, but the exact number is encoded
    // in the second operand of the call instruction
    // we have to make sure that those arguments are copied where the function expects
    // its parameters to be located at
    let args_count = second_arg(instr) as usize;
    let mut index_of_arg = vm.stack.len() - (reg_count - 1) - args_count;
    vm.closure.set_args_count(args_count);
    vm.closure.set_args_start(index_of_arg);
    let param_count = vm.closure.param_count();
    // copy arguments into registers [R(1)..R(param_count)]
    for i in 0..param_count {
        // if the caller didn't push enough arguments, we have to set the remaining
        // parameter registers to nil, so that we don't use some value from the old frame
        vm.registers[i + 1] = if i < args_count {
            vm.stack[index_of_arg].clone()
        } else {
            LuaVal::new()
        };
        index_of_arg += 1;
    }
    vm.closure.clone().call(vm);
    // restore the registers
    for (reg, i) in ((vm.top - (reg_count - 1))..(vm.top)).enumerate() {
        std::mem::swap(&mut vm.registers[reg + 1], &mut vm.stack[i]);
    }
    vm.top -= args_count + (reg_count - 1);
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
