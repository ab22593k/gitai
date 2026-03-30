pub mod args;
pub mod handlers;

pub use args::{Cli, Gitai, parse_args};
pub use handlers::handle_command;
