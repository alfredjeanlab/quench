pub fn safe_op(ptr: *const i32) -> i32 {
    // SAFETY: Caller guarantees ptr is valid
    unsafe { *ptr }
}
