use clap::{Parser, Subcommand};
use clap_complete::Shell;

#[derive(Parser)]
#[command(
    name = "hyprhook",
    about = "Run commands on Hyprland window lifecycle and focus events"
)]
pub struct Args {
    /// Path to TOML config file.
    /// Defaults to $XDG_CONFIG_HOME/hyprhook/config.toml
    #[arg(short, long, value_name = "PATH")]
    config: Option<String>,

    #[command(subcommand)]
    subcommand: Option<Action>,
}

#[derive(Subcommand)]
pub enum Action {
    /// Print a shell completion script to stdout
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
}

impl Args {
    pub fn config_path(&self) -> Option<&str> {
        self.config.as_deref()
    }

    pub fn subcommand(&self) -> Option<&Action> {
        self.subcommand.as_ref()
    }
}
