use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::config::{Config, ImageOverlayConfig};
use crate::render::image;

/// an image file and its decoded or compressed source, or why it cannot be shown
#[derive(Debug)]
pub struct LoadedImage<T = image::Source> {
    pub path: PathBuf,
    pub source: Result<Arc<T>, String>,
    /// the file as it was before decoding, none when it could not be read
    pub stamp: Option<Stamp>,
}

impl<T> Clone for LoadedImage<T> {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            source: self.source.clone(),
            stamp: self.stamp,
        }
    }
}

/// tells an image file apart from the one decoded last time
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    dev: u64,
    ino: u64,
    len: u64,
    mtime: (i64, i64),
    ctime: (i64, i64),
}

impl Stamp {
    fn of(path: &Path) -> Option<Self> {
        let meta = fs::metadata(path).ok()?;
        Some(Self {
            dev: meta.dev(),
            ino: meta.ino(),
            len: meta.size(),
            mtime: (meta.mtime(), meta.mtime_nsec()),
            ctime: (meta.ctime(), meta.ctime_nsec()),
        })
    }
}

/// the result for `file` from `known` while the file is unchanged, else a decode
fn decode_or_reuse<T>(
    file: PathBuf,
    known: &[LoadedImage<T>],
    decode: impl FnOnce(&Path) -> Result<T, image::ImageError>,
) -> LoadedImage<T> {
    // read before decoding, write during the decode is seen next time
    let stamp = Stamp::of(&file);
    let unchanged = known
        .iter()
        .find(|image| image.path == file && stamp.is_some() && image.stamp == stamp);
    let source = match unchanged {
        Some(image) => image.source.clone(),
        None => decode(&file).map(Arc::new).map_err(|err| err.to_string()),
    };
    LoadedImage {
        path: file,
        source,
        stamp,
    }
}

pub(super) struct Images {
    pub image: Option<LoadedImage>,
    pub outputs: Vec<Option<LoadedImage>>,
    pub background: Option<LoadedImage<image::Encoded>>,
    pub backgrounds: Vec<Option<LoadedImage<image::Encoded>>>,
}

pub(super) fn load(
    config: &Config,
    path: &Path,
    env: &dyn Fn(&str) -> Option<OsString>,
    known: &[LoadedImage],
    backgrounds: &[LoadedImage<image::Encoded>],
    problems: &mut Vec<String>,
) -> Images {
    let mut foreground = Loader {
        known,
        opened: Vec::new(),
        problems,
        decode: image::Source::open,
    };
    let mut open =
        |settings: &ImageOverlayConfig, label: &str| foreground.open(settings, label, path, env);
    let image = open(&config.image_overlay, "image overlay");
    let outputs = config
        .overlay
        .outputs
        .iter()
        .map(|output| open(&config.image(Some(output)), "image overlay"))
        .collect();
    let mut loader = Loader {
        known: backgrounds,
        opened: Vec::new(),
        problems,
        decode: image::Encoded::open,
    };
    let mut open =
        |settings: &ImageOverlayConfig, label: &str| loader.open(settings, label, path, env);
    let mut background = open(&config.background.image(), "background");
    let mut backgrounds: Vec<_> = config
        .overlay
        .outputs
        .iter()
        .map(|output| {
            open(
                &config.background(Some(output)).image(),
                &format!("background on {}", output.monitor),
            )
        })
        .collect();
    for image in background
        .iter_mut()
        .chain(backgrounds.iter_mut().flatten())
    {
        if image.source.as_ref().is_ok_and(|source| {
            let (width, height) = source.size();
            width > 8_388_607 || height > 8_388_607
        }) {
            let error =
                "image dimensions exceed Wayland's limit of 8388607 pixels per side".to_owned();
            problems.push(format!("background {}: {error}", image.path.display()));
            image.source = Err(error);
        }
    }
    Images {
        image,
        outputs,
        background,
        backgrounds,
    }
}

struct Loader<'a, T, F> {
    known: &'a [LoadedImage<T>],
    opened: Vec<LoadedImage<T>>,
    problems: &'a mut Vec<String>,
    decode: F,
}

impl<T, F: Fn(&Path) -> Result<T, image::ImageError>> Loader<'_, T, F> {
    fn open(
        &mut self,
        settings: &ImageOverlayConfig,
        label: &str,
        path: &Path,
        env: &dyn Fn(&str) -> Option<OsString>,
    ) -> Option<LoadedImage<T>> {
        let Some(file) = settings.file(path, env) else {
            if label.starts_with("background") && settings.enabled {
                self.problems
                    .push(format!("{label}: enabled but no path is set"));
            }
            return None;
        };
        if let Some(known) = self.opened.iter().find(|image| image.path == file) {
            return Some(known.clone());
        }
        let image = decode_or_reuse(file, self.known, &self.decode);
        if let Err(err) = &image.source {
            self.problems
                .push(format!("{label} {}: {err}", image.path.display()));
        }
        self.opened.push(image.clone());
        Some(image)
    }
}
