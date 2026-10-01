//! keys that legacy kwybars accepted and this version ignores on purpose

const NO_DAEMON: &str = "kwybars runs as a single process without a daemon";
const NO_NOTIFICATIONS: &str =
    "desktop notifications were removed, errors are in the log and `kwybars doctor`";
const NO_BACKENDS: &str = "audio is captured from PipeWire directly";

/// why a removed key no longer does anything, `None` for keys that were never valid
pub fn removed_reason(key_path: &str) -> Option<&'static str> {
    let (table, key) = key_path.rsplit_once('.')?;
    match (table, key) {
        ("daemon", "enabled" | "poll_interval_ms" | "stop_on_silence")
        | ("daemon", "overlay_command" | "overlay_args") => Some(NO_DAEMON),
        ("daemon", "notify_on_error" | "notify_cooldown_seconds") => Some(NO_NOTIFICATIONS),
        (table, key)
            if (table == "visualizer" || table.ends_with("].visualizer"))
                && (key == "backend" || key.starts_with("pipewire_")) =>
        {
            Some(NO_BACKENDS)
        }
        _ => None,
    }
}

/// one warning per reason, listing its keys in the order they were found
pub fn removed_warnings(removed: &[(String, &'static str)]) -> Vec<String> {
    let mut reasons: Vec<&'static str> = Vec::new();
    for (_, reason) in removed {
        if !reasons.contains(reason) {
            reasons.push(reason);
        }
    }
    reasons
        .into_iter()
        .map(|reason| {
            let keys: Vec<&str> = removed
                .iter()
                .filter(|(_, why)| *why == reason)
                .map(|(key, _)| key.as_str())
                .collect();
            format!("{}: removed ({reason}), ignored", keys.join(", "))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{NO_BACKENDS, NO_DAEMON, NO_NOTIFICATIONS, removed_reason, removed_warnings};

    #[test]
    fn classifies_removed_keys() {
        assert_eq!(removed_reason("daemon.overlay_command"), Some(NO_DAEMON));
        assert_eq!(
            removed_reason("daemon.notify_on_error"),
            Some(NO_NOTIFICATIONS)
        );
        assert_eq!(removed_reason("visualizer.backend"), Some(NO_BACKENDS));
        assert_eq!(
            removed_reason("visualizer.pipewire_gain"),
            Some(NO_BACKENDS)
        );
        assert_eq!(
            removed_reason("overlay.outputs[1].visualizer.pipewire_decay"),
            Some(NO_BACKENDS)
        );
        for other in [
            "daemon.typo",
            "overlay.backend",
            "backend",
            "visualizer.bakend",
        ] {
            assert_eq!(removed_reason(other), None, "{other}");
        }
    }

    #[test]
    fn groups_keys_by_reason() {
        let removed = [
            ("daemon.enabled".to_owned(), NO_DAEMON),
            ("visualizer.backend".to_owned(), NO_BACKENDS),
            ("daemon.overlay_args".to_owned(), NO_DAEMON),
        ];
        assert_eq!(
            removed_warnings(&removed),
            vec![
                format!("daemon.enabled, daemon.overlay_args: removed ({NO_DAEMON}), ignored"),
                format!("visualizer.backend: removed ({NO_BACKENDS}), ignored"),
            ]
        );
    }
}
