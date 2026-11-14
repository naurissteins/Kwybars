use std::fs::File;
use std::os::fd::AsFd;
use std::sync::Arc;

use calloop::channel::Sender;
use smithay_client_toolkit::reexports::client::protocol::{wl_buffer::WlBuffer, wl_shm};
use smithay_client_toolkit::reexports::client::{Proxy, QueueHandle, backend::ObjectId};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shm::{Shm, raw::RawPool};

use super::{Surface, Wayland};
use crate::render::ByteOrder;
use crate::render::image::Source;
use crate::wayland::handlers::NoEvents;
use crate::wayland::surface::buffers::DrawError;

pub struct Ready {
    surface: ObjectId,
    source: Arc<Source>,
    result: std::io::Result<()>,
}

pub(super) struct Upload {
    pub buffer: WlBuffer,
    _pool: RawPool,
    pub source: Arc<Source>,
}

impl Drop for Upload {
    fn drop(&mut self) {
        self.buffer.destroy();
    }
}

impl Surface {
    pub(super) fn prepare(
        &mut self,
        jobs: Option<&Sender<Ready>>,
        shm: &Shm,
        format: wl_shm::Format,
        queue: &QueueHandle<Wayland>,
    ) -> Result<(), DrawError> {
        if self.uploading.is_some()
            || self
                .upload
                .as_ref()
                .is_some_and(|upload| Arc::ptr_eq(&upload.source, &self.source))
        {
            return Ok(());
        }
        let Some(jobs) = jobs else {
            return Ok(());
        };
        let (upload, mut file) = allocate(shm, format, Arc::clone(&self.source), queue)?;
        let (jobs, source, surface) = (
            jobs.clone(),
            Arc::clone(&self.source),
            self.layer.wl_surface().id(),
        );
        std::thread::Builder::new()
            .name("kwybars-background".into())
            .spawn(move || {
                let order = if format == wl_shm::Format::Abgr8888 {
                    ByteOrder::Rgba
                } else {
                    ByteOrder::Bgra
                };
                // Only this worker writes the backing file, before the buffer is attached.
                // The main thread owns the Wayland objects and never touches its mapping.
                let result = source.write_native(&mut file, order);
                let _ = jobs.send(Ready {
                    surface,
                    source,
                    result,
                });
            })?;
        self.uploading = Some(upload);
        Ok(())
    }
}

impl Wayland {
    pub fn background_ready(&mut self, ready: Ready) {
        let Some(surface) = self
            .background
            .surfaces
            .iter_mut()
            .find(|item| item.layer.wl_surface().id() == ready.surface)
        else {
            return;
        };
        let Some(upload) = surface.uploading.take() else {
            return;
        };
        if !Arc::ptr_eq(&surface.source, &ready.source) {
            return;
        }
        match ready.result {
            Ok(()) => {
                surface.upload = Some(upload);
                surface.attached = false;
                surface.previous = None;
                surface.failed = false;
            }
            Err(error) => {
                if !surface.failed {
                    tracing::warn!("could not prepare background on {}: {error}", surface.label);
                    surface.failed = true;
                }
                surface.retry_at =
                    Some(std::time::Instant::now() + std::time::Duration::from_secs(1));
            }
        }
    }
}

fn allocate(
    shm: &Shm,
    format: wl_shm::Format,
    source: Arc<Source>,
    queue: &QueueHandle<Wayland>,
) -> Result<(Upload, File), DrawError> {
    let size = source.size();
    // viewporter source coordinates are signed 24.8 fixed-point
    if size.0 > 8_388_607 || size.1 > 8_388_607 {
        return Err(DrawError::TooLarge);
    }
    let int = |value| i32::try_from(value).map_err(|_| DrawError::TooLarge);
    let stride = size.0.checked_mul(4).ok_or(DrawError::TooLarge)?;
    let len = u64::from(stride) * u64::from(size.1);
    let len = usize::try_from(len).map_err(|_| DrawError::TooLarge)?;
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
    Ok((
        Upload {
            buffer,
            _pool: pool,
            source,
        },
        file,
    ))
}
