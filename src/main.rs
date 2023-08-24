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
    let mut c: Compiler<256> = Compiler::new(None);
    let e: Expression = vec![
	("capd_fun", vec![("a", 3.into()).into(),
	     (vec![], "a".into()).into()
	].into()).into(),
        ("ooh", 2.into()).into(),
        ("hi", (vec!["a", "b", "c"], vec!["c".into(), "ooh".into()].into()).into()).into(),
        3.into(),
        Expression::Call(Expression::Symbol(Symbol("hi".into())).into(), vec![2.into(), 3.into(), 4.into()]),
        Expression::Call(Expression::Symbol(Symbol("capd_fun".into())).into(), vec![]),
    ]
    .into();
    println!("{:?}", e);
    c.compile_expression(e).unwrap();
    let mut v = vm::VM {
        frames: vec![Frame::new(
            Rc::new(Closure {
                function: Rc::new(Function {
                    chunk: c.chunk,
                    arity: 0,
                    registers: 256,
                }),
                upvalues: vec![],
            }),
            0,
            0,
        )],
        ..Default::default()
    };
    println!("{:?}", v);
    for _ in 0..40 {
        v.step();
        println!("CURRENT STATE OF THE VM:\n{:?}\n\n\n\n", v);
    }
}
