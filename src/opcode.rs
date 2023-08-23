use std::fmt::Debug;

#[derive(Debug, Clone)]
pub enum OpCode<T: Into<usize> + Debug + Clone + TryFrom<usize>> {
    Add(T, T, T),
    /// Call the function in temporary storage, with arguments,
    /// Putting the result into the given slot
    Call(T),
    /// Save the value in the given position
    /// to the VM's temporary storage
    Save(T),
    /// Dump the values from the given position
    /// in the VM's temporary storage to the position starting with
    Dump(T, T),
    /// Return the value in the given register
    Return(T),
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
    /// Capture an upvalue from a local in the function above
    CaptureUpValueFromLocal(T),
    /// Capture an upvalue from the above function's upvalues
    CaptureUpValueFromNonLocal(T),
}
