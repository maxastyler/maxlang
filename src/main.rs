use crate::{compiler::Compiler, vm::VM};

mod compiler;
mod expression;
mod frame;
mod heap;
mod native_function;
mod opcode;
mod parser;
mod tokeniser;
mod value;
mod vm;
mod memory;

fn main() {
    let src = include_str!("programs/hello_world.maxlang");
    let ts = tokeniser::Token::tokenise_source(src, "")
        .map(|x| x.unwrap())
        .collect::<Vec<_>>();
    let (_, exp) = parser::parse_expression(&ts).unwrap();
    let mut c = Compiler::new();
    c.compile_expression(None, &exp, true).unwrap();
    let f = c.frame_to_function();
    let mut vm = VM::from_bare_function(f);
    loop {
        // println!("{:?}", vm);
        match vm.step() {
            Ok(Some(v)) => {
                println!("GOT A VALUE: {:?}", v);
                break;
            }
            Err(s) => {
                println!("{:?}", s);
                break;
            }
            _ => (),
        }
    }
}

// fn main() {
//     let (s, e) = parse_program(include_str!("./fac.maxlang")).unwrap();
//     let mut c = Compiler::new();
//     c.compile_expression(None, &e[0], true).unwrap();
//     let f = c.frame_to_function();

//     let mut vm = VM::from_bare_function(f);

// }
