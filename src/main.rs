use std::rc::Rc;

use value::{Chunk, Value};
use vm::{Frame, OpCode};

use crate::value::{Closure, Function};

mod compiler;
mod value;
mod vm;

fn main() {
    let mut vm = vm::VM {
        frames: vec![Frame {
            return_position: None,
            pointer: 0,
            stack: vec![
                Value::Integer(0),
                Value::Integer(1),
                Value::Nil,
                Value::Object(value::Object::Closure(Rc::new(Closure {
                    function: Rc::new(Function {
                        chunk: Chunk {
                            opcodes: vec![OpCode::Dump(0, 1), OpCode::Return(1)],
                            constants: vec![],
                        },
                        registers: 10,
                    }),
                }))),
            ],
            function: Rc::new(Function {
                chunk: Chunk {
                    opcodes: vec![OpCode::Add(2, 0, 1), OpCode::Save(2), OpCode::Call(3, 0)],
                    constants: vec![],
                },
                registers: 10,
            }),
        }],
        temporary_storage: vec![],
    };

    for _ in 0..10 {
        vm.step();
    }

    println!("{:?}", vm);
}
