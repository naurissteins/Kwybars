//! a few shared-memory buffers per surface, drawn into only once the
//! compositor has released them

use smithay_client_toolkit::reexports::client::protocol::wl_shm;
use smithay_client_toolkit::shm::slot::{ActivateSlotError, Buffer, CreateBufferError, SlotPool};
use smithay_client_toolkit::shm::{CreatePoolError, Shm};

use crate::render::BufferContents;

/// buffers per surface; a third covers a compositor that holds two at once
const MAX_BUFFERS: usize = 3;

/// a buffer could not be drawn or shown
#[derive(Debug, thiserror::Error)]
pub enum DrawError {
    #[error("surface too large")]
    TooLarge,
    #[error("shared memory pool: {0}")]
    Pool(#[from] CreatePoolError),
    #[error("shared memory buffer: {0}")]
    Buffer(#[from] CreateBufferError),
    #[error("attaching the buffer: {0}")]
    Attach(#[from] ActivateSlotError),
}

/// one buffer and the bars it currently holds
pub struct Slot {
    pub buffer: Buffer,
    pub contents: BufferContents,
    drawn: u64,
}

/// the buffers of one surface, all of one size and format
pub struct BufferRing {
    format: wl_shm::Format,
    pool: Option<SlotPool>,
    slots: Vec<Slot>,
    size: (u32, u32),
    draws: u64,
}

impl BufferRing {
    pub fn new(format: wl_shm::Format) -> Self {
        Self {
            format,
            pool: None,
            slots: Vec::with_capacity(MAX_BUFFERS),
            size: (0, 0),
            draws: 0,
        }
    }

    pub fn format(&self) -> wl_shm::Format {
        self.format
    }

    pub fn release(&mut self) {
        self.slots.clear();
        self.pool = None;
        self.size = (0, 0);
    }

    pub fn acquire(
        &mut self,
        shm: &Shm,
        size: (u32, u32),
        blank_contents: impl FnOnce() -> BufferContents,
    ) -> Result<Option<(&mut Slot, &mut [u8])>, DrawError> {
        if size != self.size {
            // freed slots leave drawn memory behind, so the pool goes with them
            self.slots.clear();
            self.pool = None;
            self.size = size;
        }
        self.draws += 1;
        if self.pool.is_none() {
            self.pool = Some(SlotPool::new(byte_len(size)?, shm)?);
        }
        let Some(pool) = self.pool.as_mut() else {
            return Ok(None);
        };
        let free = self
            .slots
            .iter()
            .enumerate()
            .filter(|(_, slot)| slot.buffer.canvas(pool).is_some())
            .max_by_key(|(_, slot)| slot.drawn)
            .map(|(index, _)| index);
        let index = match free {
            Some(index) => index,
            None if self.slots.len() < MAX_BUFFERS => {
                let (width, height) = (int(size.0)?, int(size.1)?);
                let stride = width.checked_mul(4).ok_or(DrawError::TooLarge)?;
                let (buffer, _) = pool.create_buffer(width, height, stride, self.format)?;
                self.slots.push(Slot {
                    buffer,
                    contents: blank_contents(),
                    drawn: 0,
                });
                self.slots.len() - 1
            }
            None => return Ok(None),
        };
        let Some(slot) = self.slots.get_mut(index) else {
            return Ok(None);
        };
        slot.drawn = self.draws;
        Ok(slot.buffer.canvas(pool).map(|canvas| (slot, canvas)))
    }
}

fn int(value: u32) -> Result<i32, DrawError> {
    i32::try_from(value).map_err(|_| DrawError::TooLarge)
}

fn byte_len((width, height): (u32, u32)) -> Result<usize, DrawError> {
    (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(DrawError::TooLarge)
}
