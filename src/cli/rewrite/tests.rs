use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};

use super::{RewriteError, atomic, match_image, switch};
use crate::app::RunOptions;
use crate::config::tests::TempDir;
use crate::xdg::fake_env;

fn options(path: &Path) -> RunOptions {
    RunOptions {
        config_path: Some(path.to_owned()),
    }
}

fn link(target: &Path, link: &Path) {
    if let Err(err) = symlink(target, link) {
        panic!("symlink {}: {err}", link.display());
    }
}

fn read(path: &Path) -> String {
    match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) => panic!("read {}: {err}", path.display()),
    }
}

/// the names in a directory, sorted
fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn switch_to(active: &Path, target: &Path) -> Result<String, RewriteError> {
    switch(&options(active), target, &fake_env(&[]))
}

fn match_in(config: &Path, overlays: &Path, wallpaper: &str) -> Result<String, RewriteError> {
    match_image(
        &options(config),
        overlays,
        Path::new(wallpaper),
        &fake_env(&[]),
    )
}

#[test]
fn switching_replaces_the_link_and_leaves_both_presets_alone() {
    let dir = TempDir::new("switch-link");
    let one = dir.write("custom/one.toml", "# one\n");
    let two = dir.write("custom/two.toml", "# two\n");
    let active = dir.path().join("current.toml");
    link(&one, &active);

    let message = switch_to(&active, &dir.path().join("custom/../custom/two.toml"));
    assert_eq!(
        message.ok(),
        Some(format!(
            "switched active config {} -> {}",
            active.display(),
            two.display()
        ))
    );
    assert_eq!(fs::read_link(&active).ok(), Some(two.clone()));
    assert_eq!(
        (read(&one), read(&two)),
        ("# one\n".into(), "# two\n".into())
    );
    assert_eq!(names(dir.path()), ["current.toml", "custom"]);

    let message = switch_to(&active, &two);
    assert_eq!(
        message.ok(),
        Some(format!("active config already points to {}", two.display()))
    );
}

#[test]
fn switching_creates_a_missing_config_path_and_its_directory() {
    let dir = TempDir::new("switch-new");
    let target = dir.write("presets/line.toml", "");
    let active = dir.path().join("kwybars/config.toml");
    assert!(switch_to(&active, &target).is_ok());
    assert_eq!(fs::read_link(&active).ok(), Some(target));
}

#[test]
fn switching_keeps_a_regular_config_as_a_backup() {
    let dir = TempDir::new("switch-backup");
    let target = dir.write("line.toml", "# line\n");
    let active = dir.write("config.toml", "# mine\n");
    let backup = dir.path().join("config.toml.bak");

    let message = switch_to(&active, &target).unwrap_or_default();
    assert!(
        message.ends_with(&format!("\nprevious config kept as {}", backup.display())),
        "{message}"
    );
    assert_eq!(read(&backup), "# mine\n");
    assert_eq!(fs::read_link(&active).ok(), Some(target.clone()));

    // a backup that holds something else is never overwritten
    let active = dir.write("other.toml", "# newer\n");
    dir.write("other.toml.bak", "# older\n");
    assert!(matches!(
        switch_to(&active, &target),
        Err(RewriteError::BackupTaken { .. })
    ));
    assert_eq!(read(&active), "# newer\n");
    assert!(!active.is_symlink());
    assert_eq!(read(&dir.path().join("other.toml.bak")), "# older\n");
}

#[test]
fn switching_to_the_file_that_is_the_config_changes_nothing() {
    let dir = TempDir::new("switch-self");
    let active = dir.write("config.toml", "# mine\n");
    let message = switch_to(&active, &active).unwrap_or_default();
    assert!(message.starts_with("active config already points to "));
    assert!(!active.is_symlink());
    assert_eq!(names(dir.path()), ["config.toml"]);
}

#[test]
fn switching_refuses_a_missing_target_or_a_directory() {
    let dir = TempDir::new("switch-target");
    let one = dir.write("one.toml", "");
    let active = dir.path().join("current.toml");
    link(&one, &active);

    assert!(matches!(
        switch_to(&active, &dir.path().join("nope.toml")),
        Err(RewriteError::TargetMissing(_))
    ));
    assert!(matches!(
        switch_to(&active, dir.path()),
        Err(RewriteError::TargetNotFile(_))
    ));
    assert_eq!(fs::read_link(&active).ok(), Some(one));
}

#[test]
fn a_failed_switch_leaves_no_temporary_link() {
    let dir = TempDir::new("switch-fail");
    let target = dir.write("line.toml", "");
    // a link cannot be renamed over a directory
    let active = dir.write("active/keep.toml", "");
    let active = active.parent().map(Path::to_owned).unwrap_or_default();

    let err = switch_to(&active, &target).err().map(|err| err.to_string());
    let expected = format!("could not replace {}: ", active.display());
    assert!(err.is_some_and(|err| err.starts_with(&expected)));
    assert_eq!(names(dir.path()), ["active", "line.toml"]);
    assert_eq!(names(&active), ["keep.toml"]);
}

const PRESET: &str = "\
# my preset
[visualizer]
bars = 48   # plenty

[image_overlay]
enabled = false # off for now
path    = 'old.png'   # the picture
opacity = 0.7

[overlay]
anchor = \"bottom\"
";

/// a config directory with two overlay images and a preset behind a link
fn setup(tag: &str) -> (TempDir, PathBuf, PathBuf, PathBuf) {
    let dir = TempDir::new(tag);
    dir.write("overlays/forest.png", "image");
    dir.write("overlays/forest.jpg", "image");
    dir.write("overlays/city.webp", "image");
    let preset = dir.write("custom/line.toml", PRESET);
    let active = dir.path().join("current.toml");
    link(&preset, &active);
    let overlays = dir.path().join("overlays");
    (dir, active, preset, overlays)
}

#[test]
fn matching_edits_the_linked_preset_and_keeps_its_formatting() {
    let (dir, active, preset, overlays) = setup("match-link");
    let image = overlays.join("forest.png");

    let message = match_in(&active, &overlays, "/walls/forest.jpeg");
    assert_eq!(
        message.ok(),
        Some(format!(
            "matched wallpaper /walls/forest.jpeg -> image overlay {}\nconfig: {} (updated)",
            image.display(),
            preset.display()
        ))
    );
    let expected = PRESET
        .replace("enabled = false # off", "enabled = true # off")
        .replace("'old.png'", &format!("\"{}\"", image.display()));
    assert_eq!(read(&preset), expected);
    assert_eq!(fs::read_link(&active).ok(), Some(preset.clone()));
    assert_eq!(names(dir.path()), ["current.toml", "custom", "overlays"]);
    assert_eq!(names(&dir.path().join("custom")), ["line.toml"]);

    let message = match_in(&active, &overlays, "forest.png").unwrap_or_default();
    assert!(message.ends_with("(unchanged)"), "{message}");
    assert_eq!(read(&preset), expected);
}

#[test]
fn matching_does_not_follow_a_link_out_of_the_config_directory() {
    let (dir, _, _, overlays) = setup("match-outside");
    let outside = dir.write("elsewhere/shared.toml", PRESET);
    let active = dir.path().join("custom/linked.toml");
    link(&outside, &active);

    let err = match_in(&active, &overlays, "city.jpg").err();
    assert!(
        matches!(&err, Some(RewriteError::LinkLeavesDir { target, .. }) if *target == outside),
        "{err:?}"
    );
    assert_eq!(read(&outside), PRESET);
    assert!(active.is_symlink());

    // naming the file itself is the way to change it
    assert!(match_in(&outside, &overlays, "city.jpg").is_ok());
    assert!(read(&outside).contains("city.webp"));
}

#[test]
fn matching_refuses_a_dangling_link() {
    let (dir, _, _, overlays) = setup("match-dangling");
    let active = dir.path().join("dangling.toml");
    link(&dir.path().join("gone.toml"), &active);
    assert!(matches!(
        match_in(&active, &overlays, "city.jpg"),
        Err(RewriteError::Io { .. })
    ));
    assert!(active.is_symlink());
    assert!(!dir.path().join("gone.toml").exists());
}

#[test]
fn matching_adds_the_section_to_a_config_without_one() {
    let (dir, _, _, overlays) = setup("match-new");
    let image = overlays.join("city.webp");
    let section = format!(
        "[image_overlay]\nenabled = true\npath = \"{}\"\n",
        image.display()
    );

    let missing = dir.path().join("fresh/config.toml");
    assert!(match_in(&missing, &overlays, "city.png").is_ok());
    assert_eq!(read(&missing), section);

    let plain = dir.write("plain.toml", "# bars\n[visualizer]\nbars = 12 # few\n");
    assert!(match_in(&plain, &overlays, "city.png").is_ok());
    assert_eq!(
        read(&plain),
        format!("# bars\n[visualizer]\nbars = 12 # few\n\n{section}")
    );

    let inline = dir.write("inline.toml", "image_overlay = { opacity = 0.5 }\n");
    assert!(match_in(&inline, &overlays, "city.png").is_ok());
    assert_eq!(
        read(&inline),
        format!(
            "image_overlay = {{ opacity = 0.5, enabled = true, path = \"{}\" }}\n",
            image.display()
        )
    );
}

#[test]
fn matching_keeps_the_permissions_of_the_config() {
    let (dir, _, _, overlays) = setup("match-mode");
    let config = dir.write("config.toml", PRESET);
    assert!(fs::set_permissions(&config, fs::Permissions::from_mode(0o600)).is_ok());
    assert!(match_in(&config, &overlays, "city.png").is_ok());
    let mode = fs::metadata(&config).map(|metadata| metadata.permissions().mode() & 0o777);
    assert_eq!(mode.ok(), Some(0o600));
}

#[test]
fn matching_leaves_a_config_it_cannot_use_untouched() {
    let (dir, _, _, overlays) = setup("match-bad");
    let broken = dir.write("broken.toml", "[image_overlay\npath = 1\n");
    let err = match_in(&broken, &overlays, "city.png").err();
    let text = err.as_ref().map(ToString::to_string).unwrap_or_default();
    assert!(matches!(err, Some(RewriteError::Parse { .. })), "{text}");
    assert!(
        text.starts_with(&format!("{}: ", broken.display())),
        "{text}"
    );
    assert_eq!(read(&broken), "[image_overlay\npath = 1\n");

    let scalar = dir.write("scalar.toml", "image_overlay = 3\n");
    assert!(matches!(
        match_in(&scalar, &overlays, "city.png"),
        Err(RewriteError::NotATable(_))
    ));
    assert_eq!(read(&scalar), "image_overlay = 3\n");
    assert_eq!(names(dir.path()).len(), 5, "{:?}", names(dir.path()));
}

#[test]
fn matching_reports_what_it_could_not_find() {
    let (dir, active, preset, overlays) = setup("match-none");
    type Case<'a> = (&'a Path, &'a str, fn(&RewriteError) -> bool);
    let cases: [Case; 4] = [
        (&overlays, "desert.jpg", |err| {
            matches!(err, RewriteError::NoMatch { .. })
        }),
        (&overlays, "..", |err| {
            matches!(err, RewriteError::NoStem(_))
        }),
        (&dir.path().join("nope"), "city.jpg", |err| {
            matches!(err, RewriteError::OverlayDirMissing(_))
        }),
        (&preset, "city.jpg", |err| {
            matches!(err, RewriteError::OverlayDirNotDir(_))
        }),
    ];
    for (overlay_dir, wallpaper, expected) in cases {
        let err = match_in(&active, overlay_dir, wallpaper).err();
        assert!(err.as_ref().is_some_and(expected), "{wallpaper}: {err:?}");
    }
    assert_eq!(read(&preset), PRESET);
}

#[test]
fn a_failed_write_keeps_the_file_and_leaves_no_temporary_one() {
    let dir = TempDir::new("write-fail");

    // a file cannot be renamed over a directory
    let taken = dir.write("taken/keep.toml", "");
    let taken = taken.parent().map(Path::to_owned).unwrap_or_default();
    let err = atomic::write(&taken, b"new")
        .err()
        .map(|err| err.to_string());
    let expected = format!("could not replace {}: ", taken.display());
    assert!(err.is_some_and(|err| err.starts_with(&expected)));
    assert_eq!(names(dir.path()), ["taken"]);

    let config = dir.write("locked/config.toml", "old");
    let locked = dir.path().join("locked");
    assert!(fs::set_permissions(&locked, fs::Permissions::from_mode(0o555)).is_ok());
    // root writes into a read-only directory anyway
    if fs::write(locked.join("probe"), "").is_err() {
        let err = atomic::write(&config, b"new")
            .err()
            .map(|err| err.to_string());
        assert!(err.is_some_and(|err| err.starts_with("could not write ")));
        assert_eq!(read(&config), "old");
        assert_eq!(names(&locked), ["config.toml"]);
    }
    assert!(fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).is_ok());
}

#[test]
fn a_leftover_temporary_file_is_replaced_not_written_through() {
    let dir = TempDir::new("write-leftover");
    let config = dir.write("config.toml", "old");
    let bystander = dir.write("bystander.toml", "mine");
    let temp = dir
        .path()
        .join(format!(".config.toml.kwybars-{}.tmp", std::process::id()));
    link(&bystander, &temp);

    assert!(atomic::write(&config, b"new").is_ok());
    assert_eq!(
        (read(&config), read(&bystander)),
        ("new".into(), "mine".into())
    );
    assert_eq!(names(dir.path()), ["bystander.toml", "config.toml"]);
}
