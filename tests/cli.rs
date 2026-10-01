//! exit codes and output of the `kwybars` binary's checking subcommands

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// a fresh directory under the system temp dir, removed on drop
struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!("kwybars-bin-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        if let Err(err) = fs::create_dir_all(&path) {
            panic!("create {}: {err}", path.display());
        }
        Self(path)
    }

    fn write(&self, name: &str, raw: &str) -> PathBuf {
        let path = self.0.join(name);
        if let Err(err) = fs::write(&path, raw) {
            panic!("write {}: {err}", path.display());
        }
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// runs the binary with a home of its own and returns status, stdout, stderr
fn kwybars(home: &Path, args: &[&str]) -> (Option<i32>, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_kwybars"))
        .args(args)
        .env_clear()
        .env("HOME", home)
        .output();
    let Output {
        status,
        stdout,
        stderr,
    } = match output {
        Ok(output) => output,
        Err(err) => panic!("could not run kwybars: {err}"),
    };
    let text = |bytes: Vec<u8>| String::from_utf8_lossy(&bytes).into_owned();
    (status.code(), text(stdout), text(stderr))
}

#[test]
fn help_lists_the_commands_and_exits_with_0() {
    let home = TempDir::new("help");
    for flag in ["--help", "-h"] {
        let (code, stdout, stderr) = kwybars(&home.0, &[flag]);
        assert_eq!(code, Some(0));
        assert!(stdout.starts_with("Usage: kwybars [OPTIONS] [COMMAND]"));
        for command in ["validate-config", "list-themes", "doctor"] {
            assert!(stdout.contains(command), "{command} missing from help");
        }
        assert!(stderr.is_empty());
    }
    let (code, stdout, _) = kwybars(&home.0, &["doctor", "--help"]);
    assert_eq!(code, Some(0));
    assert!(stdout.starts_with("Usage: kwybars"));
}

#[test]
fn usage_errors_exit_with_2_and_print_the_usage_to_stderr() {
    let home = TempDir::new("usage");
    for args in [
        &["frobnicate"][..],
        &["validate-config", "extra"],
        &["doctor", "--config"],
        &["list-themes", "--nope"],
    ] {
        let (code, stdout, stderr) = kwybars(&home.0, args);
        assert_eq!(code, Some(2), "{args:?}");
        assert!(stdout.is_empty(), "{args:?}");
        assert!(stderr.starts_with("kwybars: "), "{args:?}: {stderr}");
        assert!(stderr.contains("Usage: kwybars"), "{args:?}");
    }
}

#[test]
fn validate_config_exits_with_0_for_a_good_config_and_1_for_a_bad_one() {
    let home = TempDir::new("validate");
    let good = home.write("good.toml", "[visualizer]\nbars = 24\ntheme = \"nord\"\n");
    let good = good.to_string_lossy();
    let (code, stdout, stderr) = kwybars(&home.0, &["validate-config", "--config", &good]);
    assert_eq!(code, Some(0), "{stdout}{stderr}");
    assert!(stdout.starts_with(&format!("config: {good} (ok)\n")));
    assert!(stdout.ends_with("config validation passed\n"));

    let bad = home.write("bad.toml", "[visualizer]\nbars = \"many\"\n");
    let bad = bad.to_string_lossy();
    let (code, stdout, _) = kwybars(&home.0, &["validate-config", "-c", &bad]);
    assert_eq!(code, Some(1));
    assert!(stdout.starts_with(&format!("error: {bad}: ")), "{stdout}");
    assert!(stdout.contains("bars"), "{stdout}");
    assert!(stdout.ends_with("config validation failed: 1 error(s)\n"));

    let (code, stdout, _) = kwybars(&home.0, &["validate-config", "-c", "/nonexistent/k.toml"]);
    assert_eq!(code, Some(1));
    assert!(stdout.starts_with("error: config does not exist: /nonexistent/k.toml\n"));

    // no config at the default location is not an error
    let (code, stdout, _) = kwybars(&home.0, &["validate-config"]);
    assert_eq!(code, Some(0), "{stdout}");
}

#[test]
fn list_themes_exits_with_0() {
    let home = TempDir::new("themes");
    let (code, stdout, _) = kwybars(&home.0, &["list-themes"]);
    assert_eq!(code, Some(0));
    assert!(stdout.starts_with("available themes\n- ayu-dark (built-in)\n"));
}

#[test]
fn doctor_without_a_compositor_reports_it_and_exits_with_1() {
    let home = TempDir::new("doctor");
    let (code, stdout, _) = kwybars(&home.0, &["doctor"]);
    assert_eq!(code, Some(1), "{stdout}");
    assert!(stdout.starts_with("kwybars doctor ("));
    assert!(stdout.contains("\nerror: wayland: "), "{stdout}");
    assert!(stdout.contains("\nsession: unknown (WAYLAND_DISPLAY not set)\n"));
    assert!(stdout.contains(" issue(s) found\n"), "{stdout}");
}
