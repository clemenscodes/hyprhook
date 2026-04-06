use crate::rule::Rule;
use serde::Deserialize;
use tracing::error;

#[derive(Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    window: Vec<RuleConfig>,
}

#[derive(Deserialize)]
struct RuleConfig {
    class: Option<String>,
    title: Option<String>,
    #[serde(default)]
    on_open: Vec<Vec<String>>,
    #[serde(default)]
    on_close: Vec<Vec<String>>,
    #[serde(default)]
    on_focus: Vec<Vec<String>>,
    #[serde(default)]
    on_unfocus: Vec<Vec<String>>,
}

impl Config {
    pub fn default_path() -> String {
        let base = std::env::var("XDG_CONFIG_HOME")
            .unwrap_or_else(|_| format!("{}/.config", std::env::var("HOME").unwrap_or_default()));
        format!("{base}/hyprhook/config.toml")
    }

    pub fn load(path: &str, explicit: bool) -> Self {
        let content = match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound && !explicit => {
                return Self::default();
            }
            Err(err) => {
                error!(%err, path, "cannot read config");
                std::process::exit(1);
            }
        };
        toml::from_str(&content).unwrap_or_else(|err| {
            error!(%err, path, "config parse error");
            std::process::exit(1);
        })
    }

    pub fn into_rules(self) -> Result<Vec<Rule>, regex::Error> {
        self.window
            .into_iter()
            .map(|rule_config| {
                Rule::new(
                    rule_config.class.as_deref(),
                    rule_config.title.as_deref(),
                    rule_config.on_open,
                    rule_config.on_close,
                    rule_config.on_focus,
                    rule_config.on_unfocus,
                )
            })
            .collect()
    }
}
