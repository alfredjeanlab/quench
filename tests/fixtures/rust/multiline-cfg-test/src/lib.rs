pub fn source_code() -> i32 {
    42
}

#[cfg(
    test
)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(super::source_code(), 42);
    }
}
