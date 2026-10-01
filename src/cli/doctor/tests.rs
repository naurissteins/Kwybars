use super::{audio, desktop, service, settings};
use crate::audio::capture::{CaptureState, StatusSnapshot};
use crate::cli::report::Report;
use crate::config::{Config, OutputConfig, ShowOn};
use crate::wayland::{Probe, ProbedOutput};
use crate::xdg::fake_env;

fn output(name: &str) -> ProbedOutput {
    ProbedOutput {
        name: Some(name.to_owned()),
        label: name.to_owned(),
        mode: Some((3840, 2160)),
        logical_size: Some((2560, 1440)),
        scale_factor: 2,
    }
}

fn desk() -> Probe {
    Probe {
        layer_shell: true,
        fractional_scale: true,
        outputs: vec![output("DP-1"), output("DP-4")],
    }
}

fn compositor(probe: Result<&Probe, &String>, config: Option<&Config>) -> Report {
    let mut report = Report::default();
    desktop::compositor(&mut report, probe, config);
    report
}

fn pipewire(state: CaptureState, rate: u32) -> Report {
    let mut report = Report::default();
    let status = StatusSnapshot {
        state,
        rate,
        channels: 2,
        peak: 0.0,
        frames: 0,
    };
    audio::pipewire(&mut report, Ok(status));
    report
}

#[test]
fn a_working_desktop_lists_protocols_and_outputs() {
    let report = compositor(Ok(&desk()), Some(&Config::default()));
    assert_eq!(
        report.lines(),
        [
            "wayland: connected",
            "layer shell: supported",
            "fractional scale: supported",
            "output DP-1: 3840x2160, scale 1.5, overlay",
            "output DP-4: 3840x2160, scale 1.5, no overlay",
        ]
    );
    assert_eq!(report.status(), 0);
}

#[test]
fn no_compositor_or_no_layer_shell_is_an_error() {
    let failure = "could not connect".to_owned();
    let report = compositor(Err(&failure), None);
    assert_eq!(report.lines(), ["error: wayland: could not connect"]);
    assert_eq!(report.status(), 1);

    let probe = Probe {
        layer_shell: false,
        fractional_scale: false,
        ..desk()
    };
    let report = compositor(Ok(&probe), None);
    assert_eq!(report.errors(), 1);
    assert!(report.lines()[1].starts_with("error: layer shell: "));
    assert!(report.lines()[2].contains("not supported"));
}

#[test]
fn outputs_that_are_not_connected_are_named() {
    let mut config = Config::default();
    config.overlay.show_on = ShowOn::Named(vec!["HDMI-9".to_owned(), "DP-4".to_owned()]);
    let report = compositor(Ok(&desk()), Some(&config));
    assert_eq!(report.status(), 0);
    assert_eq!(
        report.lines()[3..],
        [
            "output DP-1: 3840x2160, scale 1.5, no overlay",
            "output DP-4: 3840x2160, scale 1.5, overlay",
            "warning: overlay.show_on: no connected output named \"HDMI-9\"",
        ]
    );

    config.overlay.show_on = ShowOn::Sections;
    config.overlay.outputs = vec![OutputConfig {
        monitor: "HDMI-9".to_owned(),
        ..OutputConfig::default()
    }];
    let report = compositor(Ok(&desk()), Some(&config));
    assert_eq!(report.errors(), 1);
    assert_eq!(
        report.lines()[5..],
        [
            "warning: [output.HDMI-9]: no connected output has that name",
            "error: outputs: no connected output gets an overlay with this config",
        ]
    );

    let empty = Probe {
        outputs: Vec::new(),
        ..desk()
    };
    assert_eq!(compositor(Ok(&empty), None).errors(), 1);
}

#[test]
fn session_line_shows_the_display() {
    let mut report = Report::default();
    desktop::session(
        &mut report,
        &fake_env(&[
            ("XDG_SESSION_TYPE", "wayland"),
            ("WAYLAND_DISPLAY", "wayland-1"),
        ]),
    );
    desktop::session(&mut report, &fake_env(&[]));
    assert_eq!(
        report.lines(),
        [
            "session: wayland (WAYLAND_DISPLAY=wayland-1)",
            "session: unknown (WAYLAND_DISPLAY not set)",
        ]
    );
}

#[test]
fn pipewire_states() {
    let streaming = pipewire(CaptureState::Streaming, 48000);
    assert_eq!(
        streaming.lines(),
        ["pipewire: connected, capturing the default output (48000 Hz, 2 channels)"]
    );
    assert_eq!(
        pipewire(CaptureState::Idle, 0).lines(),
        ["pipewire: connected, the default output is idle"]
    );
    assert_eq!(pipewire(CaptureState::Connecting, 0).status(), 0);
    assert_eq!(pipewire(CaptureState::Unavailable, 0).status(), 1);
    assert_eq!(pipewire(CaptureState::Failed, 0).status(), 1);

    let mut report = Report::default();
    audio::pipewire(&mut report, Err("no thread".to_owned()));
    assert_eq!(report.lines(), ["error: pipewire: no thread"]);
}

#[test]
fn service_states() {
    assert_eq!(
        service::line(Some("enabled")),
        "service: kwybars.service is enabled"
    );
    assert_eq!(
        service::line(Some("disabled")),
        "service: kwybars.service is installed, not enabled"
    );
    for missing in ["", "not-found"] {
        assert_eq!(
            service::line(Some(missing)),
            "service: kwybars.service is not installed"
        );
    }
    assert!(service::line(None).contains("systemctl is not available"));
}

#[test]
fn settings_use_the_config_spellings() {
    let mut report = Report::default();
    settings(&mut report, &Config::default());
    assert_eq!(
        report.lines(),
        [
            "overlay: position bottom, layer background",
            "visualizer: layout line, 50 bars, 60 fps",
        ]
    );
}
