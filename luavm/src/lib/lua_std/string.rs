use crate::Vm;
use errors::LuaError;
use lua_values::{lua_table::UserTable, LuaVal};
use std::collections::HashMap;
use stdlib::StdFunction;

pub fn get_string_module() -> (String, LuaVal) {
    let string = LuaVal::from(UserTable::new(HashMap::new()));
    for func in &[("format", lua_format)] {
        let std_func = StdFunction {
            name: func.0,
            handler: func.1,
        };
        let std_val = LuaVal::from(&std_func);
        string
            .set_attr(LuaVal::from(func.0.to_string()), std_val)
            .unwrap();
    }
    ("string".to_string(), string)
}

pub fn lua_format(vm: &mut Vm) -> Result<(), LuaError> {
    let args_start = vm.stack_frames.last().unwrap().top;
    let args_count = vm.top - args_start;
    if args_count < 1 {
        Err(LuaError::Error(
            "Expected at least an argument, which is a string.".to_string(),
        ))
    } else {
        let mut fmt_string = vm.stack[args_start].to_string()?;
        // XXX: This is VERY slow! This function is implemented only for the sake
        // of running a few luajit benchmarks.
        for arg in vm.stack[(args_start + 1)..(args_start + args_count)].iter() {
            fmt_string = fmt_string.replacen("%d", arg.to_string()?.as_ref(), 1);
        }
        vm.push(LuaVal::from(fmt_string));
        vm.closure().set_ret_vals(1);
        Ok(())
    }
}
