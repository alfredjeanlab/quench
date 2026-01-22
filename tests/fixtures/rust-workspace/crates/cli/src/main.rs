//! CLI entry point.

use workspace_core::process;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    for arg in args {
        println!("{}", process(&arg));
    }
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
