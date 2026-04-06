use tokio::{process::Command, sync::mpsc};
use tracing::{info, warn, error};

const HOOK_QUEUE_CAPACITY: usize = 64;

pub struct HookCommand {
    command: String,
    class: String,
    title: String,
}

impl HookCommand {
    fn new(command: String, class: String, title: String) -> Self {
        Self { command, class, title }
    }
}

pub type HookSender = mpsc::Sender<HookCommand>;

pub fn create_hook_channel() -> (HookSender, mpsc::Receiver<HookCommand>) {
    mpsc::channel(HOOK_QUEUE_CAPACITY)
}

pub fn enqueue_hooks(hook_sender: &HookSender, commands: &[String], class: &str, title: &str) {
    for command in commands {
        let hook_command = HookCommand::new(command.clone(), class.to_owned(), title.to_owned());
        if hook_sender.try_send(hook_command).is_err() {
            warn!(command, "hook queue full, dropping command");
        }
    }
}

pub async fn run_hook_worker(mut hook_receiver: mpsc::Receiver<HookCommand>) {
    while let Some(HookCommand { command, class, title }) = hook_receiver.recv().await {
        info!(command, "running hook");
        let result = Command::new("/bin/sh")
            .arg("-c")
            .arg(&command)
            .env("HYPRHOOK_WINDOW_CLASS", &class)
            .env("HYPRHOOK_WINDOW_TITLE", &title)
            .status()
            .await;
        match result {
            Ok(status) if status.success() => {}
            Ok(status) => warn!(command, exit_status = %status, "hook exited with non-zero status"),
            Err(err) => error!(command, %err, "hook failed to spawn"),
        }
    }
}
