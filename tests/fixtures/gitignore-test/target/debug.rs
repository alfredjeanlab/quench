//! Build artifact - should be ignored by .gitignore.
//!
//! If this file is scanned, gitignore is not working.

pub fn should_be_ignored() {
    panic!("this file should never be scanned");
}
