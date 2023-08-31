use std::fmt::Debug;

use crate::native_function::NativeFunction;

#[derive(Debug, Clone)]
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
    /// Check the boolean in .0, if false, jump to the given offset, otherwise continue
    JumpToOffsetIfFalse(RegisterIndex, VecIndex),
    /// Copy the value from 0 to 1
    CopyValue(RegisterIndex, RegisterIndex),
    /// Load the constant from the constants array at 0 to the position 1
    LoadConstant(VecIndex, RegisterIndex),
    /// Free the value at the given register
    CloseValue(RegisterIndex),
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
