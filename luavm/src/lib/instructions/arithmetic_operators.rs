use lua_value::*;
use luacompiler::bytecode::instructions::{first_arg, second_arg, third_arg};
use Vm;

/// Perform <op> on the arguments of <instr>. The arguments are converted using
/// the rules of Lua.
macro_rules! bin_op {
    ($vm:ident, $instr: ident, $op: tt) => {
        {
            let lhs = &$vm.registers[second_arg($instr) as usize];
            let rhs = &$vm.registers[third_arg($instr) as usize];
            if lhs.is_float() || rhs.is_float() {
                let float = match (lhs.to_float(), rhs.to_float()) {
                    (Some(l), Some(r)) => l $op r,
                    (_, _) => panic!("Failed to convert to float!")
                };
                Box::new(LuaFloat::new(float))
            } else {
                let int = match (lhs.to_int(), rhs.to_int()) {
                    (Some(l), Some(r)) => l $op r,
                    (_, _) => panic!("Failed to convert to int!")
                };
                Box::new(LuaInt::new(int))
            }
        }
    }
}

/// Perform <op> on the argmuents of <instr>. The arguments are always converted to
/// float.
macro_rules! float_bin_op {
    ($vm:ident, $instr: ident, $op: tt) => {
        {
            let lhs = &$vm.registers[second_arg($instr) as usize];
            let rhs = &$vm.registers[third_arg($instr) as usize];
            let float = match (lhs.to_float(), rhs.to_float()) {
                (Some(l), Some(r)) => l $op r,
                (_, _) => panic!("Failed to convert to float!")
            };
            Box::new(LuaFloat::new(float))
        }
    }
}

pub fn add(vm: &mut Vm, instr: u32) {
    let res: Box<LuaValue> = bin_op!(vm, instr, +);
    vm.registers[first_arg(instr) as usize] = res;
}

pub fn sub(vm: &mut Vm, instr: u32) {
    let res: Box<LuaValue> = bin_op!(vm, instr, -);
    vm.registers[first_arg(instr) as usize] = res;
}

pub fn mul(vm: &mut Vm, instr: u32) {
    let res: Box<LuaValue> = bin_op!(vm, instr, *);
    vm.registers[first_arg(instr) as usize] = res;
}

pub fn div(vm: &mut Vm, instr: u32) {
    let res: Box<LuaValue> = float_bin_op!(vm, instr, /);
    vm.registers[first_arg(instr) as usize] = res;
}

pub fn modulus(vm: &mut Vm, instr: u32) {
    let res: Box<LuaValue> = bin_op!(vm, instr, %);
    vm.registers[first_arg(instr) as usize] = res;
}

pub fn fdiv(vm: &mut Vm, instr: u32) {
    let res: Box<LuaValue> = {
        let lhs = &vm.registers[second_arg(instr) as usize];
        let rhs = &vm.registers[third_arg(instr) as usize];
        if lhs.is_float() || rhs.is_float() {
            let float = match (lhs.to_float(), rhs.to_float()) {
                (Some(l), Some(r)) => (l / r).floor(),
                (_, _) => panic!("Failed to convert to float!"),
            };
            Box::new(LuaFloat::new(float))
        } else {
            let int = match (lhs.to_int(), rhs.to_int()) {
                (Some(l), Some(r)) => l / r,
                (_, _) => panic!("Failed to convert to int!"),
            };
            Box::new(LuaInt::new(int))
        }
    };
    vm.registers[first_arg(instr) as usize] = res;
}

pub fn exp(vm: &mut Vm, instr: u32) {
    let res: Box<LuaValue> = {
        let lhs = &vm.registers[second_arg(instr) as usize];
        let rhs = &vm.registers[third_arg(instr) as usize];
        let float = match (lhs.to_float(), rhs.to_float()) {
            (Some(l), Some(r)) => l.powf(r),
            (_, _) => panic!("Failed to convert to float!"),
        };
        Box::new(LuaFloat::new(float))
    };
    vm.registers[first_arg(instr) as usize] = res;
}
