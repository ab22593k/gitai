use anyhow::Result;
use clap::Parser;
use gitai::{
    app::{
        args::{self, WireArgs},
        handlers,
    },
    init_app,
    ui::print_error,
};

#[derive(Parser)]
#[command(
    name = "git-wire",
    version,
    author,
    about = "Synchronize code from remote repositories",
    styles = args::get_styles(),
)]
struct WireCli {
    #[command(flatten)]
    args: WireArgs,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_app();

    let cli = WireCli::parse();

    if let Err(e) = handlers::handle_wire(cli.args).await {
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
