use errors::LuaError;
use lua_values::{lua_closure::UserFunction, LuaVal};
use luacompiler::bytecode::instructions::*;
use luacompiler::bytecode::instructions::{first_arg, second_arg};
use Vm;

// R(1) = Closure(curr_function.child(R(2)).index())
pub fn closure(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    // Take the index of the function which is the child of the current function
    let func = vm.bytecode.get_function(second_arg(instr) as usize);
    let ufunc = UserFunction::new(
        func.index(),
        func.reg_count(),
        func.param_count(),
        vec![vm.env.clone()],
    );
    vm.registers[first_arg(instr) as usize] = LuaVal::from(ufunc);
    Ok(())
}

pub fn push(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    let val = vm.registers[first_arg(instr) as usize].clone();
    vm.push(val);
    let ret_val = vm.closure.ret_vals();
    vm.closure.set_ret_vals(ret_val + third_arg(instr) as usize);
    Ok(())
}

pub fn set_top(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    let closure = vm.registers[first_arg(instr) as usize].get_closure()?;
    closure.set_args_start(vm.top);
    Ok(())
}

const MOVR_0_0_1: u32 = make_instr(Opcode::MOVR, 0, 0, 1);
const MOVR_0_0_2: u32 = make_instr(Opcode::MOVR, 0, 0, 2);

pub fn call(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    // The frame of a function has the following structure:
    // arg1       <------ closure.args_start()
    // arg2
    // ---------- <CALL> happens here, the caller pushes the arguments for the callee
    //            vm.top is here when <CALL> is processed
    // saved-reg1
    // saved-reg2
    // ---------- callee saves the registers which it is going to clobber
    // ret-val1
    // ret-val2
    // ---------- vm.top is here when the callee is ready to return

    // save the state of the caller
    let old_closure = vm.closure.clone();
    let old_pc = vm.pc;
    vm.closure = vm.registers[first_arg(instr) as usize].get_closure()?;
    let args_start = vm.closure.args_start();
    let args_count = vm.top - args_start;
    vm.closure.set_args_count(args_count);
    // push the first `reg_count` registers to the stack, as the called function
    // will modify these
    for i in 0..vm.closure.reg_count() {
        let reg = vm.registers[i].clone();
        vm.push(reg);
    }
    // prepare to move arguments into registers; the callee expects the parameters in its
    // first N registers (excluding 0 which is _ENV), where N is the number of parameters
    let mut index_of_arg = args_start;
    // copy arguments into registers [R(1)..R(param_count)]
    for i in 0..vm.closure.param_count() {
        // if the caller didn't push enough arguments, we have to set the remaining
        // parameter registers to nil, so that we don't use some value from the old frame
        vm.registers[i] = if i < args_count {
            vm.stack[index_of_arg].clone()
        } else {
            LuaVal::new()
        };
        index_of_arg += 1;
    }
    // jump to the called function
    vm.closure.clone().call(vm)?;
    // the called function might have pushed some return values; the exact number is
    // encoded by <ret_vals>
    let ret_vals = vm.closure.ret_vals();
    // restore the registers of the caller
    for (reg, i) in ((args_start + args_count)..(vm.top - ret_vals)).enumerate() {
        std::mem::swap(&mut vm.registers[reg], &mut vm.stack[i]);
    }
    // restore the state of the caller
    vm.closure.set_ret_vals(0);
    vm.closure = old_closure;
    vm.pc = old_pc;
    // if we returned values, then the next few instructions might move these into
    // registers using the MOVR instruction
    let len = vm.bytecode.get_function(vm.closure.index()).instrs_len();
    if ret_vals > 0 && vm.pc + 1 < len {
        let mut instr = vm
            .bytecode
            .get_function(vm.closure.index())
            .get_instr(vm.pc + 1);
        // special MOVR cases, see luacompiler/bytecode/instructions.rs
        // 001 is used to push all return values to the stack as arguments to another call
        // 002 is used to push all return values to the stack as return values
        if instr == MOVR_0_0_1 || instr == MOVR_0_0_2 {
            // We are going to destroy this stackframe, so we might just as well copy
            // our return values to where we expect them to be when we call the next
            // function
            for (i, r) in ((vm.top - ret_vals)..vm.top).enumerate() {
                vm.stack.swap(r, args_start + i);
            }
            vm.pc += 1;
            vm.top = args_start + ret_vals;
            // if we are returning values, then the closure's ret_vals counter should be
            // updated as well
            if third_arg(instr) > 1 {
                let curr_ret_vals = vm.closure.ret_vals();
                vm.closure.set_ret_vals(curr_ret_vals + ret_vals);
            }
        } else {
            while opcode(instr) == Opcode::MOVR as u8 {
                let from = second_arg(instr) as usize;
                // if we don't have enough return values to unpack, we return nils
                vm.registers[first_arg(instr) as usize] = if from < ret_vals {
                    vm.stack[vm.top - (ret_vals - from)].clone()
                } else {
                    LuaVal::new()
                };
                vm.pc += 1;
                if vm.pc + 1 < len {
                    instr = vm
                        .bytecode
                        .get_function(vm.closure.index())
                        .get_instr(vm.pc + 1);
                } else {
                    break;
                }
            }
            // "destroy" our stack frame
            vm.top = args_start;
        }
    } else {
        vm.top = args_start;
    }
    Ok(())
}

pub fn vararg(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    if third_arg(instr) > 0 {
        let args_count = vm.closure.args_count();
        let args_start = vm.closure.args_start();
        // make sure to skip the arguments which are the actual parameters
        for i in (args_start + vm.closure.param_count())..(args_start + args_count) {
            let val = vm.stack[i].clone();
            vm.push(val);
        }
        if third_arg(instr) > 1 {
            let ret_val = vm.closure.ret_vals();
            vm.closure
                .set_ret_vals(ret_val + args_count - vm.closure.param_count());
        }
    } else {
        let var_args_start = vm.closure.args_start() + vm.closure.param_count();
        let from = second_arg(instr) as usize;
        // if we don't have enough varargs to unpack, we return nils
        vm.registers[first_arg(instr) as usize] =
            if var_args_start + from < vm.closure.args_start() + vm.closure.args_count() {
                vm.stack[var_args_start + from].clone()
            } else {
                LuaVal::new()
            };
    }
    Ok(())
}

pub fn movr(_vm: &mut Vm, _instr: u32) -> Result<(), LuaError> {
    panic!("This should be handled by <call>.")
}

pub fn ret(vm: &mut Vm, _instr: u32) -> Result<(), LuaError> {
    let index = vm.closure.index();
    let len = vm.bytecode.get_function(index).instrs_len();
    vm.pc = len;
    Ok(())
}
