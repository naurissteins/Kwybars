use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use calloop::EventLoop;
use calloop::channel::{self, Channel, Event};

use super::{DEBOUNCE, Reloaded, Reloader};
use crate::config::{self, ConfigError, Loaded, ThemeOrigin};

const THEME: &str = include_str!("../../assets/themes/nord.toml");

static NEXT_DIR: AtomicU32 = AtomicU32::new(0);

/// a fresh directory under the system temp dir, removed on drop
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let name = format!(
            "kwybars-reload-{}-{}",
            std::process::id(),
            NEXT_DIR.fetch_add(1, Ordering::Relaxed)
        );
        let dir = std::env::temp_dir().join(name);
        let _ = fs::remove_dir_all(&dir);
        assert!(fs::create_dir_all(dir.join("presets")).is_ok());
        Self(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write(path: &Path, raw: &str) {
    assert!(fs::write(path, raw).is_ok(), "writing {}", path.display());
}

fn reloader(config: &Path) -> (Reloader, Channel<Reloaded>) {
    let loaded = config::load(config, &|_| None);
    let Ok(loaded) = loaded else {
        panic!("test config does not load");
    };
    let (sender, channel) = channel::channel();
    let Ok(reloader) = Reloader::new(config.to_owned(), &loaded, sender) else {
        panic!("no inotify");
    };
    (reloader, channel)
}

/// the loop a worker's result arrives on
struct Results(EventLoop<'static, Option<Reloaded>>);

impl Results {
    fn new(channel: Channel<Reloaded>) -> Self {
        let Ok(event_loop) = EventLoop::try_new() else {
            panic!("event loop");
        };
        let inserted = event_loop
            .handle()
            .insert_source(channel, |event, (), got| {
                if let Event::Msg(reloaded) = event {
                    *got = Some(reloaded);
                }
            });
        assert!(inserted.is_ok());
        Self(event_loop)
    }

    /// waits for the load that is running and hands its result to the reloader
    fn next(&mut self, reloader: &mut Reloader) -> Result<Loaded, ConfigError> {
        let mut got = None;
        let deadline = Instant::now() + Duration::from_secs(5);
        while got.is_none() && Instant::now() < deadline {
            let dispatched = self.0.dispatch(Some(Duration::from_millis(100)), &mut got);
            assert!(dispatched.is_ok());
        }
        let Some(reloaded) = got else {
            panic!("no result from the worker");
        };
        reloader.finish(reloaded)
    }

    /// loads now, as the debounce timer would
    fn load(&mut self, reloader: &mut Reloader) -> Result<Loaded, ConfigError> {
        let start = Instant::now();
        reloader.changed(start);
        assert_eq!(reloader.wake(start + DEBOUNCE), None);
        self.next(reloader)
    }
}

#[test]
fn edits_saves_and_link_switches_are_seen_but_other_files_are_not() {
    let dir = TempDir::new();
    let first = dir.0.join("presets/first.toml");
    let second = dir.0.join("presets/second.toml");
    write(&first, "[visualizer]\nbars = 10\n");
    write(&second, "[visualizer]\nbars = 20\n");
    let current = dir.0.join("current.toml");
    assert!(symlink(&first, &current).is_ok());
    let (mut reloader, _channel) = reloader(&current);
    assert!(!reloader.drain());

    write(&dir.0.join("presets/unrelated.toml"), "x = 1\n");
    write(&dir.0.join("notes.txt"), "x\n");
    assert!(!reloader.drain());

    // an edit of the link target
    write(&first, "[visualizer]\nbars = 11\n");
    assert!(reloader.drain());

    // an atomic save: write a temporary file, rename it over the target
    let temporary = dir.0.join("presets/.first.toml.swp");
    write(&temporary, "[visualizer]\nbars = 12\n");
    assert!(fs::rename(&temporary, &first).is_ok());
    assert!(reloader.drain());

    // colors.toml beside the link
    write(&dir.0.join("colors.toml"), "color_rgba = \"#ff0000\"\n");
    assert!(reloader.drain());

    // switching the link to another preset
    let link = dir.0.join("next.toml");
    assert!(symlink(&second, &link).is_ok());
    assert!(fs::rename(&link, &current).is_ok());
    assert!(reloader.drain());
}

#[test]
fn the_overlay_image_is_watched() {
    let dir = TempDir::new();
    let art = dir.0.join("presets/art.png");
    let save = |shade: u8| {
        let saved = image::RgbaImage::from_pixel(2, 2, image::Rgba([shade, 0, 0, 255])).save(&art);
        assert!(saved.is_ok(), "{saved:?}");
    };
    save(1);
    let path = dir.0.join("config.toml");
    write(
        &path,
        "[image_overlay]\nenabled = true\npath = \"presets/art.png\"\n",
    );
    let (mut reloader, _channel) = reloader(&path);
    assert!(!reloader.drain());
    save(2);
    assert!(reloader.drain());
    write(&dir.0.join("presets/other.png"), "x");
    assert!(!reloader.drain());
}

#[test]
fn changes_are_debounced() {
    let dir = TempDir::new();
    let path = dir.0.join("config.toml");
    write(&path, "");
    let (mut reloader, _channel) = reloader(&path);
    let start = Instant::now();
    assert_eq!(reloader.changed(start), Some(start + DEBOUNCE));
    // a later change moves the load back without another wakeup
    let later = start + Duration::from_millis(150);
    assert_eq!(reloader.changed(later), None);
    assert_eq!(reloader.wake(start + DEBOUNCE), Some(later + DEBOUNCE));
}

#[test]
fn a_load_runs_on_a_worker_and_watches_the_new_link_target() {
    let dir = TempDir::new();
    let first = dir.0.join("presets/first.toml");
    write(&first, "[visualizer]\nbars = 10\n");
    let other = dir.0.join("other");
    assert!(fs::create_dir_all(&other).is_ok());
    let second = other.join("second.toml");
    write(&second, "[visualizer]\nbars = 20\n");
    let current = dir.0.join("current.toml");
    assert!(symlink(&first, &current).is_ok());
    let (mut reloader, channel) = reloader(&current);

    let link = dir.0.join("next.toml");
    assert!(symlink(&second, &link).is_ok());
    assert!(fs::rename(&link, &current).is_ok());
    assert!(reloader.drain());
    let start = Instant::now();
    reloader.changed(start);
    assert_eq!(reloader.wake(start + DEBOUNCE), None);

    let mut results = Results::new(channel);
    let loaded = results.next(&mut reloader);
    assert!(matches!(&loaded, Ok(loaded) if loaded.config.visualizer.bars == 20));

    // the new target's directory is watched now
    write(&second, "[visualizer]\nbars = 21\n");
    assert!(reloader.drain());
}

#[test]
fn a_broken_config_is_an_error_to_keep_the_old_one() {
    let dir = TempDir::new();
    let path = dir.0.join("config.toml");
    write(&path, "");
    let (mut reloader, channel) = reloader(&path);
    write(&path, "[visualizer\nbars = ");
    reloader.changed(Instant::now());
    reloader.wake(Instant::now() + DEBOUNCE);
    assert!(Results::new(channel).next(&mut reloader).is_err());
}

#[test]
fn a_theme_created_after_the_load_is_picked_up_with_its_directory() {
    let dir = TempDir::new();
    let path = dir.0.join("config.toml");
    write(&path, "[visualizer]\ntheme = \"mine\"\n");
    let (mut reloader, channel) = reloader(&path);
    let mut results = Results::new(channel);
    assert!(!reloader.drain());

    // the directory appears first, and is watched from the next load on
    let themes = dir.0.join("themes");
    assert!(fs::create_dir(&themes).is_ok());
    assert!(reloader.drain());
    let loaded = results.load(&mut reloader);
    assert!(matches!(&loaded, Ok(loaded) if loaded.theme.is_none()));

    write(&themes.join("other.toml"), THEME);
    assert!(!reloader.drain());
    let mine = themes.join("mine.toml");
    write(&mine, THEME);
    assert!(reloader.drain());
    let loaded = results.load(&mut reloader);
    let origin = loaded
        .ok()
        .and_then(|loaded| loaded.theme)
        .map(|theme| theme.origin);
    assert_eq!(origin, Some(ThemeOrigin::File(mine.clone())));

    // and removing it again falls back
    assert!(fs::remove_file(&mine).is_ok());
    assert!(reloader.drain());
}

#[test]
fn a_file_that_shadows_a_built_in_theme_is_seen() {
    let dir = TempDir::new();
    assert!(fs::create_dir(dir.0.join("themes")).is_ok());
    let path = dir.0.join("config.toml");
    write(&path, "[visualizer]\ntheme = \"nord\"\n");
    let (mut reloader, _channel) = reloader(&path);
    assert!(!reloader.drain());
    write(&dir.0.join("themes/nord.toml"), THEME);
    assert!(reloader.drain());
}
