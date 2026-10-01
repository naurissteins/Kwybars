use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::{Command, USAGE, UsageError, parse, themes, validate};
use crate::app::RunOptions;
use crate::config::tests::TempDir;
use crate::xdg::fake_env;

fn parse_args(args: &[&str]) -> Result<Command, UsageError> {
    parse(args.iter().map(OsString::from))
}

fn run_with_config(path: &str) -> Command {
    Command::Run(RunOptions {
        config_path: Some(PathBuf::from(path)),
    })
}

#[test]
fn no_arguments_runs_with_default_config() {
    assert_eq!(
        parse_args(&[]).ok(),
        Some(Command::Run(RunOptions::default()))
    );
}

#[test]
fn config_flag_forms_are_equivalent() {
    let expected = Some(run_with_config("/tmp/custom.toml"));
    assert_eq!(parse_args(&["--config", "/tmp/custom.toml"]).ok(), expected);
    assert_eq!(parse_args(&["--config=/tmp/custom.toml"]).ok(), expected);
    assert_eq!(parse_args(&["-c", "/tmp/custom.toml"]).ok(), expected);
}

#[test]
fn help_and_version_flags() {
    assert_eq!(parse_args(&["--help"]).ok(), Some(Command::Help));
    assert_eq!(parse_args(&["-h"]).ok(), Some(Command::Help));
    assert_eq!(parse_args(&["--version"]).ok(), Some(Command::Version));
    assert_eq!(parse_args(&["-V"]).ok(), Some(Command::Version));
}

#[test]
fn help_wins_over_other_flags() {
    assert_eq!(
        parse_args(&["--version", "--config", "x.toml", "--help"]).ok(),
        Some(Command::Help)
    );
}

#[test]
fn missing_config_value_is_an_error() {
    assert!(matches!(
        parse_args(&["--config"]),
        Err(UsageError::Args(_))
    ));
}

#[test]
fn unknown_flag_is_an_error() {
    assert!(matches!(
        parse_args(&["--verbose"]),
        Err(UsageError::Args(_))
    ));
}

#[test]
fn debug_audio_command() {
    assert_eq!(
        parse_args(&["debug", "audio"]).ok(),
        Some(Command::DebugAudio)
    );
    for words in [&["debug"][..], &["debug", "video"][..]] {
        match parse_args(words) {
            Err(UsageError::UnknownCommand(name)) => assert_eq!(name, words.join(" ")),
            other => panic!("expected unknown command, got {other:?}"),
        }
    }
}

#[test]
fn unknown_command_is_named() {
    match parse_args(&["frobnicate"]) {
        Err(UsageError::UnknownCommand(name)) => assert_eq!(name, "frobnicate"),
        other => panic!("expected unknown command, got {other:?}"),
    }
}

#[test]
fn checking_commands_take_the_config_flag() {
    let options = RunOptions {
        config_path: Some(PathBuf::from("/tmp/custom.toml")),
    };
    let commands = [
        ("validate-config", Command::ValidateConfig(options.clone())),
        ("list-themes", Command::ListThemes(options.clone())),
        ("doctor", Command::Doctor(options)),
    ];
    for (word, expected) in commands {
        assert_eq!(
            parse_args(&[word, "--config", "/tmp/custom.toml"]).ok(),
            Some(expected.clone())
        );
        assert_eq!(
            parse_args(&["-c", "/tmp/custom.toml", word]).ok(),
            Some(expected)
        );
        assert!(matches!(
            parse_args(&[word, "extra"]),
            Err(UsageError::UnknownCommand(_))
        ));
    }
    assert_eq!(
        parse_args(&["doctor"]).ok(),
        Some(Command::Doctor(RunOptions::default()))
    );
}

#[test]
fn usage_names_every_command() {
    for command in [
        "validate-config",
        "list-themes",
        "doctor",
        "debug audio",
        "debug spectrum",
    ] {
        assert!(USAGE.contains(&format!("\n  {command} ")), "{command}");
    }
}

fn options(path: &Path) -> RunOptions {
    RunOptions {
        config_path: Some(path.to_owned()),
    }
}

#[test]
fn a_good_config_passes_validation() {
    let dir = TempDir::new("cli-valid");
    let path = dir.write("config.toml", "[visualizer]\ntheme = \"nord\"\n");
    let report = validate::report(&options(&path), &fake_env(&[]));
    assert_eq!(
        report.lines(),
        [
            format!("config: {} (ok)", path.display()),
            format!(
                "colors: {} (not found)",
                dir.path().join("colors.toml").display()
            ),
            "theme: nord (built-in)".to_owned(),
            "image overlay: disabled".to_owned(),
            "config validation passed".to_owned(),
        ]
    );
    assert_eq!(report.status(), 0);
}

#[test]
fn warnings_pass_validation_and_errors_fail_it() {
    let dir = TempDir::new("cli-invalid");
    let env = fake_env(&[]);

    let odd = dir.write(
        "odd.toml",
        "[visualizer]\nbackend = \"cava\"\n[image_overlay]\nopacity = 5.0\n",
    );
    let report = validate::report(&options(&odd), &env);
    let warnings = report
        .lines()
        .iter()
        .filter(|line| line.starts_with("warning: "))
        .count();
    assert_eq!((warnings, report.status()), (2, 0), "{:?}", report.lines());

    let broken = dir.write("broken.toml", "[visualizer\nbars = 3\n");
    let report = validate::report(&options(&broken), &env);
    assert_eq!(report.status(), 1);
    assert!(report.lines()[0].starts_with(&format!("error: {}: ", broken.display())));
    assert_eq!(
        report.lines().last().map(String::as_str),
        Some("config validation failed: 1 error(s)")
    );

    let missing = dir.path().join("missing.toml");
    let report = validate::report(&options(&missing), &env);
    assert_eq!(
        report.lines()[0],
        format!("error: config does not exist: {}", missing.display())
    );
    assert_eq!(report.status(), 1);
}

#[test]
fn unusable_colors_theme_and_image_fail_validation() {
    let dir = TempDir::new("cli-files");
    dir.write("colors.toml", "color_rgba = \"rgba(1, 2)\"\n");
    dir.write("art.png", "not an image");
    let path = dir.write(
        "config.toml",
        "[visualizer]\ntheme = \"nope\"\n[image_overlay]\nenabled = true\npath = \"art.png\"\n",
    );
    let report = validate::report(&options(&path), &fake_env(&[]));
    assert_eq!(report.errors(), 3, "{:?}", report.lines());
    assert!(
        !report
            .lines()
            .iter()
            .any(|line| line.starts_with("colors:"))
    );

    let empty = dir.write("empty.toml", "[image_overlay]\nenabled = true\n");
    let report = validate::report(&options(&empty), &fake_env(&[]));
    assert!(
        report.lines().contains(
            &"error: image_overlay is enabled but image_overlay.path is empty".to_owned()
        ),
        "{:?}",
        report.lines()
    );
}

#[test]
fn a_missing_default_config_is_fine_and_no_location_is_not() {
    let dir = TempDir::new("cli-default");
    let home = dir.path().to_string_lossy().into_owned();
    let report = validate::report(&RunOptions::default(), &fake_env(&[("HOME", &home)]));
    assert_eq!(report.status(), 0);
    assert!(report.lines()[0].ends_with("(not found, built-in defaults are used)"));

    let named = fake_env(&[("HOME", &home), ("KWYBARS_CONFIG", "/nonexistent/k.toml")]);
    assert_eq!(validate::report(&RunOptions::default(), &named).status(), 1);
    assert_eq!(
        validate::report(&RunOptions::default(), &fake_env(&[])).status(),
        1
    );
}

#[test]
fn a_switched_config_shows_where_the_link_points() {
    let dir = TempDir::new("cli-link");
    let target = dir.write("custom/001.toml", "");
    let link = dir.path().join("current.toml");
    if let Err(err) = std::os::unix::fs::symlink(&target, &link) {
        panic!("symlink: {err}");
    }
    let report = validate::report(&options(&link), &fake_env(&[]));
    assert_eq!(
        report.lines()[..2],
        [
            format!("config: {} (ok)", link.display()),
            format!("resolved config path: {}", target.display()),
        ]
    );
    let report = validate::report(&options(&target), &fake_env(&[]));
    assert!(!report.lines()[1].starts_with("resolved"));
}

#[test]
fn themes_list_files_before_built_ins_of_the_same_name() {
    let dir = TempDir::new("cli-themes");
    let path = dir.write("config.toml", "");
    let nord = dir.write("themes/nord.toml", "");
    let mine = dir.write("themes/mine.toml", "");
    dir.write("themes/.hidden.toml", "");
    dir.write("themes/notes.txt", "");
    let report = themes::report(&options(&path), &fake_env(&[]));
    assert_eq!(report.status(), 0);
    assert_eq!(
        report.lines(),
        [
            "available themes".to_owned(),
            "- ayu-dark (built-in)".to_owned(),
            "- catppuccin-mocha (built-in)".to_owned(),
            "- dracula (built-in)".to_owned(),
            "- everforest (built-in)".to_owned(),
            "- gruvbox (built-in)".to_owned(),
            format!("- mine ({})", mine.display()),
            format!("- nord ({})", nord.display()),
            "- rose-pine (built-in)".to_owned(),
            "- tokyo-night (built-in)".to_owned(),
        ]
    );
}
