use gitai::cli::{handle_command, parse_args};
use gitai::init_app;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = parse_args();

    init_app();

    if cli.log {
        let log_file = cli.log_file.unwrap_or_else(|| "gitai.log".to_string());
        setup_file_logging(&log_file)?;
    }

    let repository_url = cli.repository_url.clone();

    if cli.version {
        println!("gitai version: {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if let Some(command) = cli.command {
        handle_command(command, repository_url).await?;
    } else {
        println!("No command specified. Run with --help for usage information.");
    }

    Ok(())
}

fn setup_file_logging(log_file: &str) -> anyhow::Result<()> {
    if Path::new(log_file).exists() {
        let mut file = OpenOptions::new().append(true).open(log_file)?;
        writeln!(
            file,
            "\n--- Logging started at {} ---",
            chrono::Local::now()
        )?;
    }
    Ok(())
}
