//! A library with proper documentation.

/// Returns health status.
pub fn health() -> &'static str {
    "ok"
}

/// Returns version.
pub fn version() -> &'static str {
    "0.1.0"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health() {
        assert_eq!(health(), "ok");
    }
}
