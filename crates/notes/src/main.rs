use anyhow::Result;
use clap::{Args, Parser, crate_authors, crate_version};
use claw_core::{
    app::args::{get_dynamic_help, get_styles},
    common::CommonParams,
    init_app,
    output::print_error,
};
use notes::handle_release_notes_command;

#[derive(Args, Clone, Debug)]
struct NotesParams {
    #[arg(long, required = true)]
    from: String,

    #[arg(long)]
    to: Option<String>,

    #[arg(long, help = "Explicit version name to use in the release notes")]
    version_name: Option<String>,
}

#[derive(Parser)]
#[command(
    name = "git-notes",
    author = crate_authors!(),
    version = crate_version!(),
    about = "Generate release notes",
    after_help = get_dynamic_help(),
    styles = get_styles(),
)]
struct NotesArgs {
    #[command(flatten)]
    common: CommonParams,

    #[command(flatten)]
    params: NotesParams,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_app();

    let args = NotesArgs::parse();
    let NotesArgs { mut common, params } = args;
    let repository_url = std::mem::take(&mut common.repository_url);

    if let Err(e) = handle_release_notes_command(
        common,
        params.from,
        params.to,
        repository_url,
        params.version_name,
    )
    .await
    {
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
        NotesArgs::command().debug_assert();
    }
}
