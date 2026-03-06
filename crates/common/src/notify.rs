use std::collections::HashMap;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use tracing::warn;

static LAST_NOTIFICATIONS: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();

pub fn notify_error_with_cooldown(
    key: &str,
    title: &str,
    body: &str,
    enabled: bool,
    cooldown: Duration,
) {
    if !enabled {
        return;
    }
    if notifications_globally_disabled() {
        return;
    }

    if !cooldown_gate_allows(key, cooldown) {
        return;
    }

    let status = Command::new("notify-send")
        .arg("--app-name=kwybars")
        .arg("--urgency=normal")
        .arg(title)
        .arg(body)
        .status();

    match status {
        Ok(exit) if exit.success() => {}
        Ok(exit) => warn!("kwybars: notify-send exited with status {exit}"),
        Err(err) => warn!("kwybars: could not invoke notify-send: {err}"),
    }
}

fn cooldown_gate_allows(key: &str, cooldown: Duration) -> bool {
    let now = Instant::now();
    let store = LAST_NOTIFICATIONS.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut guard) = store.lock() else {
        return true;
    };

    if let Some(last) = guard.get(key)
        && now.duration_since(*last) < cooldown
    {
        return false;
    }

    guard.insert(key.to_owned(), now);
    true
}

fn notifications_globally_disabled() -> bool {
    match std::env::var("KWYBARS_DISABLE_NOTIFICATIONS") {
        Ok(value) => {
            let normalized = value.trim().to_ascii_lowercase();
            matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
        }
        Err(_) => false,
    }
}
