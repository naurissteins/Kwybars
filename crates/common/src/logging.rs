use std::env;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

static INIT_RESULT: OnceLock<Result<(), String>> = OnceLock::new();
static FILE_GUARD: OnceLock<Mutex<Option<WorkerGuard>>> = OnceLock::new();

#[derive(Debug)]
pub enum LoggingInitError {
    Init(String),
}

impl Display for LoggingInitError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Init(msg) => write!(f, "{msg}"),
        }
    }
}

impl Error for LoggingInitError {}

pub fn init_logging(process_name: &str) -> Result<(), LoggingInitError> {
    let result =
        INIT_RESULT.get_or_init(|| setup_subscriber(process_name).map_err(|e| e.to_string()));
    match result {
        Ok(()) => Ok(()),
        Err(msg) => Err(LoggingInitError::Init(msg.clone())),
    }
}

fn setup_subscriber(process_name: &str) -> Result<(), Box<dyn Error>> {
    let log_filter = env::var("KWYBARS_LOG")
        .or_else(|_| env::var("RUST_LOG"))
        .unwrap_or_else(|_| "info".to_owned());

    let env_filter = EnvFilter::try_new(log_filter).unwrap_or_else(|_| EnvFilter::new("info"));

    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_writer(std::io::stderr);

    let (file_writer, file_guard) = match build_file_writer(process_name) {
        Ok(Some(value)) => (Some(value.0), Some(value.1)),
        Ok(None) => (None, None),
        Err(err) => {
            eprintln!("kwybars: log file setup failed, continuing with stderr logging only: {err}");
            (None, None)
        }
    };

    if let Some(guard) = file_guard {
        let slot = FILE_GUARD.get_or_init(|| Mutex::new(None));
        if let Ok(mut target) = slot.lock() {
            *target = Some(guard);
        }
    }

    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(stderr_layer);

    if let Some(file_writer) = file_writer {
        let file_layer = tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_target(false)
            .with_writer(file_writer);
        subscriber.with(file_layer).try_init()?;
    } else {
        subscriber.try_init()?;
    }

    Ok(())
}

fn build_file_writer(
    process_name: &str,
) -> Result<Option<(NonBlocking, WorkerGuard)>, std::io::Error> {
    let path = default_log_path(process_name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = OpenOptions::new().create(true).append(true).open(path)?;
    let output = tracing_appender::non_blocking(file);
    Ok(Some(output))
}

fn default_log_path(process_name: &str) -> PathBuf {
    if let Ok(path) = env::var("KWYBARS_LOG_FILE") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    if let Ok(state_home) = env::var("XDG_STATE_HOME") {
        return PathBuf::from(state_home)
            .join("kwybars")
            .join(format!("{process_name}.log"));
    }

    if let Ok(home) = env::var("HOME") {
        return PathBuf::from(home)
            .join(".local/state/kwybars")
            .join(format!("{process_name}.log"));
    }

    PathBuf::from(format!("kwybars-{process_name}.log"))
}
