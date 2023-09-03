use std::rc::Rc;

use compiler::Compiler;
use parser::parse_program;
use vm::VM;

use crate::{
    expression::{Expression, Symbol},
    value::{Closure, Function},
};

mod compiler;
mod expression;
mod frame;
mod native_function;
mod opcode;
mod parser;
mod value;
mod vm;

fn main() {
    let (s, e) = parse_program(
        "{let x {fn fib (fib, n) {cond {n `< 2 => n, {fib fib {n `- 1}} `+ {fib fib {n `- 2}}}}}; x 2}",
    )
    .unwrap();
    let mut c = Compiler::new();
    c.compile_expression(None, &e[0], true).unwrap();
    let f = c.frame_to_function();
    println!("{:?}", f);

    let mut vm = VM::from_function(f);
    println!("{:?}", vm);
    let mut x = 0;
    loop {
        match vm.step(){
            Ok(Some(v)) => {
                println!("GOT A VALUE: {:?}", v);
                break;
            }
	    Err(s) => {println!("{:?}", s); break}
            _ => (),
        }
	if x > 100 {break}
	x += 1;
    }
}
