//! whether the systemd user unit starts Kwybars

use std::process::{Command, Stdio};

const UNIT: &str = "kwybars.service";

pub fn query() -> Option<String> {
    let output = Command::new("systemctl")
        .args(["--user", "is-enabled", UNIT])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// starting from the compositor's autostart is as good, so no state is an error
pub fn line(state: Option<&str>) -> String {
    match state {
        None => format!("service: systemctl is not available, {UNIT} not checked"),
        // older systemd prints nothing for a unit it does not know
        Some("" | "not-found") => format!("service: {UNIT} is not installed"),
        Some("disabled") => format!("service: {UNIT} is installed, not enabled"),
        Some(state) => format!("service: {UNIT} is {state}"),
    }
}
