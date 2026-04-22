use smithay_client_toolkit::reexports::client::protocol::{wl_shm, wl_surface};
use smithay_client_toolkit::shm::{
    Shm,
    slot::{Buffer, SlotPool},
};

use super::AppError;

const BUFFER_RING_SIZE: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BufferDimensions {
    width: u32,
    height: u32,
    stride: i32,
}

impl BufferDimensions {
    fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            stride: width as i32 * 4,
        }
    }

    fn byte_len(self) -> usize {
        self.height as usize * self.stride as usize
    }
}

pub struct SurfaceBuffers {
    pool: SlotPool,
    buffers: Vec<Buffer>,
    dimensions: Option<BufferDimensions>,
}

impl SurfaceBuffers {
    pub fn new(
        fallback_width: u32,
        fallback_height: u32,
        shm_state: &Shm,
    ) -> Result<Self, AppError> {
        let fallback_len =
            fallback_width as usize * fallback_height as usize * 4 * BUFFER_RING_SIZE;
        let pool = SlotPool::new(fallback_len.max(4), shm_state)
            .map_err(|err| AppError::BufferSetup(err.to_string()))?;

        Ok(Self {
            pool,
            buffers: Vec::new(),
            dimensions: None,
        })
    }

    pub fn render_and_attach(
        &mut self,
        width: u32,
        height: u32,
        surface: &wl_surface::WlSurface,
        render: impl FnOnce(&mut [u8]),
    ) -> Result<bool, AppError> {
        let dimensions = BufferDimensions::new(width, height);
        self.ensure_dimensions(dimensions)?;

        let Some(index) = self.next_available_buffer_index() else {
            return Ok(false);
        };

        {
            let canvas = self.buffers[index].canvas(&mut self.pool).ok_or_else(|| {
                AppError::BufferSetup("selected shm buffer is still active".into())
            })?;
            render(canvas);
        }

        self.buffers[index]
            .attach_to(surface)
            .map_err(|err| AppError::BufferSetup(err.to_string()))?;
        Ok(true)
    }

    fn ensure_dimensions(&mut self, dimensions: BufferDimensions) -> Result<(), AppError> {
        if self.dimensions == Some(dimensions) {
            return Ok(());
        }

        self.buffers.clear();
        let target_pool_len = self
            .pool
            .len()
            .max(dimensions.byte_len() * BUFFER_RING_SIZE);
        self.pool
            .resize(target_pool_len)
            .map_err(|err| AppError::BufferSetup(err.to_string()))?;

        for _ in 0..BUFFER_RING_SIZE {
            let (buffer, canvas) = self
                .pool
                .create_buffer(
                    dimensions.width as i32,
                    dimensions.height as i32,
                    dimensions.stride,
                    wl_shm::Format::Argb8888,
                )
                .map_err(|err| AppError::BufferSetup(err.to_string()))?;
            canvas.fill(0);
            self.buffers.push(buffer);
        }

        self.dimensions = Some(dimensions);
        Ok(())
    }

    fn next_available_buffer_index(&mut self) -> Option<usize> {
        self.buffers
            .iter()
            .position(|buffer| buffer.canvas(&mut self.pool).is_some())
    }
}
