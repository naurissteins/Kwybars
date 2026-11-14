use std::fs::File;
use std::os::fd::AsFd;
use std::rc::Rc;
use std::sync::Arc;

use calloop::channel::Sender;
use smithay_client_toolkit::reexports::client::QueueHandle;
use smithay_client_toolkit::reexports::client::protocol::{wl_buffer::WlBuffer, wl_shm};
use smithay_client_toolkit::shm::{Shm, raw::RawPool};

use super::{Surface, Wayland};
use crate::render::ByteOrder;
use crate::render::background::{self, Preparation};
use crate::render::image::{Encoded, ImageError};
use crate::wayland::handlers::NoEvents;
use crate::wayland::surface::buffers::DrawError;

pub struct Ready {
    source: Arc<Encoded>,
    preparation: Preparation,
    result: Result<(), ImageError>,
}

pub(super) struct Upload {
    pub buffer: WlBuffer,
    _pool: RawPool,
    pub source: Arc<Encoded>,
    pub preparation: Preparation,
}

impl Upload {
    fn matches(&self, source: &Arc<Encoded>, preparation: Preparation) -> bool {
        Arc::ptr_eq(&self.source, source) && self.preparation == preparation
    }
}

impl Drop for Upload {
    fn drop(&mut self) {
        self.buffer.destroy();
    }
}

#[derive(Default)]
pub(super) struct Uploads {
    ready: Vec<Rc<Upload>>,
    pending: Option<Upload>,
}

impl Uploads {
    pub fn find(&self, source: &Arc<Encoded>, preparation: Preparation) -> Option<Rc<Upload>> {
        self.ready
            .iter()
            .find(|upload| upload.matches(source, preparation))
            .cloned()
    }

    pub fn clear(&mut self) {
        self.ready.clear();
        self.pending = None;
    }

    pub fn prune(&mut self) {
        self.ready.retain(|upload| Rc::strong_count(upload) > 1);
    }

    pub fn prepare(
        &mut self,
        source: &Arc<Encoded>,
        preparation: Preparation,
        jobs: Option<&Sender<Ready>>,
        shm: &Shm,
        format: wl_shm::Format,
        queue: &QueueHandle<Wayland>,
    ) -> Result<(), DrawError> {
        // One short-lived preparation worker at a time, even across rapid reloads.
        if self.pending.is_some() {
            return Ok(());
        }
        let Some(jobs) = jobs else {
            return Ok(());
        };
        let (upload, mut file) = allocate(shm, format, Arc::clone(source), preparation, queue)?;
        let (jobs, source) = (jobs.clone(), Arc::clone(source));
        std::thread::Builder::new()
            .name("kwybars-background".into())
            .spawn(move || {
                let order = if format == wl_shm::Format::Abgr8888 {
                    ByteOrder::Rgba
                } else {
                    ByteOrder::Bgra
                };
                // The worker only writes the file, before attachment; Wayland stays on main.
                let result = source.decode().and_then(|decoded| {
                    background::write(&decoded, preparation, &mut file, order)
                        .map_err(ImageError::from)
                });
                let _ = jobs.send(Ready {
                    source,
                    preparation,
                    result,
                });
            })?;
        self.pending = Some(upload);
        Ok(())
    }
}

impl Surface {
    pub(super) fn update_upload(
        &mut self,
        uploads: &mut Uploads,
        jobs: Option<&Sender<Ready>>,
        shm: &Shm,
        format: wl_shm::Format,
        queue: &QueueHandle<Wayland>,
    ) -> Result<(), DrawError> {
        if let Some(screen) = self.screen {
            let preparation = Preparation::new(
                self.source.size(),
                self.scale.buffer_size(screen),
                self.config.zoom,
                self.config.shake,
            );
            if let Some(preparation) = preparation {
                if let Some(upload) = uploads.find(&self.source, preparation) {
                    if self
                        .upload
                        .as_ref()
                        .is_none_or(|old| !Rc::ptr_eq(old, &upload))
                    {
                        self.upload = Some(upload);
                        self.attached = false;
                        self.previous = None;
                        self.failed = false;
                    }
                    Ok(())
                } else {
                    uploads.prepare(&self.source, preparation, jobs, shm, format, queue)
                }
            } else {
                Err(crate::wayland::surface::buffers::DrawError::TooLarge)
            }
        } else {
            Ok(())
        }
    }
}

impl Wayland {
    pub fn background_ready(&mut self, ready: Ready) {
        let Some(upload) = self.background.uploads.pending.take() else {
            return;
        };
        if !upload.matches(&ready.source, ready.preparation) {
            return;
        }
        match ready.result {
            Ok(()) => self.background.uploads.ready.push(Rc::new(upload)),
            Err(error) => {
                for surface in &mut self.background.surfaces {
                    if Arc::ptr_eq(&surface.source, &ready.source) {
                        if !surface.failed {
                            tracing::warn!(
                                "could not prepare background on {}: {error}",
                                surface.label
                            );
                            surface.failed = true;
                        }
                        surface.retry_at =
                            Some(std::time::Instant::now() + std::time::Duration::from_secs(1));
                    }
                }
            }
        }
    }
}

fn allocate(
    shm: &Shm,
    format: wl_shm::Format,
    source: Arc<Encoded>,
    preparation: Preparation,
    queue: &QueueHandle<Wayland>,
) -> Result<(Upload, File), DrawError> {
    let size = preparation.size;
    let int = |value| i32::try_from(value).map_err(|_| DrawError::TooLarge);
    let stride = size.0.checked_mul(4).ok_or(DrawError::TooLarge)?;
    let len =
        usize::try_from(u64::from(stride) * u64::from(size.1)).map_err(|_| DrawError::TooLarge)?;
    let mut pool = RawPool::new(len, shm)?;
    let file = File::from(pool.as_fd().try_clone_to_owned()?);
    let buffer = pool.create_buffer(
        0,
        int(size.0)?,
        int(size.1)?,
        int(stride)?,
        format,
        NoEvents,
        queue,
    );
    tracing::debug!(
        "preparing background buffer {}x{} from {}x{}",
        size.0,
        size.1,
        source.size().0,
        source.size().1
    );
    Ok((
        Upload {
            buffer,
            _pool: pool,
            source,
            preparation,
        },
        file,
    ))
}
