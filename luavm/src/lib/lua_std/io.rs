use crate::Vm;
use errors::LuaError;
use lua_values::{lua_table::UserTable, LuaVal};
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use stdlib::StdFunction;

pub fn get_io_module() -> (String, LuaVal) {
    let io = LuaVal::from(UserTable::new(HashMap::new()));
    for func in &[("write", lua_write)] {
        let std_func = StdFunction {
            name: func.0,
            handler: func.1,
        };
        let std_val = LuaVal::from(&std_func);
        io.set_attr(LuaVal::from(func.0.to_string()), std_val)
            .unwrap();
    }
    ("io".to_string(), io)
}

pub fn lua_write(vm: &mut Vm) -> Result<(), LuaError> {
    let args_start = vm.closure.args_start();
    let args_count = vm.closure.args_count();
    let mut s = String::new();
    for i in args_start..(args_start + args_count - 1) {
        write!(s, "{}\t", &vm.stack[i]).unwrap();
    }
    println!("{}{}", s, &vm.stack[args_start + args_count - 1]);
    Ok(())
}
