pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

// Test module - should produce inline_cfg_test
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(1, 2), 3);
    }
}

// Test helper function - should produce cfg_test_helper
#[cfg(test)]
fn test_fixture() -> i32 {
    42
}

// Test-only impl block - should produce cfg_test_helper
#[cfg(test)]
impl super::Foo {
    fn for_test() -> Self {
        Foo
    }
}

// Test-only struct - should produce cfg_test_item
#[cfg(test)]
struct TestContext {
    value: i32,
}

pub struct Foo;
