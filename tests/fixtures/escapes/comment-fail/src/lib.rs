pub fn risky_op(ptr: *const i32) -> i32 {
    unsafe { *ptr }  // Missing SAFETY comment
}
