use anyhow::{anyhow, Context, Result};
use std::{collections::HashMap, iter::repeat, rc::Rc};

use crate::{
    expression::{Expression, Literal, Symbol},
    opcode::OpCode,
};

#[derive(PartialEq, Debug, Clone)]
pub enum Local {
    /// A register which contains a value, which should not be reused
    Reserved,
    /// A register which contains a value, which can be reused
    ToClear,
    /// A register which doesn't contain a value
    None,
}

#[derive(PartialEq, Debug, Clone)]
pub struct LocalIndex(usize);
#[derive(PartialEq, Debug, Clone)]
pub struct CaptureIndex(usize);
#[derive(PartialEq, Debug, Clone)]
pub struct ConstIndex(usize);

#[derive(PartialEq, Debug, Clone)]
pub enum FrameIndex {
    LocalIndex(LocalIndex),
    CaptureIndex(CaptureIndex),
    ConstIndex(ConstIndex),
}

#[derive(Debug)]
pub struct CompilerFunction {
    pub opcodes: Vec<OpCode<FrameIndex>>,
    pub constants: Vec<Value>,
    pub functions: Vec<Rc<CompilerFunction>>,
    pub arity: usize,
    pub captures: usize,
    pub registers: usize,
}

#[derive(Debug)]
pub struct CompilerFrame {
    pub names: HashMap<(usize, Symbol), FrameIndex>,
    pub locals: Vec<Local>,
    /// A triple of symbol, depth, register
    pub captures: Vec<FrameIndex>,
    pub depth: usize,
    pub opcodes: Vec<OpCode<FrameIndex>>,
    pub constants: Vec<Value>,
    pub functions: Vec<Rc<Function>>,
}

impl CompilerFrame {
    fn to_function(self, arity: usize) -> Function {
        Function {
            opcodes: self
                .opcodes
                .iter()
                .map(|o| {
                    o.convert(
                        &mut |v_index| v_index as u8,
                        &mut |ref_index| match ref_index {
                            FrameIndex::LocalIndex(LocalIndex(l)) => l + self.captures.len(),
                            FrameIndex::CaptureIndex(CaptureIndex(c)) => c,
                        } as u8,
                    )
                })
                .collect(),

            constants: self.constants,
            functions: self.functions,
            arity,
            capture_offset: self.captures.len(),
            registers: self.locals.len() + self.captures.len(),
        }
    }

    fn increase_scope(&mut self) {
        self.depth += 1;
    }

    fn reduce_scope(&mut self) {
        self.depth -= 1;
        self.names.retain(|(depth, _), _| *depth <= self.depth);
    }

    fn assign_name(&mut self, name: &Symbol, register: FrameIndex) {
        self.names.insert((self.depth, name.clone()), register);
    }

    /// Set any local that is ToClear and is not named to None, and add a CloseValue OpCode
    fn clear_unused_locals(&mut self) {
        for (i, l) in self.locals.iter_mut().enumerate() {
            match l {
                Local::ToClear
                    if !self
                        .names
                        .iter()
                        .any(|(_, r)| *r == FrameIndex::LocalIndex(LocalIndex(i))) =>
                {
                    *l = Local::None;
                    self.opcodes.push(OpCode::CloseValue(i))
                }
                _ => (),
            }
        }
    }

    pub fn new(arguments: &Vec<Symbol>, depth: usize) -> Self {
        CompilerFrame {
            locals: repeat(Local::ToClear).take(arguments.len()).collect(),
            names: HashMap::from_iter(
                arguments
                    .iter()
                    .enumerate()
                    .map(|(i, s)| ((depth, s.clone()), FrameIndex::LocalIndex(LocalIndex(i)))),
            ),
            captures: vec![],
            depth: depth + 1,
            opcodes: vec![],
            constants: vec![],
            functions: vec![],
        }
    }

    /// Finds the symbol with the same name at the greatest depth <= self.depth,
    fn find_local(&self, symbol: &Symbol) -> Option<FrameIndex> {
        self.names
            .iter()
            .filter(|((d, s), _)| {
                debug_assert!(*d <= self.depth);
                s == symbol
            })
            .max_by_key(|((d, _), _)| d)
            .map(|(_, i)| i.clone())
    }

    /// If the capture index already exists in captures, return its position,
    /// otherwise create a new one
    fn resolve_capture(&mut self, index: FrameIndex) -> CaptureIndex {
        CaptureIndex(
            self.captures
                .iter()
                .position(|c| *c == index)
                .unwrap_or_else(|| {
                    self.captures.push(index);
                    self.captures.len() - 1
                }),
        )
    }

    /// Returns a tuple of the index of the next free register and a mutable reference to it
    fn reserve_next_free_register(&mut self) -> (LocalIndex, &mut Local) {
        let index = self.locals.iter().position(|l| matches!(l, Local::None));

        match index {
            Some(i) => {
                self.locals[i] = Local::Reserved;
                self.locals.get_mut(i).map(|r| (LocalIndex(i), r)).unwrap()
            }
            None => {
                self.locals.push(Local::Reserved);
                let index = self.locals.len() - 1;
                self.locals
                    .last_mut()
                    .map(|l| (LocalIndex(index), l))
                    .unwrap()
            }
        }
    }

    fn add_literal(&mut self, target_register: LocalIndex, literal: &Literal) {
        self.constants.push((literal.clone()).into());

        self.opcodes.push(OpCode::LoadConstant(
            self.constants.len() - 1,
            FrameIndex::LocalIndex(target_register),
        ));
    }

    fn compile_literal(
        &mut self,
        position: Option<LocalIndex>,
        literal: &Literal,
    ) -> Result<LocalIndex> {
        let pos = position.unwrap_or_else(|| self.reserve_next_free_register().0);
        self.add_literal(pos.clone(), literal);
        Ok(pos)
    }
}

#[derive(Debug)]
pub struct Compiler {
    frames: Vec<CompilerFrame>,
}

impl Compiler {
    fn clear_unused_locals(&mut self) -> Result<()> {
        self.frames
            .last_mut()
            .map(|f| f.clear_unused_locals())
            .context("No last frame")
    }

    /// Set the local at the given index to ToClear
    fn drop_register(&mut self, index: FrameIndex) -> Result<()> {
        match index {
            FrameIndex::LocalIndex(index) => {
                let ls = &mut self.frames.last_mut().unwrap().locals;
                if ls[index.0] == Local::Reserved {
                    ls[index.0] = Local::ToClear;
                }
            }
            _ => (),
        }
        Ok(())
    }

    fn reserve_next_free_register(&mut self) -> Result<(LocalIndex, &mut Local)> {
        self.frames
            .last_mut()
            .context("No last frame")
            .map(|f| f.reserve_next_free_register())
    }

    fn find_local_symbol(&self, frame_index: usize, symbol: &Symbol) -> Option<FrameIndex> {
        self.frames.get(frame_index)?.find_local(symbol)
    }

    fn push_opcode(&mut self, opcode: OpCode<usize, FrameIndex>) -> Result<usize> {
        self.frames.last_mut().unwrap().opcodes.push(opcode);
        Ok(self.frames.last().unwrap().opcodes.len() - 1)
    }

    fn increase_scope(&mut self) {
        self.frames.last_mut().unwrap().increase_scope()
    }

    fn reduce_scope(&mut self) {
        self.frames.last_mut().unwrap().reduce_scope()
    }

    fn assign_name(&mut self, symbol: &Symbol, register: FrameIndex) -> Result<()> {
        self.frames
            .last_mut()
            .map(|f| f.assign_name(symbol, register))
            .context("Could not get last frame")
    }

    fn find_nonlocal_symbol(&mut self, frame: usize, symbol: &Symbol) -> Option<FrameIndex> {
        if frame == 0 {
            None
        } else {
            let parent_index = frame - 1;
            self.find_local_symbol(parent_index, symbol)
                .or_else(|| self.find_nonlocal_symbol(parent_index, symbol))
                .and_then(|i| {
                    Some(FrameIndex::CaptureIndex(
                        self.frames.get_mut(frame)?.resolve_capture(i),
                    ))
                })
        }
    }

    fn resolve_symbol(&mut self, symbol: &Symbol) -> Option<FrameIndex> {
        let last_frame_index = self.frames.len() - 1;
        self.find_local_symbol(last_frame_index, symbol)
            .or_else(|| self.find_nonlocal_symbol(last_frame_index, symbol))
    }

    fn compile_literal(
        &mut self,
        position: Option<LocalIndex>,
        literal: &Literal,
    ) -> Result<FrameIndex> {
        self.frames
            .last_mut()
            .unwrap()
            .compile_literal(position, literal)
            .map(|l| FrameIndex::LocalIndex(l))
    }

    fn compile_symbol(
        &mut self,
        position: Option<LocalIndex>,
        symbol: &Symbol,
    ) -> Result<FrameIndex> {
        if let Some(i) = self.resolve_symbol(symbol) {
            match position {
                Some(p) => match i.clone() {
                    FrameIndex::LocalIndex(l) if l == p => Ok(i),
                    x => {
                        self.push_opcode(OpCode::CopyValue(x, FrameIndex::LocalIndex(p.clone())))?;
                        Ok(FrameIndex::LocalIndex(p))
                    }
                },
                None => Ok(i),
            }
        } else if let Some(fun) = NativeFunction::resolve_symbol(symbol) {
            let result_pos =
                position.unwrap_or_else(|| self.reserve_next_free_register().unwrap().0);
            self.push_opcode(OpCode::InsertNativeFunction(
                fun,
                FrameIndex::LocalIndex(result_pos.clone()),
            ))?;
            Ok(FrameIndex::LocalIndex(result_pos))
        } else {
            Err(anyhow!("Could not resolve symbol: {:?}", symbol))
        }
    }

    fn compile_assign(
        &mut self,
        position: Option<LocalIndex>,
        symbol: &Symbol,
        expression: &Expression,
        tail_position: bool,
    ) -> Result<Option<FrameIndex>> {
        match self.compile_expression(position, expression, tail_position)? {
            Some(p) => {
                self.assign_name(symbol, p.clone())?;
                Ok(Some(p))
            }
            None => Ok(None),
        }
    }

    /// Compile a Vec of expressions, and discard all their results
    fn compile_and_ignore_expressions(&mut self, expressions: &Vec<Expression>) -> Result<()> {
        for expression in expressions {
            let index = self.compile_expression(None, expression, false)?.unwrap();
            self.drop_register(index)?;
            self.clear_unused_locals()?;
        }
        Ok(())
    }

    fn compile_block(
        &mut self,
        position: Option<LocalIndex>,
        ignored: &Vec<Expression>,
        result: &Expression,
        tail_position: bool,
    ) -> Result<Option<FrameIndex>> {
        self.increase_scope();
        self.compile_and_ignore_expressions(ignored)?;
        let res = self.compile_expression(position, result, tail_position);
        self.reduce_scope();
        res
    }

    fn compile_call(
        &mut self,
        position: Option<LocalIndex>,
        function: &Expression,
        arguments: &Vec<Expression>,
        tail_position: bool,
    ) -> Result<Option<FrameIndex>> {
        let function_index = self.compile_expression(None, function, false)?.unwrap();
        let mut arg_indices = vec![];
        for arg in arguments {
            arg_indices.push(self.compile_expression(None, arg, false)?.unwrap());
        }

        if tail_position {
            self.push_opcode(OpCode::TailCall(function_index))?;
            for i in arg_indices {
                self.push_opcode(OpCode::CallArgument(i))?;
            }
            Ok(None)
        } else {
            let result_pos =
                position.unwrap_or_else(|| self.reserve_next_free_register().unwrap().0);
            self.push_opcode(OpCode::Call(
                function_index.clone(),
                FrameIndex::LocalIndex(result_pos.clone()),
            ))?;
            for i in arg_indices.iter() {
                self.push_opcode(OpCode::CallArgument(i.clone()))?;
            }
            arg_indices.push(function_index);
            for i in arg_indices {
                self.drop_register(i)?;
            }
            self.clear_unused_locals()?;
            Ok(Some(FrameIndex::LocalIndex(result_pos)))
        }
    }

    fn compile_function(
        &mut self,
        position: Option<LocalIndex>,
        args: &Vec<Symbol>,
        body: &Expression,
    ) -> Result<FrameIndex> {
        self.frames.push(CompilerFrame::new(
            args,
            self.frames.last().unwrap().depth + 1,
        ));
        self.increase_scope();

        self.compile_expression(None, body, true)?;

        let new_frame = self.frames.pop().unwrap();
        let captures = new_frame.captures.clone();
        let frame = self.frames.last_mut().unwrap();

        frame
            .functions
            .push(Rc::new(new_frame.to_function(args.len())));
        let function_index = frame.functions.len() - 1;
        let closure_index = position.unwrap_or_else(|| frame.reserve_next_free_register().0);
        frame.opcodes.push(OpCode::CreateClosure(
            function_index,
            FrameIndex::LocalIndex(closure_index.clone()),
        ));
        captures
            .into_iter()
            .for_each(|c| frame.opcodes.push(OpCode::CaptureValue(c)));
        Ok(FrameIndex::LocalIndex(closure_index))
    }

    fn compile_condition(
        &mut self,
        position: Option<LocalIndex>,
        clauses: &Vec<(Expression, Expression)>,
        otherwise: &Expression,
        tail_position: bool,
    ) -> Result<Option<FrameIndex>> {
        self.increase_scope();
        let result_pos = position.unwrap_or_else(|| self.reserve_next_free_register().unwrap().0);
        let mut jump_end_pos = vec![];
        for (clause, result) in clauses {
            // Generate an expression for the clause in a new index
            let clause_index = self.compile_expression(None, &clause, false)?.unwrap();
            let prev_op_pos = self.push_opcode(OpCode::Crash)?;
            self.increase_scope();
            self.compile_expression(Some(result_pos.clone()), &result, tail_position)?;
            self.reduce_scope();
            // Free the register for the clause
            self.drop_register(clause_index.clone())?;
            self.clear_unused_locals()?;
            jump_end_pos.push(self.push_opcode(OpCode::Crash)?);
            self.frames.last_mut().unwrap().opcodes[prev_op_pos] = OpCode::JumpToPositionIfFalse(
                clause_index,
                self.frames.last().unwrap().opcodes.len(),
            )
        }
        self.compile_expression(Some(result_pos.clone()), otherwise, tail_position)?;

        self.reduce_scope();
        // patch in all of the jumps after the expressions
        for p in jump_end_pos {
            self.frames.last_mut().unwrap().opcodes[p] =
                OpCode::Jump(self.frames.last().unwrap().opcodes.len())
        }
        if tail_position {
            Ok(None)
        } else {
            Ok(Some(FrameIndex::LocalIndex(result_pos)))
        }
    }

    pub fn compile_expression(
        &mut self,
        position: Option<LocalIndex>,
        expression: &Expression,
        tail_position: bool,
    ) -> Result<Option<FrameIndex>> {
        let expression_result = match expression {
            Expression::Condition(clauses, otherwise) => {
                self.compile_condition(position, clauses, otherwise, tail_position)?
            }
            Expression::Call(function, arguments) => {
                self.compile_call(position, function, arguments, tail_position)?
            }
            Expression::Assign(symbol, expression) => {
                self.compile_assign(position, symbol, expression, tail_position)?
            }
            Expression::Function(arguments, body) => {
                Some(self.compile_function(position, arguments, body)?)
            }
            Expression::Block(ignored, result) => {
                self.compile_block(position, ignored, result, tail_position)?
            }
            Expression::Literal(literal) => Some(self.compile_literal(position, literal)?),
            Expression::Symbol(symbol) => Some(self.compile_symbol(position, symbol)?),
        };
        Ok(match expression_result {
            Some(index) if tail_position => {
                self.push_opcode(OpCode::Return(index))?;
                None
            }
            x => x,
        })
    }

    pub fn new() -> Compiler {
        Compiler {
            frames: vec![CompilerFrame::new(&vec![], 0)],
        }
    }

    pub fn frame_to_function(&mut self) -> Function {
        let f = self.frames.pop().unwrap();
        f.to_function(0)
    }
}

#[cfg(test)]
mod tests {
    use super::Compiler;
    use crate::parser::parse_program;
    // #[test]
    // fn test_compiler_is_working() {
    //     let (s, e) = parse_program(
    //         "fn fib (fib, n) {cond {n `< 2 => n, {fib fib {n `- 1}} `+ {fib fib {n `- 2}}}}",
    //     )
    //     .unwrap();
    //     let mut c = Compiler::new();
    //     c.compile_expression(None, &e[0], true).unwrap();
    //     assert_eq!(c.frame_to_function().opcodes, vec![]);
    //     assert_eq!(s, "");
    //     assert_eq!(e, vec![]);
    // }
}
