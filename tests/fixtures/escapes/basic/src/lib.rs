pub fn risky(opt: Option<i32>) -> i32 {
    opt.unwrap()  // Line 2: violation
}
