use std::{collections::HashMap, iter::repeat, rc::Rc};

use crate::{
    expression::{Block, Expression, Let, Literal, LocatedExpression, Symbol},
    native_function::NativeFunction,
    opcode::{CaptureIndex, ConstantIndex, FunctionIndex, OpCode, RegisterIndex, ValueIndex},
    value::{Function, Placeholder, Value, ValueError},
};

#[derive(Debug)]
pub enum CompilerError {
    NoFrames,
    NoElementsInLet,
    NoNativeSymbol,
}

type Result<T> = std::result::Result<T, CompilerError>;

#[derive(PartialEq, Debug, Clone)]
pub enum Local {
    /// A register which contains a value, which should not be reused
    Reserved,
    /// A register which contains a value, which can be reused
    ToClear,
    /// A register which doesn't contain a value
    None,
}

#[derive(Debug)]
pub struct CompilerFrame {
    pub names: HashMap<(usize, Symbol), ValueIndex>,
    pub locals: Vec<Local>,
    /// A triple of symbol, depth, register
    pub captures: Vec<ValueIndex>,
    pub depth: usize,
    pub opcodes: Vec<OpCode>,
    pub constants: Vec<Value>,
    pub functions: Vec<Rc<Function>>,
}

impl CompilerFrame {
    fn to_function(self, arity: usize) -> Function {
        Function {
            opcodes: self.opcodes,
            constants: self.constants,
            functions: self.functions,
            arity,
            num_captures: self.captures.len(),
            num_registers: self.locals.len(),
        }
    }

    fn increase_scope(&mut self) {
        self.depth += 1;
    }

    fn reduce_scope(&mut self) {
        self.depth -= 1;
        self.names.retain(|(depth, _), _| *depth <= self.depth);
    }

    fn assign_name(&mut self, name: &Symbol, register: ValueIndex) {
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
                        .any(|(_, r)| *r == ValueIndex::Register(RegisterIndex(i as u8))) =>
                {
                    *l = Local::None;
                    // self.opcodes.push(OpCode::CloseValue(i))
                }

                _ => (),
            }
        }
    }

    /// Create a new compiler frame with the given arguments and depth
    pub fn new(arguments: &Vec<Symbol>, depth: usize) -> Self {
        CompilerFrame {
            locals: repeat(Local::ToClear).take(arguments.len()).collect(),
            names: HashMap::from_iter(arguments.iter().enumerate().map(|(i, s)| {
                (
                    (depth, s.clone()),
                    ValueIndex::Register(RegisterIndex(i as u8)),
                )
            })),
            captures: vec![],
            depth: depth + 1,
            opcodes: vec![],
            constants: vec![],
            functions: vec![],
        }
    }

    /// Finds the symbol with the same name at the greatest depth <= self.depth,
    fn find_local(&self, symbol: &Symbol) -> Option<ValueIndex> {
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
    fn resolve_capture(&mut self, index: ValueIndex) -> CaptureIndex {
        CaptureIndex(
            self.captures
                .iter()
                .position(|c| *c == index)
                .unwrap_or_else(|| {
                    self.captures.push(index);
                    self.captures.len() - 1
                }) as u8,
        )
    }

    /// Returns a tuple of the index of the next free register and a mutable reference to it
    fn reserve_next_free_register(&mut self) -> (RegisterIndex, &mut Local) {
        let index = self.locals.iter().position(|l| matches!(l, Local::None));

        match index {
            Some(i) => {
                self.locals[i] = Local::Reserved;
                self.locals
                    .get_mut(i)
                    .map(|r| (RegisterIndex(i as u8), r))
                    .unwrap()
            }
            None => {
                self.locals.push(Local::Reserved);
                let index = self.locals.len() - 1;
                self.locals
                    .last_mut()
                    .map(|l| (RegisterIndex(index as u8), l))
                    .unwrap()
            }
        }
    }

    fn add_literal(&mut self, literal: &Literal) -> ConstantIndex {
        self.constants.push((literal.clone()).into());
        ConstantIndex((self.constants.len() - 1) as u8)
    }

    fn compile_literal(
        &mut self,
        position: Option<RegisterIndex>,
        literal: &Literal,
    ) -> Result<ValueIndex> {
        let lit_pos = self.add_literal(literal);
        Ok(match position {
            Some(p) => {
                self.opcodes
                    .push(OpCode::CopyValue(ValueIndex::Constant(lit_pos), p.clone()));
                ValueIndex::Register(p)
            }
            None => ValueIndex::Constant(lit_pos),
        })
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
            .ok_or(CompilerError::NoFrames)
    }

    /// Set the local at the given index to ToClear
    fn drop_register(&mut self, index: ValueIndex) -> Result<()> {
        match index {
            ValueIndex::Register(RegisterIndex(i)) => {
                let ls = &mut self.frames.last_mut().unwrap().locals;
                if ls[i as usize] == Local::Reserved {
                    ls[i as usize] = Local::ToClear;
                }
            }
            _ => (),
        }
        Ok(())
    }

    fn reserve_next_free_register(&mut self) -> Result<(RegisterIndex, &mut Local)> {
        self.frames
            .last_mut()
            .ok_or(CompilerError::NoFrames)
            .map(|f| f.reserve_next_free_register())
    }

    fn find_local_symbol(&self, frame_index: usize, symbol: &Symbol) -> Option<ValueIndex> {
        self.frames.get(frame_index)?.find_local(symbol)
    }

    fn push_opcode(&mut self, opcode: OpCode) -> Result<usize> {
        self.frames.last_mut().unwrap().opcodes.push(opcode);
        Ok(self.frames.last().unwrap().opcodes.len() - 1)
    }

    fn increase_scope(&mut self) {
        self.frames.last_mut().unwrap().increase_scope()
    }

    fn reduce_scope(&mut self) {
        self.frames.last_mut().unwrap().reduce_scope()
    }

    fn assign_name(&mut self, symbol: &Symbol, register: ValueIndex) -> Result<()> {
        self.frames
            .last_mut()
            .map(|f| f.assign_name(symbol, register))
            .ok_or(CompilerError::NoFrames)
    }

    fn find_nonlocal_symbol(&mut self, frame: usize, symbol: &Symbol) -> Option<ValueIndex> {
        if frame == 0 {
            None
        } else {
            let parent_index = frame - 1;
            self.find_local_symbol(parent_index, symbol)
                .or_else(|| self.find_nonlocal_symbol(parent_index, symbol))
                .and_then(|i| {
                    Some(ValueIndex::Capture(
                        self.frames.get_mut(frame)?.resolve_capture(i),
                    ))
                })
        }
    }

    fn resolve_symbol(&mut self, symbol: &Symbol) -> Option<ValueIndex> {
        let last_frame_index = self.frames.len() - 1;
        self.find_local_symbol(last_frame_index, symbol)
            .or_else(|| self.find_nonlocal_symbol(last_frame_index, symbol))
    }

    fn compile_literal(
        &mut self,
        position: Option<RegisterIndex>,
        literal: &Literal,
    ) -> Result<ValueIndex> {
        self.frames
            .last_mut()
            .unwrap()
            .compile_literal(position, literal)
    }

    fn resolve_native_symbol(
        &mut self,
        position: Option<RegisterIndex>,
        symbol: &Symbol,
    ) -> Result<RegisterIndex> {
        let func = NativeFunction::resolve_symbol(symbol).ok_or(CompilerError::NoNativeSymbol)?;
        let register = position.unwrap_or_else(|| self.reserve_next_free_register().unwrap().0);
        self.push_opcode(OpCode::InsertNativeFunction(func, register.clone()));
        Ok(register)
    }

    fn compile_symbol(
        &mut self,
        position: Option<RegisterIndex>,
        symbol: &Symbol,
    ) -> Result<ValueIndex> {
        if let Some(i) = self.resolve_symbol(symbol) {
            // This symbol is declared in scope.
            // If position is not None, and is not equal to position i, emit a copy
            match position {
                Some(p) if i == ValueIndex::Register(p.clone()) => Ok(i),
                Some(p) => {
                    self.push_opcode(OpCode::CopyValue(i.clone(), p))?;
                    Ok(i)
                }
                None => Ok(i),
            }
        } else {
            Ok(ValueIndex::Register(
                self.resolve_native_symbol(position, symbol)?,
            ))
        }
    }

    fn declare_recursive_symbol(
        &mut self,
        position: Option<RegisterIndex>,
        symbol: &Symbol,
    ) -> RegisterIndex {
        let i = position.unwrap_or_else(|| self.reserve_next_free_register().unwrap().0);
        self.push_opcode(OpCode::DeclareRecursive(i.clone()))
            .unwrap();
        self.assign_name(symbol, ValueIndex::Register(i.clone()))
            .unwrap();
        i
    }

    fn compile_recursive_let<'a>(
        &mut self,
        position: Option<RegisterIndex>,
        pairs: &Vec<(Symbol, LocatedExpression<'a>)>,
        tail_position: bool,
    ) -> Result<Option<ValueIndex>> {
        let ((last_symbol, last_exp), ignored) =
            pairs.split_last().ok_or(CompilerError::NoElementsInLet)?;
        let ignored_pointers: Vec<_> = ignored
            .iter()
            .map(|(s, _)| self.declare_recursive_symbol(None, s))
            .collect();
        let last_pointer = self.declare_recursive_symbol(position, last_symbol);
        for (p, (s, e)) in ignored_pointers.into_iter().zip(ignored) {
            let pos = self.compile_expression(None, &e, false)?.unwrap();
            self.push_opcode(OpCode::FillRecursive(pos, p)).unwrap();
            self.clear_unused_locals()?;
        }
        if let Some(p) = self.compile_expression(None, last_exp, tail_position)? {
            self.push_opcode(OpCode::FillRecursive(p.clone(), last_pointer))?;
            Ok(Some(p))
        } else {
            Ok(None)
        }
    }

    fn compile_non_recursive_let<'a>(
        &mut self,
        position: Option<RegisterIndex>,
        pairs: &Vec<(Symbol, LocatedExpression<'a>)>,
        tail_position: bool,
    ) -> Result<Option<ValueIndex>> {
        let ((last_symbol, last_expression), ignored) =
            pairs.split_last().ok_or(CompilerError::NoElementsInLet)?;
        for (symbol, exp) in ignored {
            match self.compile_expression(None, exp, false)? {
                Some(i) => self.assign_name(symbol, i.clone())?,
                None => unreachable!(),
            }
        }
        match self.compile_expression(position, last_expression, tail_position)? {
            Some(i) => {
                self.assign_name(last_symbol, i.clone()).unwrap();
                Ok(Some(i))
            }
            None => Ok(None),
        }
    }

    fn compile_let<'a>(
        &mut self,
        recursive: bool,
        position: Option<RegisterIndex>,
        pairs: &Vec<(Symbol, LocatedExpression<'a>)>,
        tail_position: bool,
    ) -> Result<Option<ValueIndex>> {
        if recursive {
            self.compile_recursive_let(position, pairs, tail_position)
        } else {
            self.compile_non_recursive_let(position, pairs, tail_position)
        }
    }

    /// Compile a Vec of expressions, and discard all their results
    fn compile_and_ignore_expressions<'a>(
        &mut self,
        expressions: &Vec<LocatedExpression<'a>>,
    ) -> Result<()> {
        for expression in expressions {
            let index = self.compile_expression(None, expression, false)?.unwrap();
            self.drop_register(index)?;
            self.clear_unused_locals()?;
        }
        Ok(())
    }

    fn compile_block<'a>(
        &mut self,
        scope_introducing: bool,
        position: Option<RegisterIndex>,
        ignored: &Vec<LocatedExpression<'a>>,
        result: &LocatedExpression<'a>,
        tail_position: bool,
    ) -> Result<Option<ValueIndex>> {
        if scope_introducing {
            self.increase_scope();
        }

        self.compile_and_ignore_expressions(ignored)?;
        let res = self.compile_expression(position, result, tail_position);
        if scope_introducing {
            self.reduce_scope();
        }
        res
    }

    fn compile_call(
        &mut self,
        position: Option<RegisterIndex>,
        function: &LocatedExpression,
        arguments: &Vec<LocatedExpression>,
        tail_position: bool,
    ) -> Result<Option<ValueIndex>> {
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
            self.push_opcode(OpCode::Call(function_index.clone(), result_pos.clone()))?;
            for i in arg_indices.iter() {
                self.push_opcode(OpCode::CallArgument(i.clone()))?;
            }
            arg_indices.push(function_index);
            for i in arg_indices {
                self.drop_register(i)?;
            }
            self.clear_unused_locals()?;
            Ok(Some(ValueIndex::Register(result_pos)))
        }
    }

    fn compile_function<'a>(
        &mut self,
        position: Option<RegisterIndex>,
        args: &Vec<Symbol>,
        body: &LocatedExpression<'a>,
    ) -> Result<ValueIndex> {
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
        let function_index = (frame.functions.len() - 1) as u8;
        let closure_index = position.unwrap_or_else(|| frame.reserve_next_free_register().0);
        frame.opcodes.push(OpCode::CreateClosure(
            FunctionIndex(function_index),
            closure_index.clone(),
        ));
        captures
            .into_iter()
            .for_each(|c| frame.opcodes.push(OpCode::CaptureValue(c)));
        Ok(ValueIndex::Register(closure_index))
    }

    fn compile_condition(
        &mut self,
        position: Option<RegisterIndex>,
        clauses: &Vec<(LocatedExpression, LocatedExpression)>,
        otherwise: &LocatedExpression,
        tail_position: bool,
    ) -> Result<Option<ValueIndex>> {
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
                (self.frames.last().unwrap().opcodes.len() - prev_op_pos) as i16,
            )
        }
        self.compile_expression(Some(result_pos.clone()), otherwise, tail_position)?;

        self.reduce_scope();
        // patch in all of the jumps after the expressions
        for p in jump_end_pos {
            self.frames.last_mut().unwrap().opcodes[p] =
                OpCode::Jump((self.frames.last().unwrap().opcodes.len() - p) as i16)
        }
        if tail_position {
            Ok(None)
        } else {
            Ok(Some(ValueIndex::Register(result_pos)))
        }
    }

    pub fn compile_expression<'a>(
        &mut self,
        position: Option<RegisterIndex>,
        expression: &LocatedExpression<'a>,
        tail_position: bool,
    ) -> Result<Option<ValueIndex>> {
        let expression_result = match &expression.expression {
            Expression::Condition(clauses, otherwise) => {
                self.compile_condition(position, &clauses, otherwise.as_ref(), tail_position)?
            }
            Expression::Call(function, arguments) => {
                self.compile_call(position, function.as_ref(), &arguments, tail_position)?
            }
            Expression::Let(Let { recursive, pairs }) => {
                self.compile_let(*recursive, position, pairs, tail_position)?
            }
            Expression::Function(arguments, body) => {
                Some(self.compile_function(position, &arguments, body.as_ref())?)
            }
            Expression::Block(Block {
                scope_introducing,
                ignored,
                last,
            }) => self.compile_block(
                *scope_introducing,
                position,
                &ignored,
                last.as_ref(),
                tail_position,
            )?,
            Expression::Literal(literal) => Some(self.compile_literal(position, &literal)?),
            Expression::Symbol(symbol) => Some(self.compile_symbol(position, &symbol)?),
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
