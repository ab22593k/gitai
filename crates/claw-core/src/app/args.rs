use clap::Args;
use clap::builder::{Styles, styling::AnsiColor};
use colored::Colorize;

#[derive(Args, Clone, Debug)]
pub struct MessageParams {
    #[arg(short, long, help = "Print the generated message to stdout and exit")]
    pub print: bool,

    #[arg(
        long,
        help = "Complete a commit message instead of generating from scratch"
    )]
    pub complete: bool,

    #[arg(
        long,
        help = "Prefix text to complete (required when using --complete)",
        requires = "complete"
    )]
    pub prefix: Option<String>,

    #[arg(
        long,
        help = "Context ratio for completion (0.0 to 1.0, default: 0.5)",
        requires = "complete",
        value_parser = parse_context_ratio
    )]
    pub context_ratio: Option<f32>,
}

pub fn get_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Magenta.on_default().bold())
        .usage(AnsiColor::Cyan.on_default().bold())
        .literal(AnsiColor::Green.on_default().bold())
        .placeholder(AnsiColor::Yellow.on_default())
        .valid(AnsiColor::Blue.on_default().bold())
        .invalid(AnsiColor::Red.on_default().bold())
        .error(AnsiColor::Red.on_default().bold())
}

#[must_use]
pub fn get_dynamic_help() -> String {
    let mut providers = crate::llm::engine::get_available_provider_names();
    providers.sort();

    let providers_list = providers
        .iter()
        .map(|p| format!("{}", (*p).bold()))
        .collect::<Vec<_>>()
        .join(" • ");

    format!("\nAvailable LLM Providers: {providers_list}")
}

fn parse_context_ratio(s: &str) -> Result<f32, String> {
    let val: f32 = s
        .parse()
        .map_err(|_| format!("'{s}' is not a valid number"))?;
    if !(0.0..=1.0).contains(&val) {
        return Err(format!(
            "context ratio must be between 0.0 and 1.0, got {val}"
        ));
    }
    Ok(val)
}
