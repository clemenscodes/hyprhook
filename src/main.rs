/// hyprhook — run scripts on Hyprland window lifecycle and focus events.
///
/// Config example (~/.config/hyprhook/config.toml):
///
///   [[window]]
///   class     = "gamescope"
///   title     = "Counter-Strike 2"
///   on_open   = ["obs-cli start-recording"]
///   on_close  = ["obs-cli stop-recording"]
///   on_focus  = ["hyprctl dispatch submap gaming"]
///   on_unfocus = ["hyprctl dispatch submap reset"]
///
/// All four event types are optional — omit any you don't need.
///
/// Each command runs via `sh -c` with:
///   HYPRHOOK_WINDOW_CLASS  — window class
///   HYPRHOOK_WINDOW_TITLE  — window title

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use clap::Parser;
use hyprland::{
    data::{Client, Clients},
    event_listener::AsyncEventListener,
    prelude::*,
};
use regex::Regex;
use serde::Deserialize;
use tokio::process::Command;

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "hyprhook",
    about = "Run scripts on Hyprland window lifecycle and focus events"
)]
struct Args {
    /// Path to TOML config file.
    /// Defaults to $XDG_CONFIG_HOME/hyprhook/config.toml
    #[arg(short, long, value_name = "PATH")]
    config: Option<String>,
}

// ---------------------------------------------------------------------------
// Config
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
    on_open: Vec<String>,
    #[serde(default)]
    on_close: Vec<String>,
    #[serde(default)]
    on_focus: Vec<String>,
    #[serde(default)]
    on_unfocus: Vec<String>,
}

fn default_config_path() -> String {
    let base = std::env::var("XDG_CONFIG_HOME")
        .unwrap_or_else(|_| format!("{}/.config", std::env::var("HOME").unwrap_or_default()));
    format!("{}/hyprhook/config.toml", base)
}

fn load_config(path: &str, explicit: bool) -> Config {
    let content = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && !explicit => {
            // Default path doesn't exist — that's fine, run with no rules.
            return Config::default();
        }
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
    on_open: Vec<String>,
    on_close: Vec<String>,
    on_focus: Vec<String>,
    on_unfocus: Vec<String>,
}

impl Rule {
    fn from_config(cfg: RuleConfig) -> Result<Self, regex::Error> {
        Ok(Self {
            class: cfg.class.as_deref().map(Regex::new).transpose()?,
            title: cfg.title.as_deref().map(Regex::new).transpose()?,
            on_open: cfg.on_open,
            on_close: cfg.on_close,
            on_focus: cfg.on_focus,
            on_unfocus: cfg.on_unfocus,
        })
    }

    fn matches(&self, class: &str, title: &str) -> bool {
        self.class.as_ref().map_or(true, |r| r.is_match(class))
            && self.title.as_ref().map_or(true, |r| r.is_match(title))
    }
}

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

struct State {
    /// All currently open windows: address → (class, title).
    /// Populated at startup and kept in sync via open/close events.
    open: HashMap<String, (String, String)>,
    /// Class/title of the currently focused window.
    focused_class: String,
    focused_title: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn matching_rules<'a>(rules: &'a [Rule], class: &str, title: &str) -> Vec<&'a Rule> {
    rules.iter().filter(|r| r.matches(class, title)).collect()
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
    let explicit = args.config.is_some();
    let config_path = args.config.unwrap_or_else(default_config_path);
    let config = load_config(&config_path, explicit);

    let rules: Arc<Vec<Rule>> = Arc::new(
        config
            .window
            .into_iter()
            .map(Rule::from_config)
            .collect::<Result<_, _>>()
            .unwrap_or_else(|e| {
                eprintln!("hyprhook: invalid regex in config: {}", e);
                std::process::exit(1);
            }),
    );

    eprintln!("hyprhook: loaded {} rule(s) from {}", rules.len(), config_path);

    // Seed the open-windows map from whatever is already running.
    let mut open: HashMap<String, (String, String)> = HashMap::new();
    if let Ok(clients) = Clients::get() {
        for c in clients.iter() {
            open.insert(c.address.to_string(), (c.class.clone(), c.title.clone()));
        }
    }

    // Seed focused window.
    let (focused_class, focused_title) = match Client::get_active() {
        Ok(Some(c)) => (c.class, c.title),
        _ => (String::new(), String::new()),
    };

    let state: Arc<Mutex<State>> = Arc::new(Mutex::new(State {
        open,
        focused_class,
        focused_title,
    }));

    let mut listener = AsyncEventListener::new();

    // --- window opened -------------------------------------------------------
    {
        let rules = Arc::clone(&rules);
        let state = Arc::clone(&state);
        listener.add_window_opened_handler(move |data| {
            let rules = Arc::clone(&rules);
            let state = Arc::clone(&state);
            Box::pin(async move {
                let class = data.window_class;
                let title = data.window_title;
                let addr = data.window_address.to_string();
                state
                    .lock()
                    .unwrap()
                    .open
                    .insert(addr, (class.clone(), title.clone()));
                for rule in matching_rules(&rules, &class, &title) {
                    spawn_hooks(&rule.on_open, &class, &title);
                }
            })
        });
    }

    // --- window closed -------------------------------------------------------
    {
        let rules = Arc::clone(&rules);
        let state = Arc::clone(&state);
        listener.add_window_closed_handler(move |addr| {
            let rules = Arc::clone(&rules);
            let state = Arc::clone(&state);
            Box::pin(async move {
                let key = addr.to_string();
                let entry = state.lock().unwrap().open.remove(&key);
                if let Some((class, title)) = entry {
                    for rule in matching_rules(&rules, &class, &title) {
                        spawn_hooks(&rule.on_close, &class, &title);
                    }
                }
            })
        });
    }

    // --- focus changed -------------------------------------------------------
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

                let (old_class, old_title) = {
                    let mut st = state.lock().unwrap();
                    let prev = (st.focused_class.clone(), st.focused_title.clone());
                    st.focused_class = new_class.clone();
                    st.focused_title = new_title.clone();
                    prev
                };

                // Unfocus: rules that matched the old window.
                for rule in matching_rules(&rules, &old_class, &old_title) {
                    spawn_hooks(&rule.on_unfocus, &old_class, &old_title);
                }
                // Focus: rules that match the new window.
                for rule in matching_rules(&rules, &new_class, &new_title) {
                    spawn_hooks(&rule.on_focus, &new_class, &new_title);
                }
            })
        });
    }

    listener.start_listener_async().await
}
