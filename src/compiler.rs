use anyhow::{Context, Result};
use std::{ops::Index, rc::Rc};

use crate::{
    expression::{Expression, Literal, Symbol},
    opcode::OpCode,
    value::{Function, Value},
};

#[derive(PartialEq, Debug)]
pub struct Named {
    pub name: Symbol,
    pub depth: usize,
}

#[derive(PartialEq, Debug)]
pub enum Local {
    /// A named register, which contains a value which should not be reused
    Named(Named),
    /// A register which contains a value, which should not be reused
    Reserved,
    /// A register which contains a value, which can be reused
    ToClear,
    /// A register which doesn't contain a value
    None,
}

#[derive(PartialEq, Debug, Clone)]
pub enum FrameIndex {
    LocalIndex(usize),
    CaptureIndex(usize),
}

#[derive(Debug)]
pub struct CompilerFrame {
    pub locals: Vec<Local>,
    pub captures: Vec<FrameIndex>,
    pub depth: usize,
    pub opcodes: Vec<OpCode<usize, FrameIndex>>,
    pub constants: Vec<Value>,
    pub functions: Vec<Rc<Function>>,
}

impl CompilerFrame {
    fn new(arguments: Vec<Symbol>, depth: usize) -> Self {
        CompilerFrame {
            locals: arguments
                .into_iter()
                .map(|s| Local::Named(Named { name: s, depth }))
                .collect(),
            captures: vec![],
            depth: depth + 1,
            opcodes: vec![],
            constants: vec![],
            functions: vec![],
        }
    }

    /// Finds the symbol with the same name at the greatest depth <= self.depth,
    fn find_local(&self, symbol: &Symbol) -> Option<FrameIndex> {
        self.locals
            .iter()
            .enumerate()
            .filter_map(|(i, l)| match l {
                Local::Named(n @ Named { name, depth })
                    if *name == *symbol && *depth <= self.depth =>
                {
                    Some((i, n))
                }
                _ => None,
            })
            .max_by_key(|(_, l)| l.depth)
            .map(|(i, _)| FrameIndex::LocalIndex(i))
    }

    /// If the capture index already exists in captures, return its position,
    /// otherwise create a new one
    fn resolve_capture(&mut self, index: FrameIndex) -> FrameIndex {
        FrameIndex::CaptureIndex(
            self.captures
                .iter()
                .position(|c| *c == index)
                .unwrap_or_else(|| {
                    self.captures.push(index);
                    self.captures.len() - 1
                }),
        )
    }

    fn reserve_next_free_register(&mut self) -> Option<(usize, &mut Local)> {
        self.locals
            .iter_mut()
            .enumerate()
            .find(|(_, l)| {
                if matches!(l, Local::None) {
                    **l = Local::Reserved;
                    true
                } else {
                    false
                }
            })
            .or_else(|| {
                self.locals.push(Local::Reserved);
                self.locals.last_mut().map(|l| (self.locals.len() - 1, l))
            })
    }

    /// Remove all symbols from the current scope (depth == self.depth)
    fn clear_scope_of_symbol(&mut self, symbol: &Symbol) {
        self.locals
            .iter_mut()
            .enumerate()
            .filter(|(_, l)| {
                matches!(l,Local::Named(Named { name, depth }) if name == symbol
				      && *depth == self.depth)
            })
            .for_each(|(i, l)| {
                *l = Local::Reserved;
            });
    }

    fn add_literal(&mut self, literal: &Literal) -> Result<FrameIndex> {
        self.constants.push((*literal).into());
        let (i, _) = self
            .reserve_next_free_register()
            .context("Cannot reserve a register")?;
        let index = FrameIndex::LocalIndex(i);
        self.opcodes
            .push(OpCode::LoadConstant(self.constants.len() - 1, index));
        Ok(index)
    }
}

pub struct Compiler {
    pub frames: Vec<CompilerFrame>,
}

impl Compiler {
    fn frame(&self, index: usize) -> Option<&CompilerFrame> {
        self.frames.get(index)
    }

    fn last_frame(&self) -> Result<&CompilerFrame> {
        self.frames.last().context("Could not get last frame")
    }

    fn find_local_symbol(&self, frame_index: usize, symbol: &Symbol) -> Option<FrameIndex> {
        self.frame(frame_index)?.find_local(symbol)
    }

    fn push_opcode(&mut self, opcode: OpCode<usize, FrameIndex>) -> Result<()> {
        self.last_frame()?.opcodes.push(opcode);
        Ok(())
    }

    fn find_nonlocal_symbol(&mut self, frame_index: usize, symbol: &Symbol) -> Option<FrameIndex> {
        if frame_index == 0 {
            None
        } else {
            let parent_index = frame_index - 1;
            self.find_local_symbol(parent_index, symbol)
                .or_else(|| self.find_nonlocal_symbol(parent_index, symbol))
                .and_then(|i| Some(self.frame(frame_index)?.resolve_capture(i)))
        }
    }

    fn resolve_symbol(&mut self, symbol: &Symbol) -> Result<FrameIndex> {
        let last_frame_index = self.frames.len() - 1;
        self.find_local_symbol(last_frame_index, symbol)
            .or_else(|| self.find_nonlocal_symbol(last_frame_index, symbol))
            .context(format!("Could not find symbol {:?}", symbol))
    }

    /// Compile the given symbol. Reserves one local
    fn compile_symbol(&mut self, symbol: &Symbol) -> Result<FrameIndex> {
        self.resolve_symbol(symbol)
    }

    fn compile_literal(&mut self, literal: &Literal) -> Result<FrameIndex> {
        self.last_frame()?.add_literal(literal)
    }

    fn compile_assign(
        &mut self,
        symbol: &Symbol,
        expression: &Expression,
        tail_position: bool,
    ) -> Result<Option<FrameIndex>> {
        if tail_position {
            self.compile_expression(expression, true)
        } else {
            // compile before clearing the scope of the symbol
            let expression_position = self
                .compile_expression(expression, false)?
                .context("Didn't get a return position")?;

            let lf = self.last_frame()?;
            // Clear any appearance of this symbol in the current scope
            lf.clear_scope_of_symbol(symbol);
            let new_local = Local::Named(Named {
                name: *symbol,
                depth: lf.depth,
            });
            Ok(Some(match expression_position {
                FrameIndex::LocalIndex(i) => {
                    // The result of the expression is in a register, just assign
                    // this to a new name
                    lf.locals[i] = new_local;
                    expression_position
                }
                FrameIndex::CaptureIndex(i) => {
                    // The result of the expression was a capture,
                    // copy the capture to a new local
                    let (new_position, new_local_ref) = lf
                        .reserve_next_free_register()
                        .context("Could not reserve free register")?;
                    lf.opcodes.push(OpCode::CopyValue(
                        expression_position,
                        FrameIndex::LocalIndex(new_position),
                    ));
                    *new_local_ref = new_local;
                    FrameIndex::LocalIndex(new_position)
                }
            }))
        }
    }

    fn maybe_return_value(
        &mut self,
        expression_result_position: FrameIndex,
        tail_position: bool,
    ) -> Result<Option<FrameIndex>> {
        if tail_position {
            self.last_frame()?
                .opcodes
                .push(OpCode::Return(expression_result_position));
            Ok(None)
        } else {
            Ok(Some(expression_result_position))
        }
    }

    fn clear_unreserved_locals(&mut self) -> Result<()> {
        let frame = self.last_frame()?;
        frame.locals.iter_mut().enumerate().for_each(|(i, l)| {
            if matches!(l, Local::ToClear) {
                *l = Local::None;
                frame
                    .opcodes
                    .push(OpCode::CloseValue(FrameIndex::LocalIndex(i)))
            }
        });
        Ok(())
    }

    /// Compile a Vec of expressions, and discard all their results
    fn compile_and_ignore_expressions(
        &mut self,
        expressions: Vec<Expression>,
    ) -> Result<Vec<usize>> {
        let frame = self.last_frame()?;
        let mut delayed_clear = vec![];
        for expression in expressions {
            let pos = self
                .compile_expression(&expression, false)?
                .context("Got no frame index from a non-tail expression")?;
            match pos {
                FrameIndex::LocalIndex(i) => match frame.locals[i] {
                    Local::Reserved => {
                        frame.locals[i] = Local::None;
                        frame
                            .opcodes
                            .push(OpCode::CloseValue(FrameIndex::LocalIndex(i)))
                    }
                    Local::Named(_) => delayed_clear.push(i),
                    _ => (),
                },
                _ => (),
            };
        }
        Ok(delayed_clear)
    }

    fn compile_block(
        &mut self,
        ignored: Vec<Expression>,
        result: &Expression,
        tail_position: bool,
    ) -> Result<Option<FrameIndex>> {
        let frame = self.last_frame()?;
        frame.depth += 1;
        let delayed_clear = self.compile_and_ignore_expressions(ignored)?;
        let res = self.compile_expression(result, tail_position);
        frame.depth -= 1;
        delayed_clear.iter().for_each(|&i| {
            frame.locals[i] = Local::None;
            frame
                .opcodes
                .push(OpCode::CloseValue(FrameIndex::LocalIndex(i)));
        });
        res
    }
    fn clear_reserved_indices(&mut self, indices: Vec<FrameIndex>) -> Result<()> {
        for l in indices {
            match l {
                FrameIndex::LocalIndex(i) => {
                    let frame = self.last_frame()?;
                    if matches!(frame.locals[i], Local::Reserved) {
                        frame.locals[i] = Local::None;
                        frame
                            .opcodes
                            .push(OpCode::CloseValue(FrameIndex::LocalIndex(i)))
                    }
                }
                _ => (),
            }
        }
        Ok(())
    }

    fn compile_call(
        &mut self,
        function: &Expression,
        arguments: Vec<Expression>,
        tail_position: bool,
    ) -> Result<Option<FrameIndex>> {
        let function_index = self.compile_expression(function, false)?.unwrap();
        let mut arg_locs = vec![function_index];
        for arg in arguments {
            arg_locs.push(self.compile_expression(&arg, false)?.unwrap());
        }
        if tail_position {
            self.push_opcode(OpCode::TailCall(function_index))?;
            for l in arg_locs {
                self.push_opcode(OpCode::CallArgument(l))?;
            }
            Ok(None)
        } else {
            let (result_index, _) = self
                .last_frame()?
                .reserve_next_free_register()
                .context("Could not get a free register")?;
            self.push_opcode(OpCode::Call(
                function_index,
                FrameIndex::LocalIndex(result_index),
            ))?;
            for l in arg_locs {
                self.push_opcode(OpCode::CallArgument(l))?;
            }
            self.clear_reserved_indices(arg_locs);
            Ok(Some(FrameIndex::LocalIndex(result_index)))
        }
    }

    fn compile_function(&mut self, args: Vec<Symbol>, body: &Expression) -> Result<FrameIndex> {
        self.frames
            .push(CompilerFrame::new(args, self.last_frame()?.depth + 1));
        self.last_frame()?.depth += 1;
        self.compile_expression(body, true)?;
        let new_frame = self.frames.pop().context("Could not get a last frame")?;
        let frame = self.last_frame()?;
        frame.functions.push(Rc::new(Function {
            opcodes: new_frame.opcodes,
            functions: new_frame.functions,
            constants: new_frame.constants,
            arity: args.len(),
            registers: new_frame.locals.len() + new_frame.captures.len(),
        }));
        let function_index = frame.functions.len() - 1;
        let (closure_index, _) = frame
            .reserve_next_free_register()
            .context("Cannot reserved register")?;
        frame.opcodes.push(OpCode::CreateClosure(
            function_index,
            FrameIndex::LocalIndex(closure_index),
        ));
        new_frame
            .captures
            .iter()
            .for_each(|c| frame.opcodes.push(OpCode::CaptureValue(*c)));
        Ok(FrameIndex::LocalIndex(closure_index))
    }

    fn compile_condition(
        &mut self,
        clauses: Vec<(Expression, Expression)>,
        otherwise: &Expression,
        tail_position: bool,
    ) -> Result<Option<FrameIndex>> {
        let frame = self.last_frame()?;
        frame.depth += 1;
	let mut prev_op_pos;
	let mut jump_end_pos = vec![];
        for (clause, result) in clauses {
            let clause_pos = self.compile_expression(&clause, false)?;
            frame.opcodes.push(OpCode::Crash);
            prev_op_pos = frame.opcodes.len() - 1;
	    frame.depth += 1;
	    let result_pos = self.compile_expression(&result, tail_position)?;
	    frame.opcodes.push(OpCode::Crash);
	    jump_end_pos.push(frame.opcodes.len()-1);
	    frame.depth -= 1;
        }
        Ok(None)
    }

    fn compile_expression(
        &mut self,
        expression: &Expression,
        tail_position: bool,
    ) -> Result<Option<FrameIndex>> {
        match expression {
            Expression::Condition(_, _) => todo!(),
            Expression::Call(function, arguments) => {
                self.compile_call(&function, *arguments, tail_position)
            }
            Expression::Function(args, body) => {
                let pos = self.compile_function(*args, &body)?;
                self.maybe_return_value(pos, tail_position)
            }
            Expression::Assign(symbol, expression) => {
                self.compile_assign(symbol, &expression, tail_position)
            }
            Expression::Block(ignored, exp) => self.compile_block(*ignored, exp, tail_position),
            Expression::Literal(literal) => {
                let pos = self.compile_literal(literal)?;
                self.maybe_return_value(pos, tail_position)
            }
            Expression::Symbol(symbol) => {
                let pos = self.compile_symbol(symbol)?;
                self.maybe_return_value(pos, tail_position)
            }
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_something_works() {
        assert!(2 == 2);
    }
}
