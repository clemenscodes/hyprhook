use tokio::{process::Command, sync::mpsc};
use tracing::{error, info, warn};

const HOOK_QUEUE_CAPACITY: usize = 64;

pub struct HookCommand {
    argv: Vec<String>,
}

pub type HookSender = mpsc::Sender<HookCommand>;

pub fn create_hook_channel() -> (HookSender, mpsc::Receiver<HookCommand>) {
    mpsc::channel(HOOK_QUEUE_CAPACITY)
}

pub fn enqueue_hooks(hook_sender: &HookSender, commands: &[Vec<String>]) {
    for argv in commands {
        if argv.is_empty() {
            warn!("skipping empty command");
            continue;
        }
        let hook_command = HookCommand { argv: argv.clone() };
        if hook_sender.try_send(hook_command).is_err() {
            warn!(command = argv[0], "hook queue full, dropping command");
        }
    }
}

pub async fn run_hook_worker(mut hook_receiver: mpsc::Receiver<HookCommand>) {
    while let Some(HookCommand { argv }) = hook_receiver.recv().await {
        let Some((program, args)) = argv.split_first() else {
            continue;
        };
        info!(command = program, "running hook");
        let result = Command::new(program).args(args).status().await;
        match result {
            Ok(status) if status.success() => {}
            Ok(status) => {
                warn!(command = program, exit_status = %status, "hook exited with non-zero status")
            }
            Err(err) => error!(command = program, %err, "hook failed to spawn"),
        }
    }
}
