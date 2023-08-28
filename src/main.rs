use std::rc::Rc;

use compiler::Compiler;
use value::{Chunk, Value};
use vm::Frame;

use crate::{
    expression::{Expression, Symbol},
    value::{Closure, Function},
};

mod compiler;
mod expression;
mod native_function;
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
    let e: Expression = Expression::Condition(
        vec![
            (("woah", false.into()).into(), true.into()),
            (true.into(), Expression::Symbol(Symbol("woah".into()))),
        ],
        Box::new(2.into()),
    );
    let e: Expression = Expression::Call(
        Expression::Symbol(Symbol("+".into())).into(),
        vec![2.into(), 3.into()],
    )
    .into();
    let fn_body: Expression = Expression::Condition(
        vec![(("<".into(), vec!["n".into(), 2.into()]).into(), "n".into())],
        Box::new(
            (
                "+".into(),
                vec![
                    (
                        "fib".into(),
                        vec![
                            "fib".into(),
                            ("-".into(), vec!["n".into(), 2.into()]).into(),
                        ],
                    )
                        .into(),
                    (
                        "fib".into(),
                        vec![
                            "fib".into(),
                            ("-".into(), vec!["n".into(), 1.into()]).into(),
                        ],
                    )
                        .into(),
                ],
            )
                .into(),
        ),
    );
    let fib: Expression = ("fib", (vec!["fib", "n"], fn_body.into()).into()).into();
    let f = c
        .compile_expression_as_function(
            vec![fib, ("fib".into(), vec!["fib".into(), 35.into()]).into()].into(),
        )
        .unwrap();
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
    let res = loop {
        match v.step() {
            Ok(Some(v)) => break v,
            Err(e) => panic!("{:?}", e),
            _ => (),
        }
    };

    println!("{:?}", v);
    println!("{:?}", res);
}
