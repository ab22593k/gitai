use anyhow::Result;
use clap::{Parser, crate_authors, crate_version};
use claw_core::{
    app::{
        args::{self, NotesParams},
        handlers,
    },
    common::CommonParams,
    init_app,
    output::print_error,
};

#[derive(Parser)]
#[command(
    name = "git-notes",
    author = crate_authors!(),
    version = crate_version!(),
    about = "Generate release notes",
    after_help = args::get_dynamic_help(),
    styles = args::get_styles(),
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

    if let Err(e) = handlers::handle_notes(
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
