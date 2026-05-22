use std::io::{self, BufRead, BufReader, ErrorKind, Read};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::display::GraphicalEnvironment;
use kwybars_common::config::DaemonConfig;
use kwybars_engine::ipc::FRAME_SOCKET_ENV;
use tracing::{info, warn};

const SPAWN_RETRY_INTERVAL: Duration = Duration::from_millis(1800);
const DISPLAY_WARNING_INTERVAL: Duration = Duration::from_secs(30);

pub struct OverlayProcess {
    child: Option<Child>,
    last_spawn_attempt: Option<Instant>,
    last_display_warning: Option<Instant>,
}

impl OverlayProcess {
    pub fn new() -> Self {
        Self {
            child: None,
            last_spawn_attempt: None,
            last_display_warning: None,
        }
    }

    pub fn ensure_running(
        &mut self,
        daemon: &DaemonConfig,
        config_path: &Path,
        frame_socket_path: Option<&Path>,
        now: Instant,
    ) -> io::Result<()> {
        if self.child.is_some() {
            return Ok(());
        }
        if self
            .last_spawn_attempt
            .is_some_and(|last| now.duration_since(last) < SPAWN_RETRY_INTERVAL)
        {
            return Ok(());
        }

        self.last_spawn_attempt = Some(now);
        let Some(graphical_environment) = GraphicalEnvironment::detect() else {
            self.warn_missing_display(now);
            return Ok(());
        };

        let mut command = build_command(
            daemon,
            config_path,
            frame_socket_path,
            &graphical_environment,
        );
        let mut child = command.spawn()?;
        if let Some(stderr) = child.stderr.take() {
            spawn_overlay_stderr_forwarder(stderr);
        }
        self.child = Some(child);
        info!(
            "kwybars-daemon: started overlay process ({})",
            daemon.overlay_command
        );
        Ok(())
    }

    fn warn_missing_display(&mut self, now: Instant) {
        if self
            .last_display_warning
            .is_some_and(|last| now.duration_since(last) < DISPLAY_WARNING_INTERVAL)
        {
            return;
        }

        self.last_display_warning = Some(now);
        warn!(
            "kwybars-daemon: cannot start overlay yet; WAYLAND_DISPLAY/DISPLAY is missing and no Wayland socket was found"
        );
    }

    pub fn poll_exit(&mut self) -> io::Result<Option<ExitStatus>> {
        let Some(child) = self.child.as_mut() else {
            return Ok(None);
        };

        let maybe_exit = child.try_wait()?;
        if maybe_exit.is_some() {
            self.child = None;
        }

        Ok(maybe_exit)
    }

    pub fn is_running(&self) -> bool {
        self.child.is_some()
    }

    pub fn stop(&mut self) -> io::Result<()> {
        let Some(mut child) = self.child.take() else {
            return Ok(());
        };

        if let Err(err) = child.kill() {
            // Process might have just exited between poll and stop.
            if err.kind() != ErrorKind::InvalidInput {
                return Err(err);
            }
        }
        let _ = child.wait();
        info!("kwybars-daemon: stopped overlay process");
        Ok(())
    }
}

impl Drop for OverlayProcess {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn build_command(
    daemon: &DaemonConfig,
    config_path: &Path,
    frame_socket_path: Option<&Path>,
    graphical_environment: &GraphicalEnvironment,
) -> Command {
    let command_name = if daemon.overlay_command.trim().is_empty() {
        "kwybars-overlay"
    } else {
        daemon.overlay_command.trim()
    };

    let mut command = Command::new(command_name);
    if !daemon.overlay_args.is_empty() {
        command.args(&daemon.overlay_args);
    }
    command.env("KWYBARS_CONFIG", config_path);
    command.env("KWYBARS_DISABLE_NOTIFICATIONS", "1");
    graphical_environment.apply_to(&mut command);
    if let Some(frame_socket_path) = frame_socket_path {
        command.env(FRAME_SOCKET_ENV, frame_socket_path);
    }
    command.stdin(Stdio::null());
    command.stderr(Stdio::piped());
    command
}

fn spawn_overlay_stderr_forwarder(stderr: impl Read + Send + 'static) {
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line_result in reader.lines() {
            let Ok(line) = line_result else {
                break;
            };
            if should_suppress_gtk_warning(&line) {
                continue;
            }
            eprintln!("{line}");
        }
    });
}

fn should_suppress_gtk_warning(line: &str) -> bool {
    line.contains("Unknown key gtk-menu-images in ")
        || line.contains("Unknown key gtk-button-images in ")
}
