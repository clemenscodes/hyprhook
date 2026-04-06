use tokio::{process::Command, sync::mpsc};
use tracing::{error, info, warn};

const HOOK_QUEUE_CAPACITY: usize = 64;

pub(crate) struct HookCommand {
    argv: Vec<String>,
}

pub struct HookSender {
    sender: mpsc::Sender<HookCommand>,
}

impl HookSender {
    pub fn enqueue(&self, argv: &[String]) {
        if argv.is_empty() {
            return;
        }
        let hook_command = HookCommand {
            argv: argv.to_vec(),
        };
        if self.sender.try_send(hook_command).is_err() {
            warn!(command = argv[0], "hook queue full, dropping command");
        }
    }
}

impl Clone for HookSender {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

pub fn create_hook_channel() -> (HookSender, mpsc::Receiver<HookCommand>) {
    let (sender, receiver) = mpsc::channel(HOOK_QUEUE_CAPACITY);
    (HookSender { sender }, receiver)
}

pub async fn run_hook_worker(mut receiver: mpsc::Receiver<HookCommand>) {
    while let Some(HookCommand { argv }) = receiver.recv().await {
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
