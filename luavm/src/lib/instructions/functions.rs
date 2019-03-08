use errors::LuaError;
use lua_values::{lua_closure::UserFunction, LuaVal};
use luacompiler::bytecode::instructions::{first_arg, second_arg, *};
use luacompiler::bytecode::BCProviderType;
use std::mem::swap;
use StackFrame;
use Vm;

// R(1) = Closure(curr_function.child(R(2)).index())
pub fn closure(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    // Take the index of the function which is the child of the current function
    let func = vm.bytecode.get_function(second_arg(instr) as usize);
    let upvals_len = func.upvals_count();
    let mut upvals = Vec::with_capacity(upvals_len + 1);
    upvals.push(vm.env.clone());
    for _ in 0..upvals_len {
        upvals.push(LuaVal::new());
    }

    let caller_index = vm.stack_frames[vm.curr_frame].closure.index()?;
    if let Some(provides) = vm
        .bytecode
        .get_function(caller_index)
        .provides()
        .get(&(func.index() as u8))
    {
        for (provider, upval) in provides.iter() {
            match provider {
                BCProviderType::Reg(r) => {
                    upvals[*upval as usize] = vm.registers[*r as usize].clone()
                }
                BCProviderType::Upval(u) => {
                    upvals[*upval as usize] = vm.closure().get_upval(*u as usize)?
                }
            };
        }
    }
    let ufunc = UserFunction::new(func.index(), func.reg_count(), func.param_count(), upvals);
    vm.registers[first_arg(instr) as usize] = LuaVal::from(ufunc);
    Ok(())
}

pub fn push(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    let val = vm.registers[first_arg(instr) as usize].clone();
    vm.push(val);
    vm.closure().inc_ret_vals(third_arg(instr) as usize)?;
    Ok(())
}

pub fn set_top(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    let closure = vm.registers[first_arg(instr) as usize].clone();
    vm.stack_frames.push(StackFrame {
        closure,
        top: vm.top,
    });
    Ok(())
}

const MOVR_0_0_1: u32 = make_instr(Opcode::MOVR, 0, 0, 1);
const MOVR_0_0_2: u32 = make_instr(Opcode::MOVR, 0, 0, 2);

pub fn call(vm: &mut Vm, _instr: u32) -> Result<(), LuaError> {
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
    let old_pc = vm.pc;
    // update the current frame to the last one
    let old_curr_frame = vm.curr_frame;
    let caller_index = vm.closure().index()?;
    let args_start = vm.stack_frames.last().unwrap().top;
    let args_count = vm.top - args_start;
    vm.curr_frame = vm.stack_frames.len() - 1;
    // update upvals of the callee
    let callee_index = vm.closure().index()?;
    set_upvals(vm, caller_index, callee_index, old_curr_frame)?;
    // push the first `reg_count` registers to the stack, as the called function
    // will modify these
    for i in 0..vm.closure().reg_count()? {
        let mut nil = LuaVal::new();
        swap(&mut nil, &mut vm.registers[i]);
        vm.push(nil);
    }
    // prepare to move arguments into registers; the callee expects the parameters in its
    // first N registers (excluding 0 which is _ENV), where N is the number of parameters
    let mut index_of_arg = args_start;
    // copy arguments into registers [R(1)..R(param_count)]
    for i in 0..vm.closure().param_count()? {
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
    vm.closure().clone().call(vm)?;
    // the called function might have pushed some return values; the exact number is
    // encoded by <ret_vals>
    let ret_vals = vm.closure().ret_vals()?;
    // restore the registers of the caller
    for (reg, i) in ((args_start + args_count)..(vm.top - ret_vals)).enumerate() {
        swap(&mut vm.registers[reg], &mut vm.stack[i]);
    }
    // restore upvals
    pop_upvals(vm, caller_index, callee_index, old_curr_frame)?;
    // restore the state of the caller
    vm.closure().set_ret_vals(0)?;
    vm.stack_frames.pop();
    vm.curr_frame = old_curr_frame;
    vm.pc = old_pc;
    // if we returned values, then the next few instructions might move these into
    // registers using the MOVR instruction
    let len = vm.bytecode.get_function(caller_index).instrs_len();
    if vm.pc + 1 < len {
        let mut instr = vm.bytecode.get_function(caller_index).get_instr(vm.pc + 1);
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
                vm.closure().inc_ret_vals(ret_vals)?;
            }
        } else {
            while opcode(instr) == Opcode::MOVR as u8 {
                let from = second_arg(instr) as usize;
                // if we don't have enough return values to unpack, we return nils
                vm.registers[first_arg(instr) as usize] = if from < ret_vals {
                    let mut nil = LuaVal::new();
                    swap(&mut nil, &mut vm.stack[vm.top - (ret_vals - from)]);
                    nil
                } else {
                    LuaVal::new()
                };
                vm.pc += 1;
                if vm.pc + 1 < len {
                    instr = vm.bytecode.get_function(caller_index).get_instr(vm.pc + 1);
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

#[inline(always)]
fn set_upvals(
    vm: &Vm,
    caller_index: usize,
    callee_index: usize,
    old_curr_frame: usize,
) -> Result<(), LuaError> {
    if let Some(upvals) = vm
        .bytecode
        .get_function(caller_index)
        .provides()
        .get(&(callee_index as u8))
    {
        for (provider, upval) in upvals.iter() {
            match provider {
                BCProviderType::Reg(r) => vm
                    .closure()
                    .set_upval(*upval as usize, vm.registers[*r as usize].clone())?,
                BCProviderType::Upval(u) => vm.closure().set_upval(
                    *upval as usize,
                    vm.stack_frames[old_curr_frame]
                        .closure
                        .get_upval(*u as usize)?,
                )?,
            };
        }
    }
    Ok(())
}

#[inline(always)]
fn pop_upvals(
    vm: &mut Vm,
    caller_index: usize,
    callee_index: usize,
    old_curr_frame: usize,
) -> Result<(), LuaError> {
    if let Some(upvals) = vm
        .bytecode
        .get_function(caller_index)
        .provides()
        .get(&(callee_index as u8))
    {
        for (provider, upval) in upvals.iter() {
            match provider {
                BCProviderType::Reg(r) => {
                    vm.registers[*r as usize] = vm.closure().get_upval(*upval as usize)?
                }
                BCProviderType::Upval(u) => vm.stack_frames[old_curr_frame]
                    .closure
                    .set_upval(*u as usize, vm.closure().get_upval(*upval as usize)?)?,
            };
        }
    }
    Ok(())
}

pub fn vararg(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
    if third_arg(instr) > 0 {
        let (args_start, args_count, param_count) = {
            let curr_frame = &vm.stack_frames[vm.curr_frame];
            let start = curr_frame.top;
            (
                start,
                vm.top - start - curr_frame.closure.reg_count()?,
                curr_frame.closure.param_count()?,
            )
        };
        // make sure to skip the arguments which are the actual parameters
        for i in (args_start + param_count)..(args_start + args_count) {
            let val = vm.stack[i].clone();
            vm.push(val);
        }
        if third_arg(instr) > 1 {
            let curr_frame = &mut vm.stack_frames[vm.curr_frame];
            curr_frame.closure.inc_ret_vals(args_count - param_count)?;
        }
    } else {
        let curr_frame = &vm.stack_frames[vm.curr_frame];
        let args_start = curr_frame.top;
        let var_args_start = args_start + curr_frame.closure.param_count()?;
        let from = second_arg(instr) as usize;
        let args_count = vm.top - args_start - curr_frame.closure.reg_count()?;
        // if we don't have enough varargs to unpack, we return nils
        vm.registers[first_arg(instr) as usize] = if var_args_start + from < args_start + args_count
        {
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
    let index = vm.closure().index()?;
    let len = vm.bytecode.get_function(index).instrs_len();
    vm.pc = len;
    Ok(())
}
