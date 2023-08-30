use std::fmt::Debug;

use crate::native_function::NativeFunction;

#[derive(Debug, Clone)]
pub enum OpCode<Uoo, V> {
    /// Call the function in register .0,
    /// Putting the result into the register .1
    /// Arguments are given after the call
    Call(V, V),
    /// Create a tail call with the function in the given register
    /// Followed by `CallArgument`s
    TailCall(V),

    CallArgument(V),

    /// Return the value in the given register
    Return(V),

    /// Unconditionally jump to the given position
    Jump(Uoo),
    /// Check the boolean .0, if false, jump to the given offset, otherwise continue
    JumpToOffsetIfFalse(T, T),
    /// Close the upvalue in the given position
    CloseUpValue(T),
    /// Copy the value from 0 to 1
    CopyValue(T, T),
    /// Load the constant from the constants array at 0 to the position 1
    LoadConstant(T, T),
    /// Load the upvalue in the given upvalue slot into the given slot
    LoadUpValue(T, T),
    /// Free the value at the given index
    CloseValue(T),
    /// Create closure. Takes the index of the function in the current chunk,
    /// puts the result in the register .1
    CreateClosure(T, T),
    //// Capture upvalues only ever appear after CreateClosure
    /// Capture an upvalue from a local in the function above
    CaptureUpValueFromLocal(T),
    /// Capture an upvalue from the above function's upvalues
    CaptureUpValueFromNonLocal(T),
    /// Unconditional crash
    Crash,
    /// Insert this native function into the given register
    InsertNativeFunction(NativeFunction, T),
}
