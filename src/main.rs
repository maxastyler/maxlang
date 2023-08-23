use std::rc::Rc;

use compiler::{Compiler, Expression, Literal, Symbol};
use value::{Chunk, Value};
use vm::Frame;

use crate::value::{Closure, Function};

mod compiler;
mod opcode;
mod value;
mod vm;

fn main() {
    // let mut vm = vm::VM {
    //     frames: vec![Frame {
    //         return_position: None,
    //         pointer: 0,
    //         stack: vec![
    //             Value::Integer(0),
    //             Value::Integer(1),
    //             Value::Nil,
    //             Value::Object(value::Object::Closure(Rc::new(Closure {
    //                 function: Rc::new(Function {
    //                     chunk: Chunk {
    //                         opcodes: vec![OpCode::Dump(0, 1), OpCode::Return(1)],
    //                         constants: vec![],
    // 			    functions: vec![]
    //                     },
    //                     registers: 10,

    //                 }),
    // 		    upvalues: vec![]
    //             }))),
    //         ],
    //         function: Rc::new(Function {
    //             chunk: Chunk {
    //                 opcodes: vec![OpCode::Add(2, 0, 1), OpCode::Save(2), OpCode::Call(3, 0)],
    //                 constants: vec![],
    //             },
    //             registers: 10,
    //         }),
    //     }],
    //     temporary_storage: vec![],
    // };

    // for _ in 0..10 {
    //     vm.step();
    // }

    // println!("{:?}", vm);
    let mut c: Compiler<10> = Compiler::new(None);

    c.compile_expression(Expression::Block(
        vec![
            Expression::Assign(
                Symbol("cool_fun".into()),
                Box::new(Expression::Function(
                    vec![],
                    Box::new(Expression::Literal(Literal(2))),
                )),
            ),
            Expression::Assign(
                Symbol("other_function".into()),
                Box::new(Expression::Function(
                    vec![],
                    Box::new(Expression::Literal(Literal(3))),
                )),
            ),
            Expression::Assign(
                Symbol("Hiii".into()),
                Box::new(Expression::Symbol(Symbol("other_function".into()))),
            ),
            Expression::Assign(
                Symbol("Hiioo".into()),
                Box::new(Expression::Symbol(Symbol("other_function".into()))),
            ),
            // Expression::Literal(Literal(2))
        ],
        // vec![],
        Box::new(Expression::Assign(
            Symbol("other_function".into()),
            Box::new(Expression::Call(
                Box::new(Expression::Symbol(Symbol("cool_fun".into()))),
                vec![],
            )),
        )),
    ))
    .unwrap();
    println!("{:?}", c);
}
