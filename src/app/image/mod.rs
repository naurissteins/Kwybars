//! the image overlay's state and its workers' results

use std::sync::Arc;

use calloop::LoopHandle;
use calloop::channel;

use super::App;
use crate::config::{ImageOverlayConfig, Loaded, LoadedImage};
use crate::render::image::{Overlay, Overlays};

/// lets image workers reach the surfaces they scale for
pub(super) fn start(
    handle: &LoopHandle<'static, App>,
    app: &mut App,
) -> Result<(), calloop::Error> {
    let (jobs, results) = channel::channel();
    handle
        .insert_source(results, |event, (), app| {
            if let channel::Event::Msg(ready) = event {
                app.wayland.image_ready(ready);
            }
        })
        .map_err(|err| err.error)?;
    app.wayland.set_image_jobs(jobs);
    app.wayland.set_images(app.images.clone());
    Ok(())
}

pub(super) fn start_backgrounds(
    handle: &LoopHandle<'static, App>,
    app: &mut App,
) -> Result<(), calloop::Error> {
    let (jobs, results) = channel::channel();
    handle
        .insert_source(results, |event, (), app| {
            if let channel::Event::Msg(ready) = event {
                app.wayland.background_ready(ready);
                app.render();
            }
        })
        .map_err(|error| error.error)?;
    app.wayland.set_background_jobs(jobs);
    app.wayland.set_backgrounds(app.backgrounds.clone());
    Ok(())
}

pub(super) fn backgrounds(
    loaded: &Loaded,
    showing: &Overlays,
    previous: &crate::config::Config,
) -> Overlays {
    let make = |config, loaded: Option<&LoadedImage>, old: Option<&Overlay>| {
        let source = match &loaded?.source {
            Ok(source) => Arc::clone(source),
            Err(_) => Arc::clone(&old?.source),
        };
        Some(Overlay { config, source })
    };
    Overlays {
        base: make(
            loaded.config.background.image(),
            loaded.background.as_ref(),
            showing.base.as_ref(),
        ),
        outputs: loaded
            .config
            .overlay
            .outputs
            .iter()
            .enumerate()
            .map(|(index, output)| {
                let old = previous
                    .overlay
                    .outputs
                    .iter()
                    .position(|old| old.monitor == output.monitor);
                let old = old
                    .and_then(|index| showing.of(Some(index)))
                    .or(showing.base.as_ref());
                make(
                    loaded.config.background(Some(output)).image(),
                    loaded
                        .output_backgrounds
                        .get(index)
                        .and_then(Option::as_ref),
                    old,
                )
            })
            .collect(),
    }
}

pub(super) fn same_backgrounds(left: &Overlays, right: &Overlays) -> bool {
    let same = |left: Option<&Overlay>, right: Option<&Overlay>| match (left, right) {
        (Some(left), Some(right)) => {
            left.config == right.config && Arc::ptr_eq(&left.source, &right.source)
        }
        (None, None) => true,
        _ => false,
    };
    left.outputs.len() == right.outputs.len()
        && same(left.base.as_ref(), right.base.as_ref())
        && left
            .outputs
            .iter()
            .zip(&right.outputs)
            .all(|(left, right)| same(left.as_ref(), right.as_ref()))
}

/// the images to show after a load; a file that could not be read keeps
/// the pixels its surface is showing, with the new settings
pub(super) fn overlays(loaded: &Loaded, showing: &Overlays) -> Overlays {
    let config = &loaded.config;
    Overlays {
        base: overlay(
            config.image(None),
            loaded.image.as_ref(),
            showing.base.as_ref(),
            showing,
        ),
        outputs: config
            .overlay
            .outputs
            .iter()
            .enumerate()
            .map(|(entry, output)| {
                overlay(
                    config.image(Some(output)),
                    loaded.output_images.get(entry).and_then(Option::as_ref),
                    showing.of(Some(entry)),
                    showing,
                )
            })
            .collect(),
    }
}

fn overlay(
    config: ImageOverlayConfig,
    loaded: Option<&LoadedImage>,
    previous: Option<&Overlay>,
    showing: &Overlays,
) -> Option<Overlay> {
    let source = match &loaded?.source {
        Ok(source) => Arc::clone(showing.holding(source).unwrap_or(source)),
        Err(_) => Arc::clone(&previous?.source),
    };
    Some(Overlay { source, config })
}

#[cfg(test)]
mod tests;
