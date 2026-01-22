//! A CLI tool with shell script helpers.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        println!("Usage: mixed <command>");
        return;
    }
    match args[0].as_str() {
        "hello" => println!("Hello, world!"),
        "version" => println!("mixed 0.1.0"),
        _ => eprintln!("Unknown command: {}", args[0]),
    }
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
