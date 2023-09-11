// use std::rc::Rc;

// use compiler::Compiler;
// use parser::parse_program;
// use vm::VM;

// use crate::{
//     expression::{Expression, Symbol},
//     value::{Closure, Function},
// };

// mod compiler;
mod expression;
// mod frame;
// mod native_function;
// mod opcode;
mod parser;
mod tokeniser;
// mod value;
// mod vm;

fn main() {
    // let (s, e) = parse_program(include_str!("./fac.maxlang")).unwrap();
    // let mut c = Compiler::new();
    // c.compile_expression(None, &e[0], true).unwrap();
    // let f = c.frame_to_function();

    // let mut vm = VM::from_bare_function(f);
    // loop {
    //     match vm.step() {
    //         Ok(Some(v)) => {
    //             println!("GOT A VALUE: {:?}", v);
    //             break;
    //         }
    //         Err(s) => {
    //             println!("{:?}", s);
    //             break;
    //         }
    //         _ => (),
    //     }
    // }
}
