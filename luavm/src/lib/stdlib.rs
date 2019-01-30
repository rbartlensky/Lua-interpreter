use super::Vm;
use std::fmt::Write as FmtWrite;

pub const STDLIB_FUNCS: &'static [StdFunction] = &[StdFunction {
    name: "print",
    handler: lua_print,
    param_count: 0,
}];

pub struct StdFunction {
    name: &'static str,
    handler: fn(&mut Vm),
    param_count: usize,
}

impl StdFunction {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn handler(&self) -> fn(&mut Vm) {
        self.handler.clone()
    }

    pub fn param_count(&self) -> usize {
        self.param_count
    }
}

pub fn lua_print(vm: &mut Vm) {
    let args_start = vm.closure.args_start();
    let args_count = vm.closure.args_count();
    let mut s = String::new();
    for i in args_start..(args_start + args_count - 1) {
        write!(s, "{}\t", &vm.stack[i]).unwrap();
    }
    println!("{}{}", s, &vm.stack[args_start + args_count - 1]);
}
