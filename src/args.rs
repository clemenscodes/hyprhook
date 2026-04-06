use clap::Parser;

#[derive(Parser)]
#[command(
    name = "hyprhook",
    about = "Run scripts on Hyprland window lifecycle and focus events"
)]
pub struct Args {
    /// Path to TOML config file.
    /// Defaults to $XDG_CONFIG_HOME/hyprhook/config.toml
    #[arg(short, long, value_name = "PATH")]
    config: Option<String>,
}

impl Args {
    pub fn config_path(self) -> Option<String> {
        self.config
    }
}
