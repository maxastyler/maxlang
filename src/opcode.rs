use std::fmt::Debug;

use crate::native_function::NativeFunction;

#[derive(Debug, Clone, PartialEq)]
pub enum OpCode<VecIndex, RegisterIndex> {
    /// Call the function in register .0,
    /// Putting the result into the register .1
    /// Arguments are given after the call
    Call(RegisterIndex, RegisterIndex),
    /// Create a tail call with the function in the given register
    /// Followed by `CallArgument`s
    TailCall(RegisterIndex),

    CallArgument(RegisterIndex),

    /// Return the value in the given register
    Return(RegisterIndex),

    /// Unconditionally jump to the given position
    Jump(VecIndex),
    /// Check the boolean in .0, if false, jump to the given position, otherwise continue
    JumpToPositionIfFalse(RegisterIndex, VecIndex),
    /// Copy the value from 0 to 1
    CopyValue(RegisterIndex, RegisterIndex),
    /// Load the constant from the constants array at 0 to the position 1
    LoadConstant(VecIndex, RegisterIndex),
    /// Free the value at the given register
    CloseValue(VecIndex),
    /// Create closure. Takes the index of the function in the current chunk,
    /// puts the result in the register .1
    CreateClosure(VecIndex, RegisterIndex),
    /// Capture a value from the current function
    CaptureValue(RegisterIndex),
    /// Unconditional crash
    Crash,
    /// Insert this native function into the given register
    InsertNativeFunction(NativeFunction, RegisterIndex),
}

impl<A, B> OpCode<A, B>
where
    A: Clone,
    B: Clone,
{
    pub fn convert<C, D>(
        &self,
        a_to_c: &mut impl FnMut(A) -> C,
        b_to_d: &mut impl FnMut(B) -> D,
    ) -> OpCode<C, D> {
        match self {
            OpCode::Call(a, b) => OpCode::Call(b_to_d(a.clone()), b_to_d(b.clone())),
            OpCode::TailCall(a) => OpCode::TailCall(b_to_d(a.clone())),
            OpCode::CallArgument(a) => OpCode::CallArgument(b_to_d(a.clone())),
            OpCode::Return(a) => OpCode::Return(b_to_d(a.clone())),
            OpCode::Jump(a) => OpCode::Jump(a_to_c(a.clone())),
            OpCode::JumpToPositionIfFalse(a, b) => {
                OpCode::JumpToPositionIfFalse(b_to_d(a.clone()), a_to_c(b.clone()))
            }
            OpCode::CopyValue(a, b) => OpCode::CopyValue(b_to_d(a.clone()), b_to_d(b.clone())),
            OpCode::LoadConstant(a, b) => {
                OpCode::LoadConstant(a_to_c(a.clone()), b_to_d(b.clone()))
            }
            OpCode::CloseValue(a) => OpCode::CloseValue(a_to_c(a.clone())),
            OpCode::CreateClosure(a, b) => {
                OpCode::CreateClosure(a_to_c(a.clone()), b_to_d(b.clone()))
            }
            OpCode::CaptureValue(a) => OpCode::CaptureValue(b_to_d(a.clone())),
            OpCode::Crash => OpCode::Crash,
            OpCode::InsertNativeFunction(a, b) => {
                OpCode::InsertNativeFunction(a.clone(), b_to_d(b.clone()))
            }
        }
    }
}
