/// hyprhook — run scripts when Hyprland windows gain or lose focus.
///
/// Reads a TOML config file with [[window]] rules, each specifying optional
/// class/title regex patterns and lists of shell commands to run on focus
/// and blur events.
///
/// Config example (~/.config/hyprhook/config.toml):
///
///   [[window]]
///   class    = "gamescope"
///   title    = "Counter-Strike 2"
///   on_focus = ["gamemode start", "obs-cli start-recording"]
///   on_blur  = ["gamemode stop",  "obs-cli stop-recording"]
///
/// Each command is executed via `sh -c` and receives:
///   HYPRHOOK_WINDOW_CLASS  — class of the newly focused window
///   HYPRHOOK_WINDOW_TITLE  — title of the newly focused window

use std::sync::{Arc, Mutex};

use clap::Parser;
use hyprland::{data::Client, event_listener::AsyncEventListener, prelude::*};
use regex::Regex;
use serde::Deserialize;
use tokio::process::Command;

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "hyprhook",
    about = "Run scripts when Hyprland windows gain or lose focus"
)]
struct Args {
    /// Path to TOML config file.
    /// Defaults to $XDG_CONFIG_HOME/hyprhook/config.toml
    /// (falling back to ~/.config/hyprhook/config.toml).
    #[arg(short, long, value_name = "PATH")]
    config: Option<String>,
}

// ---------------------------------------------------------------------------
// Config (TOML)
// ---------------------------------------------------------------------------

#[derive(Deserialize, Default)]
struct Config {
    #[serde(default)]
    window: Vec<RuleConfig>,
}

#[derive(Deserialize)]
struct RuleConfig {
    class: Option<String>,
    title: Option<String>,
    #[serde(default)]
    on_focus: Vec<String>,
    #[serde(default)]
    on_blur: Vec<String>,
}

fn default_config_path() -> String {
    let base = std::env::var("XDG_CONFIG_HOME")
        .unwrap_or_else(|_| format!("{}/.config", std::env::var("HOME").unwrap_or_default()));
    format!("{}/hyprhook/config.toml", base)
}

fn load_config(path: &str) -> Config {
    let content = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("hyprhook: cannot read config {}: {}", path, e);
            std::process::exit(1);
        }
    };
    toml::from_str(&content).unwrap_or_else(|e| {
        eprintln!("hyprhook: config parse error in {}: {}", path, e);
        std::process::exit(1);
    })
}

// ---------------------------------------------------------------------------
// Rule
// ---------------------------------------------------------------------------

struct Rule {
    class: Option<Regex>,
    title: Option<Regex>,
    on_focus: Vec<String>,
    on_blur: Vec<String>,
}

impl Rule {
    fn from_config(cfg: RuleConfig) -> Result<Self, regex::Error> {
        Ok(Self {
            class: cfg.class.as_deref().map(Regex::new).transpose()?,
            title: cfg.title.as_deref().map(Regex::new).transpose()?,
            on_focus: cfg.on_focus,
            on_blur: cfg.on_blur,
        })
    }

    fn matches(&self, class: &str, title: &str) -> bool {
        self.class.as_ref().map_or(true, |r| r.is_match(class))
            && self.title.as_ref().map_or(true, |r| r.is_match(title))
    }
}

// ---------------------------------------------------------------------------
// Focus state
// ---------------------------------------------------------------------------

struct FocusState {
    class: String,
    title: String,
    /// Indices into `rules` that matched the current window.
    matched: Vec<usize>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn matching_indices(rules: &[Rule], class: &str, title: &str) -> Vec<usize> {
    rules
        .iter()
        .enumerate()
        .filter(|(_, r)| r.matches(class, title))
        .map(|(i, _)| i)
        .collect()
}

/// Spawn each command via `sh -c`. Fire-and-forget; errors go to stderr.
fn spawn_hooks(cmds: &[String], class: &str, title: &str) {
    for cmd in cmds {
        let cmd = cmd.clone();
        let class = class.to_owned();
        let title = title.to_owned();
        tokio::spawn(async move {
            eprintln!("hyprhook: running {:?}", cmd);
            let result = Command::new("sh")
                .arg("-c")
                .arg(&cmd)
                .env("HYPRHOOK_WINDOW_CLASS", &class)
                .env("HYPRHOOK_WINDOW_TITLE", &title)
                .status()
                .await;
            match result {
                Ok(s) if s.success() => {}
                Ok(s) => eprintln!("hyprhook: {:?} exited with {}", cmd, s),
                Err(e) => eprintln!("hyprhook: {:?} failed to spawn: {}", cmd, e),
            }
        });
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> hyprland::Result<()> {
    let args = Args::parse();
    let config_path = args.config.unwrap_or_else(default_config_path);
    let config = load_config(&config_path);

    let rules: Vec<Rule> = config
        .window
        .into_iter()
        .map(Rule::from_config)
        .collect::<Result<_, _>>()
        .unwrap_or_else(|e| {
            eprintln!("hyprhook: invalid regex in config: {}", e);
            std::process::exit(1);
        });

    eprintln!("hyprhook: loaded {} rule(s) from {}", rules.len(), config_path);

    let rules = Arc::new(rules);

    // Bootstrap: fire on_focus for whichever window is active right now.
    let initial = match Client::get_active() {
        Ok(Some(c)) => {
            let matched = matching_indices(&rules, &c.class, &c.title);
            for &i in &matched {
                spawn_hooks(&rules[i].on_focus, &c.class, &c.title);
            }
            FocusState { class: c.class, title: c.title, matched }
        }
        _ => FocusState { class: String::new(), title: String::new(), matched: vec![] },
    };

    let state: Arc<Mutex<FocusState>> = Arc::new(Mutex::new(initial));

    let mut listener = AsyncEventListener::new();

    {
        let rules = Arc::clone(&rules);
        let state = Arc::clone(&state);

        listener.add_active_window_changed_handler(move |data| {
            let rules = Arc::clone(&rules);
            let state = Arc::clone(&state);
            Box::pin(async move {
                let (new_class, new_title) = match data {
                    Some(w) => (w.class, w.title),
                    None => (String::new(), String::new()),
                };

                let new_matched = matching_indices(&rules, &new_class, &new_title);
                let mut st = state.lock().unwrap();

                // Rules that were active and no longer match → blur.
                for &i in &st.matched {
                    if !new_matched.contains(&i) {
                        spawn_hooks(&rules[i].on_blur, &st.class, &st.title);
                    }
                }
                // Rules that now match but didn't before → focus.
                for &i in &new_matched {
                    if !st.matched.contains(&i) {
                        spawn_hooks(&rules[i].on_focus, &new_class, &new_title);
                    }
                }

                st.class = new_class;
                st.title = new_title;
                st.matched = new_matched;
            })
        });
    }

    listener.start_listener_async().await
}
