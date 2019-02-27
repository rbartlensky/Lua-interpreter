use errors::LuaError;
use lua_values::LuaVal;
use luacompiler::bytecode::instructions::{first_arg, second_arg, third_arg};
use Vm;

macro_rules! rel_op {
    ($name: tt, $op: tt) => {
        pub fn $name(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
            let res = {
                let lhs = &vm.registers[second_arg(instr) as usize];
                let rhs = &vm.registers[third_arg(instr) as usize];
                LuaVal::from(lhs $op rhs)
            };
            vm.registers[first_arg(instr) as usize] = res;
            Ok(())
        }
    };
}

rel_op!(eq, ==);
rel_op!(lt, <);
rel_op!(gt, >);
rel_op!(le, <=);
rel_op!(ge, >=);
rel_op!(ne, !=);
