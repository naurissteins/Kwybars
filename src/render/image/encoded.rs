//! immutable compressed snapshots preserve last-good images without decoded pixels

use std::fs::File;
use std::io::{Cursor, Read};
use std::path::Path;

use memmap2::{Mmap, MmapMut};

use super::{ImageError, MAX_PIXELS, Source};

const MAX_BYTES: u64 = MAX_PIXELS * 8;

#[derive(Debug)]
pub struct Encoded {
    bytes: Mmap,
    size: (u32, u32),
}

impl Encoded {
    pub fn open(path: &Path) -> Result<Self, ImageError> {
        let mut file = File::open(path)?;
        let len = file.metadata()?.len();
        if len == 0 || len > MAX_BYTES {
            return Err(ImageError::EncodedSize);
        }
        let mut bytes =
            MmapMut::map_anon(usize::try_from(len).map_err(|_| ImageError::EncodedSize)?)?;
        file.read_exact(&mut bytes)?;
        if file.read(&mut [0])? != 0 {
            return Err(ImageError::EncodedSize);
        }
        let bytes = bytes.make_read_only()?;
        // Validate fully on the load worker; malformed replacements keep the old image.
        let source = Source::decode(Cursor::new(&bytes[..]))?;
        Ok(Self {
            bytes,
            size: source.size(),
        })
    }

    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    pub(crate) fn decode(&self) -> Result<Source, ImageError> {
        Source::decode(Cursor::new(&self.bytes[..]))
    }
}

impl PartialEq for Encoded {
    fn eq(&self, other: &Self) -> bool {
        self.size == other.size && self.bytes[..] == other.bytes[..]
    }
}
impl Eq for Encoded {}
