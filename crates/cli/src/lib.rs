pub mod cli;
pub mod config;
pub mod discovery;
pub mod error;
pub mod reader;
pub mod walker;

pub use cli::{CheckArgs, Cli, Command, InitArgs, OutputFormat, ReportArgs};
pub use config::IgnoreConfig;
pub use error::{Error, ExitCode, Result};
pub use reader::{FileContent, FileReader, ReadStrategy};
pub use walker::{FileWalker, WalkStats, WalkedFile, WalkerConfig};
