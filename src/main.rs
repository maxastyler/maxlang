use crate::compiler::Compiler;

mod compiler;
mod expression;
mod frame;
mod native_function;
mod opcode;
mod parser;
mod tokeniser;
mod value;
mod vm;

fn main() {
    let ts = tokeniser::Token::tokenise_source("(cond {2 ~ 3; else 2}; 3)", "")
        .map(|x| x.unwrap())
        .collect::<Vec<_>>();
    let (_, exp) = parser::parse_expression(&ts).unwrap();
    let mut c = Compiler::new();
    c.compile_expression(None, &exp, true).unwrap();
    let f = c.frame_to_function();
    println!("{:?}", f);
}

// fn main() {
//     let (s, e) = parse_program(include_str!("./fac.maxlang")).unwrap();
//     let mut c = Compiler::new();
//     c.compile_expression(None, &e[0], true).unwrap();
//     let f = c.frame_to_function();

//     let mut vm = VM::from_bare_function(f);
//     loop {
//         match vm.step() {
//             Ok(Some(v)) => {
//                 println!("GOT A VALUE: {:?}", v);
//                 break;
//             }
//             Err(s) => {
//                 println!("{:?}", s);
//                 break;
//             }
//             _ => (),
//         }
//     }
// }
