use std::fmt::Debug;

use crate::native_function::NativeFunction;

pub type VecOffset = i16;
pub type VecIndex = u8;

#[derive(Debug, Clone, PartialEq)]
pub struct RegisterIndex(pub VecIndex);

impl From<RegisterIndex> for usize {
    fn from(value: RegisterIndex) -> Self {
        value.0 as usize
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstantIndex(pub VecIndex);

impl From<ConstantIndex> for usize {
    fn from(value: ConstantIndex) -> Self {
        value.0 as usize
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaptureIndex(pub VecIndex);

impl From<CaptureIndex> for usize {
    fn from(value: CaptureIndex) -> Self {
        value.0 as usize
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValueIndex {
    Register(RegisterIndex),
    Constant(ConstantIndex),
    Capture(CaptureIndex),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionIndex(pub VecIndex);

impl From<FunctionIndex> for usize {
    fn from(value: FunctionIndex) -> Self {
        value.0 as usize
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OpCode {
    /// Call the function in register .0,
    /// Putting the result into the register .1
    /// Arguments are given after the call
    Call(ValueIndex, RegisterIndex),
    /// Create a tail call with the function in the given register
    /// Followed by `CallArgument`s
    TailCall(ValueIndex),
    CallArgument(ValueIndex),
    /// Create a recursive let's pointer
    DeclareRecursive(RegisterIndex),
    /// Fill a recursive let's predeclared symbol in 1 with an actual value from 0
    FillRecursive(ValueIndex, RegisterIndex),
    /// Return the value in the given register
    Return(ValueIndex),
    /// Unconditionally jump with the given offset
    Jump(VecOffset),
    /// Check the boolean in .0, if false, jump by the given offset, otherwise continue
    JumpToPositionIfFalse(ValueIndex, VecOffset),
    /// Copy the value from 0 to 1
    CopyValue(ValueIndex, RegisterIndex),

    /// Free the value at the given register
    CloseValue(RegisterIndex),
    /// Create closure. Takes the index of the function in the current chunk,
    /// puts the result in the register .1
    CreateClosure(FunctionIndex, RegisterIndex),
    /// Capture a value from the currently running function and put it
    /// into the closure being created
    CaptureValue(ValueIndex),
    /// Unconditional crash
    Crash,
    /// Insert this native function into the given register
    InsertNativeFunction(NativeFunction, RegisterIndex),
}
