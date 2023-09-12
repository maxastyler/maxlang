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
    let ts = tokeniser::Token::tokenise_source("a `do 2 3 `something other!", "")
        .map(|x| x.unwrap())
        .collect::<Vec<_>>();
    let (left, exp) = parser::parse_expression(&ts).unwrap();
    println!("LEFT: {:?}", left);
    println!("{:?}", exp);
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
