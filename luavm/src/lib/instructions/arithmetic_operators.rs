use errors::LuaError;
use luacompiler::bytecode::instructions::{first_arg, second_arg, third_arg};
use Vm;

/// Generates a function called `$op`, which takes two parameters: a mutable reference to a
/// vm and an instruction, and returns whether the instruction is executed succesfully
/// or not by the vm. This macro is used to generate add, sub, etc. functions which all
/// have the same implementation. The name of the function ($op) is also the name of
/// the method that is called on the operands of the instruction. For example:
/// `bin_op!(add);` generates an `add` function which extracts the arguments of the
/// instruction (lhs, and rhs), and calls `lhs.add(rhs)`.
macro_rules! bin_op {
    ($op: tt) => {
        pub fn $op(vm: &mut Vm, instr: u32) -> Result<(), LuaError> {
            let res = {
                let lhs = &vm.registers[second_arg(instr) as usize];
                let rhs = &vm.registers[third_arg(instr) as usize];
                lhs.$op(rhs)?
            };
            vm.registers[first_arg(instr) as usize] = res;
            Ok(())
        }
    };
}

bin_op!(add);
bin_op!(sub);
bin_op!(mul);
bin_op!(div);
bin_op!(modulus);
bin_op!(fdiv);
bin_op!(exp);
