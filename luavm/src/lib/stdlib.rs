use super::errors::LuaError;
use super::Vm;
use lua_values::LuaVal;
use std::fmt::Write as FmtWrite;

pub const STDLIB_FUNCS: &'static [StdFunction] = &[
    StdFunction {
        name: "assert",
        handler: lua_assert,
    },
    StdFunction {
        name: "print",
        handler: lua_print,
    },
    StdFunction {
        name: "tonumber",
        handler: lua_tonumber,
    },
];

pub struct StdFunction {
    pub name: &'static str,
    pub handler: fn(&mut Vm) -> Result<(), LuaError>,
}

impl StdFunction {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn handler(&self) -> fn(&mut Vm) -> Result<(), LuaError> {
        self.handler.clone()
    }
}

pub fn lua_print(vm: &mut Vm) -> Result<(), LuaError> {
    let args_start = vm.closure.args_start();
    let args_count = vm.closure.args_count();
    let mut s = String::new();
    for i in args_start..(args_start + args_count - 1) {
        write!(s, "{}\t", &vm.stack[i]).unwrap();
    }
    println!("{}{}", s, &vm.stack[args_start + args_count - 1]);
    Ok(())
}

pub fn lua_assert(vm: &mut Vm) -> Result<(), LuaError> {
    let args_start = vm.closure.args_start();
    let args_count = vm.closure.args_count();
    if args_count == 0 {
        return Err(LuaError::Error(
            "assert expects at least one argument!".to_string(),
        ));
    }
    if vm.stack[args_start].to_bool() {
        for i in args_start..(args_start + args_count) {
            let val = vm.stack[i].clone();
            vm.stack.push(val);
        }
        vm.closure.set_ret_vals(args_count);
        Ok(())
    } else {
        let message = if args_count > 1 {
            vm.stack[args_start + args_count].to_string()?
        } else {
            "assertion failed!".to_string()
        };
        Err(LuaError::Error(message))
    }
}

pub fn lua_tonumber(vm: &mut Vm) -> Result<(), LuaError> {
    let args_start = vm.closure.args_start();
    let args_count = vm.closure.args_count();
    if args_count == 0 {
        Err(LuaError::Error(
            "tonumber expects at least one argument!".to_string(),
        ))
    } else if args_count == 1 {
        let val = vm.stack[args_start].clone();
        if val.is_number() {
            vm.stack.push(val);
            vm.closure.set_ret_vals(1);
            Ok(())
        } else {
            if let Ok(num) = val.to_int() {
                vm.stack.push(LuaVal::from(num));
            } else if let Ok(num) = val.to_float() {
                vm.stack.push(LuaVal::from(num));
            } else {
                vm.stack.push(LuaVal::new());
            }
            vm.closure.set_ret_vals(1);
            Ok(())
        }
    } else {
        unimplemented!("tonumber with two+ arguments")
    }
}
