use std::sync::Arc;

use crate::config::{LoadedImage, load_background_image};
use crate::render::image::Encoded;
use crate::sweetbg::Output;

pub(super) struct Ready {
    pub epoch: u64,
    pub request: u64,
    pub images: Vec<(String, Arc<Encoded>)>,
    pub known: Vec<LoadedImage<Encoded>>,
    pub errors: Vec<String>,
}

pub(super) fn load(
    epoch: u64,
    request: u64,
    outputs: Vec<Output>,
    previous: Vec<LoadedImage<Encoded>>,
) -> Ready {
    let mut known: Vec<LoadedImage<Encoded>> = Vec::new();
    let mut images = Vec::new();
    let mut errors = Vec::new();
    for output in outputs {
        let loaded = match known.iter().find(|image| image.path == output.path) {
            Some(image) => image.clone(),
            None => {
                let image = load_background_image(output.path, &previous);
                known.push(image.clone());
                image
            }
        };
        match &loaded.source {
            Ok(source) => images.push((output.name, Arc::clone(source))),
            Err(error) => errors.push(format!(
                "sweetbg background on {} ({}): {error}",
                output.name,
                loaded.path.display()
            )),
        }
    }
    Ready {
        epoch,
        request,
        images,
        known,
        errors,
    }
}
