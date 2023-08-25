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
    let mut c: Compiler<256> = Compiler::new(None);
    let e: Expression = vec![
        ("a", 3.into()).into(),
        (
            "fun_1",
            (
                vec![],
                (vec![], Expression::Symbol(Symbol("a".into()))).into(),
            )
                .into(),
        )
            .into(),
        (
            "capd_fun",
            vec![("a", 3.into()).into(), (vec![], "a".into()).into()].into(),
        )
            .into(),
        ("ooh", 2.into()).into(),
        (
            "hi",
            (vec!["a", "b", "c"], vec!["c".into(), "ooh".into()].into()).into(),
        )
            .into(),
        3.into(),
        Expression::Call(
            Expression::Symbol(Symbol("hi".into())).into(),
            vec![2.into(), 3.into(), 4.into()],
        ),
        Expression::Call(Expression::Symbol(Symbol("capd_fun".into())).into(), vec![]),
    ]
    .into();
    println!("{:?}", e);
    let f = c.compile_expression_as_function(e).unwrap();
    println!("COMPILED");
    println!("{:?}", c);
    let mut v = vm::VM {
        frames: vec![Frame::new(
            Rc::new(Closure {
                function: f,
                upvalues: vec![],
            }),
            0,
            0,
        )],
        ..Default::default()
    };
    println!("{:?}", v);
    let v = loop {
        match v.step() {
            Ok(Some(v)) => break v,
            _ => (),
        }
    };

    println!("{:?}", v);
}
