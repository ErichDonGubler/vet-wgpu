use std::{fs, io::stdout, path::PathBuf};

use anyhow::Context;
use clap::Parser;
use log::LevelFilter;
use subcommand::{rev_list::RevListCommand, run::AnalyzeCommand};

mod analysis;
mod config;
pub(crate) mod data_provider;
mod subcommand;

/// User-guided analysis of the trust associated with a commit range.
#[derive(Debug, Parser)]
struct CliArgs {
    #[clap(long = "config", default_value = concat!("./", env!("CARGO_CRATE_NAME"), ".toml"))]
    config_path: PathBuf,
    #[clap(long)]
    gh_auth_token: Option<String>,
    #[clap(subcommand)]
    cmd: CliArgsSubcommand,
}

#[derive(Debug, Parser)]
enum CliArgsSubcommand {
    Analyze(AnalyzeCommand),
    /// Commands that have a 1:1 match with script invocations from this project's predecessor.
    #[clap(subcommand)]
    Compat(CompatCommand),
}

#[derive(Debug, Parser)]
enum CompatCommand {
    RevList(RevListCommand),
    // TODO: "fetch commits"
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_module(env!("CARGO_CRATE_NAME"), LevelFilter::Info)
        .parse_default_env()
        .init();

    let CliArgs {
        config_path,
        cmd,
        gh_auth_token,
    } = CliArgs::parse();

    // TODO: use `sh` and `json` files, like before
    let config = {
        // TODO: note path we attempted to access
        let config_toml = fs::read_to_string(&config_path).context("failed to read config file")?;
        // TODO: note path we attempted to access
        let config = toml::de::from_str(&config_toml)
            .context("failed to parse config from TOML config file")?;
        config
    };

    match cmd {
        CliArgsSubcommand::Analyze(cmd) => cmd.run(config, gh_auth_token).await,
        CliArgsSubcommand::Compat(cmd) => match cmd {
            CompatCommand::RevList(cmd) => {
                let stdout = stdout();
                cmd.run(config, Box::new(stdout.lock())).await
            }
        },
    }
}
