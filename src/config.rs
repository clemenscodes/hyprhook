use crate::rule::{Rule, RuleSet};
use serde::Deserialize;
use tracing::error;

#[derive(Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    rule: Vec<RuleConfig>,
}

#[derive(Deserialize)]
struct RuleConfig {
    class: Option<String>,
    title: Option<String>,
    #[serde(default)]
    on_open: Vec<String>,
    #[serde(default)]
    on_close: Vec<String>,
    #[serde(default)]
    on_focus: Vec<String>,
    #[serde(default)]
    on_unfocus: Vec<String>,
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

    pub fn into_rules(self) -> Result<RuleSet, regex::Error> {
        let rules = self
            .rule
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
            .collect();
        RuleSet::new(rules)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(toml: &str) -> RuleSet {
        toml::from_str::<Config>(toml)
            .unwrap()
            .into_rules()
            .unwrap()
    }

    #[test]
    fn empty_config_produces_no_rules() {
        assert!(parse("").is_empty());
    }

    #[test]
    fn rule_with_class_and_title_compiles() {
        let set = parse(
            r#"
            [[rule]]
            class = "^gamescope$"
            title = "Counter-Strike 2"
        "#,
        );
        assert_eq!(set.len(), 1);
        assert_eq!(set.matching("gamescope", "Counter-Strike 2").len(), 1);
        assert!(set.matching("other", "Counter-Strike 2").is_empty());
    }

    #[test]
    fn rule_with_only_class_compiles() {
        let set = parse(
            r#"
            [[rule]]
            class = "^firefox$"
        "#,
        );
        assert_eq!(set.len(), 1);
        assert_eq!(set.matching("firefox", "any title").len(), 1);
        assert!(set.matching("other", "any title").is_empty());
    }

    #[test]
    fn commands_survive_round_trip() {
        let set = parse(
            r#"
            [[rule]]
            class = "^gamescope$"
            on_focus = ["hyprctl", "dispatch", "submap", "gaming"]
            on_unfocus = ["hyprctl", "dispatch", "submap", "reset"]
        "#,
        );
        let matched = set.matching("gamescope", "anything");
        assert_eq!(
            matched[0].on_focus(),
            &[
                "hyprctl".to_owned(),
                "dispatch".to_owned(),
                "submap".to_owned(),
                "gaming".to_owned(),
            ]
        );
        assert_eq!(
            matched[0].on_unfocus(),
            &[
                "hyprctl".to_owned(),
                "dispatch".to_owned(),
                "submap".to_owned(),
                "reset".to_owned(),
            ]
        );
        assert!(matched[0].on_open().is_empty());
        assert!(matched[0].on_close().is_empty());
    }

    #[test]
    fn multiple_rules_all_compile() {
        let set = parse(
            r#"
            [[rule]]
            class = "^gamescope$"

            [[rule]]
            class = "^firefox$"
        "#,
        );
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn invalid_class_regex_returns_error() {
        let config: Config = toml::from_str(
            r#"
            [[rule]]
            class = "[invalid"
        "#,
        )
        .unwrap();
        assert!(config.into_rules().is_err());
    }

    #[test]
    fn invalid_title_regex_returns_error() {
        let config: Config = toml::from_str(
            r#"
            [[rule]]
            title = "[invalid"
        "#,
        )
        .unwrap();
        assert!(config.into_rules().is_err());
    }
}
