//! hyprhook — run commands on Hyprland window lifecycle and focus events.
//!
//! Config example (~/.config/hyprhook/config.toml):
//!
//!   [[rule]]
//!   class     = "gamescope"
//!   title     = "Counter-Strike 2"
//!   on_open   = [["obs-cli", "start-recording"]]
//!   on_close  = [["obs-cli", "stop-recording"]]
//!   on_focus  = [["hyprctl", "dispatch", "submap", "gaming"]]
//!   on_unfocus = [["hyprctl", "dispatch", "submap", "reset"]]
//!
//! All four event types are optional — omit any you don't need.
//!
//! Each command is an argv list: the first element is the executable
//! (use an absolute path when not on PATH), the rest are arguments.

mod args;
mod config;
mod hook;
mod rule;
mod state;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use args::{Action, Args};
use clap::{CommandFactory, Parser};
use config::Config;
use hook::{create_hook_channel, run_hook_worker};
use hyprland::{
    data::{Client, Clients},
    event_listener::AsyncEventListener,
    prelude::*,
};
use rule::Rule;
use state::{State, WindowInfo};
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> hyprland::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("hyprhook=info")),
        )
        .init();

    let args = Args::parse();

    if let Some(Action::Completions { shell }) = args.subcommand() {
        clap_complete::generate(*shell, &mut Args::command(), "hyprhook", &mut std::io::stdout());
        return Ok(());
    }

    let config_path_override = args.config_path().map(str::to_owned);
    let is_explicit = config_path_override.is_some();
    let config_path = config_path_override.unwrap_or_else(Config::default_path);
    let config = Config::load(&config_path, is_explicit);

    let rules: Arc<Vec<Rule>> = Arc::new(config.into_rules().unwrap_or_else(|error| {
        error!(%error, "invalid regex in config");
        std::process::exit(1);
    }));

    info!(count = rules.len(), config = %config_path, "loaded rules");

    if rules.is_empty() {
        warn!("no rules configured, nothing to do");
        return Ok(());
    }

    let mut open: HashMap<String, WindowInfo> = HashMap::new();
    if let Ok(clients) = Clients::get() {
        for client in clients.iter() {
            let info = WindowInfo::new(client.class.clone(), client.title.clone());
            open.insert(client.address.to_string(), info);
        }
    }

    let focused = match Client::get_active() {
        Ok(Some(client)) => WindowInfo::new(client.class, client.title),
        _ => WindowInfo::default(),
    };

    let state: Arc<Mutex<State>> = Arc::new(Mutex::new(State::new(open, focused)));

    let (hook_sender, hook_receiver) = create_hook_channel();
    tokio::spawn(run_hook_worker(hook_receiver));

    let mut listener = AsyncEventListener::new();

    {
        let rules = Arc::clone(&rules);
        let state = Arc::clone(&state);
        let hook_sender = hook_sender.clone();
        listener.add_window_opened_handler(move |data| {
            let rules = Arc::clone(&rules);
            let state = Arc::clone(&state);
            let hook_sender = hook_sender.clone();
            Box::pin(async move {
                let class = data.window_class;
                let title = data.window_title;
                let address = data.window_address.to_string();
                let info = WindowInfo::new(class.clone(), title.clone());
                state.lock().unwrap().insert_open(address, info);
                for rule in Rule::matching(&rules, &class, &title) {
                    hook_sender.enqueue(rule.on_open());
                }
            })
        });
    }

    {
        let rules = Arc::clone(&rules);
        let state = Arc::clone(&state);
        let hook_sender = hook_sender.clone();
        listener.add_window_closed_handler(move |address| {
            let rules = Arc::clone(&rules);
            let state = Arc::clone(&state);
            let hook_sender = hook_sender.clone();
            Box::pin(async move {
                let key = address.to_string();
                if let Some(info) = state.lock().unwrap().remove_open(&key) {
                    for rule in Rule::matching(&rules, info.class(), info.title()) {
                        hook_sender.enqueue(rule.on_close());
                    }
                }
            })
        });
    }

    {
        let rules = Arc::clone(&rules);
        let state = Arc::clone(&state);
        let hook_sender = hook_sender.clone();
        listener.add_active_window_changed_handler(move |data| {
            let rules = Arc::clone(&rules);
            let state = Arc::clone(&state);
            let hook_sender = hook_sender.clone();
            Box::pin(async move {
                let new_info = match data {
                    Some(window) => WindowInfo::new(window.class, window.title),
                    None => WindowInfo::default(),
                };
                let new_class = new_info.class().to_owned();
                let new_title = new_info.title().to_owned();
                let previous_info = state.lock().unwrap().update_focus(new_info);
                for rule in Rule::matching(&rules, previous_info.class(), previous_info.title()) {
                    hook_sender.enqueue(rule.on_unfocus());
                }
                for rule in Rule::matching(&rules, &new_class, &new_title) {
                    hook_sender.enqueue(rule.on_focus());
                }
            })
        });
    }

    listener.start_listener_async().await.map_err(|error| {
        error!(
            %error,
            "failed to connect to Hyprland IPC socket — is Hyprland running? Is HYPRLAND_INSTANCE_SIGNATURE set?"
        );
        error
    })
}
