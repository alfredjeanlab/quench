//! Output formatting for check results.

pub mod json;
pub mod text;

/// Output formatting options.
#[derive(Debug, Clone)]
pub struct FormatOptions {
    /// Maximum violations to show (None = unlimited).
    pub limit: Option<usize>,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            limit: Some(15), // Default per spec
        }
    }
}

impl FormatOptions {
    /// Create options with no limit.
    pub fn no_limit() -> Self {
        Self { limit: None }
    }

    /// Create options with a specific limit.
    pub fn with_limit(limit: usize) -> Self {
        Self { limit: Some(limit) }
    }
}
