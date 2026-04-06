use crate::rule::Rule;
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
        self.rule
            .into_iter()
            .map(|rule_config| {
                validate_commands(&rule_config.on_open, "on_open");
                validate_commands(&rule_config.on_close, "on_close");
                validate_commands(&rule_config.on_focus, "on_focus");
                validate_commands(&rule_config.on_unfocus, "on_unfocus");
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

fn validate_commands(commands: &[Vec<String>], field: &str) {
    for argv in commands {
        if argv.is_empty() {
            error!(field, "empty command in config — each command must have at least one element");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(toml: &str) -> Vec<Rule> {
        toml::from_str::<Config>(toml).unwrap().into_rules().unwrap()
    }

    #[test]
    fn empty_config_produces_no_rules() {
        let rules = parse("");
        assert!(rules.is_empty());
    }

    #[test]
    fn rule_with_class_and_title_compiles() {
        let rules = parse(r#"
            [[rule]]
            class = "^gamescope$"
            title = "Counter-Strike 2"
        "#);
        assert_eq!(rules.len(), 1);
        assert!(rules[0].matches("gamescope", "Counter-Strike 2"));
        assert!(!rules[0].matches("other", "Counter-Strike 2"));
    }

    #[test]
    fn rule_with_only_class_compiles() {
        let rules = parse(r#"
            [[rule]]
            class = "^firefox$"
        "#);
        assert_eq!(rules.len(), 1);
        assert!(rules[0].matches("firefox", "any title"));
        assert!(!rules[0].matches("other", "any title"));
    }

    #[test]
    fn commands_survive_round_trip() {
        let rules = parse(r#"
            [[rule]]
            class = "^gamescope$"
            on_focus = [["hyprctl", "dispatch", "submap", "gaming"]]
            on_unfocus = [["hyprctl", "dispatch", "submap", "reset"]]
        "#);
        assert_eq!(rules[0].on_focus(), &[vec![
            "hyprctl".to_owned(), "dispatch".to_owned(),
            "submap".to_owned(), "gaming".to_owned(),
        ]]);
        assert_eq!(rules[0].on_unfocus(), &[vec![
            "hyprctl".to_owned(), "dispatch".to_owned(),
            "submap".to_owned(), "reset".to_owned(),
        ]]);
        assert!(rules[0].on_open().is_empty());
        assert!(rules[0].on_close().is_empty());
    }

    #[test]
    fn multiple_rules_all_compile() {
        let rules = parse(r#"
            [[rule]]
            class = "^gamescope$"

            [[rule]]
            class = "^firefox$"
        "#);
        assert_eq!(rules.len(), 2);
    }

    #[test]
    fn multiple_commands_per_event_are_preserved() {
        let rules = parse(r#"
            [[rule]]
            class = "^foo$"
            on_open = [["cmd1", "arg1"], ["cmd2", "arg2"]]
        "#);
        assert_eq!(rules[0].on_open().len(), 2);
        assert_eq!(rules[0].on_open()[0][0], "cmd1");
        assert_eq!(rules[0].on_open()[1][0], "cmd2");
    }

    #[test]
    fn invalid_class_regex_returns_error() {
        let config: Config = toml::from_str(r#"
            [[rule]]
            class = "[invalid"
        "#).unwrap();
        assert!(config.into_rules().is_err());
    }

    #[test]
    fn invalid_title_regex_returns_error() {
        let config: Config = toml::from_str(r#"
            [[rule]]
            title = "[invalid"
        "#).unwrap();
        assert!(config.into_rules().is_err());
    }
}
