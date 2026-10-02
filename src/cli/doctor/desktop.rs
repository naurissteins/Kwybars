//! the session, the compositor's protocols, and the outputs

use std::ffi::OsString;

use crate::cli::report::Report;
use crate::config::{Config, OverlayConfig, ShowOn};
use crate::wayland::{Probe, ProbedOutput};

pub fn session(report: &mut Report, env: &dyn Fn(&str) -> Option<OsString>) {
    let text = |name: &str| {
        env(name)
            .map(|value| value.to_string_lossy().into_owned())
            .filter(|value| !value.is_empty())
    };
    let kind = text("XDG_SESSION_TYPE").unwrap_or_else(|| "unknown".to_owned());
    report.line(match text("WAYLAND_DISPLAY") {
        Some(display) => format!("session: {kind} (WAYLAND_DISPLAY={display})"),
        None => format!("session: {kind} (WAYLAND_DISPLAY not set)"),
    });
}

pub fn compositor(report: &mut Report, probe: Result<&Probe, &String>, config: Option<&Config>) {
    let probe = match probe {
        Ok(probe) => probe,
        Err(err) => {
            report.error(format!("wayland: {err}"));
            return;
        }
    };
    report.line("wayland: connected");
    if probe.layer_shell {
        report.line("layer shell: supported");
    } else {
        report.error(
            "layer shell: the compositor does not support wlr-layer-shell, which Kwybars needs to place its bars on the desktop",
        );
    }
    report.line(if probe.fractional_scale {
        "fractional scale: supported"
    } else {
        "fractional scale: not supported, whole-number scales are used"
    });
    let default = OverlayConfig::default();
    outputs(
        report,
        probe,
        config.map_or(&default, |config| &config.overlay),
    );
}

fn outputs(report: &mut Report, probe: &Probe, overlay: &OverlayConfig) {
    if probe.outputs.is_empty() {
        report.error("outputs: the compositor reports none");
        return;
    }
    let (selected, warnings) = probe.selected(overlay);
    for (output, selected) in probe.outputs.iter().zip(&selected) {
        report.line(output_line(output, *selected));
    }
    for warning in warnings {
        report.warning(warning);
    }
    if !selected.contains(&true) {
        report.error("outputs: no connected output gets an overlay with this config");
        return;
    }
    // the runtime falls back to the primary output without saying so
    let ShowOn::Named(wanted) = &overlay.show_on else {
        return;
    };
    let connected = |name: &String| {
        matches!(name.as_str(), "primary")
            || name.trim_start_matches("index:").parse::<usize>().is_ok()
            || probe
                .outputs
                .iter()
                .any(|output| output.name.as_deref() == Some(name.as_str()))
    };
    for missing in wanted.iter().filter(|name| !connected(name)) {
        report.warning(format!(
            "overlay.show_on: no connected output named {missing:?}"
        ));
    }
}

fn output_line(output: &ProbedOutput, selected: bool) -> String {
    let size = match output.mode {
        Some((width, height)) => format!("{width}x{height}"),
        None => "unknown size".to_owned(),
    };
    format!(
        "output {}: {size}, scale {}, {}",
        output.label,
        output.scale(),
        if selected { "overlay" } else { "no overlay" }
    )
}
