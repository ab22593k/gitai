use anyhow::Result;
use clap::Parser;
use gitai::{
    app::{self, WireArgs},
    init_logger, init_tracing_to_file,
    ui::print_error,
};

#[derive(Parser)]
#[command(
    name = "git-wire",
    version,
    author,
    about = "Synchronize code from remote repositories",
    styles = app::get_styles(),
)]
struct WireCli {
    #[command(flatten)]
    args: WireArgs,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logger();
    init_tracing_to_file();

    let cli = WireCli::parse();

    if let Err(e) = app::handle_wire(cli.args).await {
        print_error(&format!("Error: {e}"));
        std::process::exit(1);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        WireCli::command().debug_assert();
    }
}
