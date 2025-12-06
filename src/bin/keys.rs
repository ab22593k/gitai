use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use gait::core::llm;
use keyring::{Entry, Error as KeyringError};
use std::io::{self, Write};

#[derive(Parser)]
#[command(name = "git-keys", about = "Manage `gait` providers API keys")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Store an API key for a provider
    Set {
        /// Provider name (e.g. openai, anthropic)
        provider: String,
    },
    /// Retrieve an API key for a provider
    Get {
        /// Provider name
        provider: String,
    },
    /// Delete an API key for a provider
    Delete {
        /// Provider name
        provider: String,
    },
    /// List all configured providers
    List,
}

fn main() -> Result<()> {
    // Enable logging if env is set, or just use println for CLI debugging
    env_logger::try_init().ok();

    let cli = Cli::parse();

    match cli.command {
        Commands::Set { provider } => {
            let provider = provider.to_lowercase();

            print!("Enter API key for {}: ", provider.cyan());
            io::stdout().flush()?;

            let key = rpassword::read_password().context("Failed to read password from stdin")?;

            if key.trim().is_empty() {
                println!("{}", "API key cannot be empty".red());
                return Ok(());
            }

            match Entry::new("gait", &provider) {
                Ok(entry) => match entry.set_password(&key) {
                    Ok(()) => {
                        println!("{}", "API key saved successfully".green());
                    }
                    Err(e) => {
                        println!("{}", format!("Failed to set password: {e}").red());
                    }
                },
                Err(e) => {
                    println!("{}", format!("Failed to create keyring entry: {e}").red());
                }
            }
        }
        Commands::Get { provider } => {
            let provider = provider.to_lowercase();

            match Entry::new("gait", &provider) {
                Ok(entry) => match entry.get_password() {
                    Ok(key) => println!("{key}"),
                    Err(e) => {
                        println!("{}", format!("No API key found or error: {e}").red());
                    }
                },
                Err(e) => {
                    println!("{}", format!("Failed to create keyring entry: {e}").red());
                }
            }
        }
        Commands::Delete { provider } => {
            let provider = provider.to_lowercase();
            match Entry::new("gait", &provider) {
                Ok(entry) => match entry.delete_credential() {
                    Ok(()) => println!("{}", "API key deleted successfully".green()),
                    Err(e) => {
                        println!("{}", format!("Failed to delete password: {e}").red());
                    }
                },
                Err(e) => {
                    println!("{}", format!("Failed to create keyring entry: {e}").red());
                }
            }
        }
        Commands::List => {
            println!("Configured providers in keyring:");
            let providers = llm::get_available_provider_names();

            let mut found_any = false;

            for provider in providers {
                let provider = provider.to_lowercase();

                // Attempt to access each one
                match Entry::new("gait", &provider) {
                    Ok(entry) => match entry.get_password() {
                        Ok(_) => {
                            println!("  - {}", provider.green());
                            found_any = true;
                        }
                        Err(KeyringError::NoEntry) => {}
                        Err(e) => {
                            println!("  - {} (Error: {})", provider.yellow(), e);
                        }
                    },
                    Err(e) => {
                        println!("  - {} (Error accessing keyring: {})", provider.yellow(), e);
                    }
                }
            }

            if !found_any {
                println!("  (No keys found)");
            }
        }
    }

    Ok(())
}
