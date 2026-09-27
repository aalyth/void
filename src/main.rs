use crate::{
    code::{Bytecode, CodeBuilder, CodeError, Constant, Jump, Type},
    vm::{Function, Vm},
};

mod code;
mod vm;

fn func_fib() -> Result<Vec<u8>, CodeError> {
    let mut builder = CodeBuilder::new()
        .declare_local("n".to_string(), Type::Int)?
        .get_local("n".to_string())?
        .push_constant(Constant::Int(2))?
        .jmp_if(Jump::IfLe, "Return".to_string())
        .get_local("n".to_string())?
        .push_constant(Constant::Int(1))?
        .sub(Type::Int);
    builder.push_instr(Bytecode::Call);
    builder.code.extend((0 as u16).to_le_bytes());
    builder = builder
        .get_local("n".to_string())?
        .push_constant(Constant::Int(2))?
        .sub(Type::Int);
    builder.push_instr(Bytecode::Call);
    builder.code.extend((0 as u16).to_le_bytes());
    builder
        .add(Type::Int)
        .instr_return()
        .set_label("Return".to_string())?
        .push_constant(Constant::Int(1))?
        .instr_return()
        .build()
}

fn func_main() -> Result<Vec<u8>, CodeError> {
    let mut builder = CodeBuilder::new().push_constant(Constant::Int(35))?;
    builder.push_instr(Bytecode::Call);
    builder.code.extend((0 as u16).to_le_bytes());
    builder.build()
}

fn main() {
    // def fib(n: int) -> int:
    //     if n > 2:
    //         return fib(n-1) + fib(n-2)
    //     return 1
    // if __name__ == '__main__':
    //     print(fib(35))
    let mut vm = Vm::new();
    vm.add_constant(Constant::Int(2));
    vm.add_constant(Constant::Int(1));
    vm.add_constant(Constant::Int(35));

    let fib = func_fib().unwrap();
    println!("{:#?}", fib);

    vm.add_function(Function {
        arity: 1,
        locals: 0,
        code: fib,
    });
    vm.add_function(Function {
        arity: 0,
        locals: 0,
        code: vec![1, 2, 8, 0, 0],
    });

    vm.execute(1);

    println!("{:#?}", vm.stack);
    println!("Hello, world!");
}
