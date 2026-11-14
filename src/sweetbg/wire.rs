use std::ffi::OsStr;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;

use super::{Event, Output};

pub(super) const MAX_PAYLOAD: usize = 4096;
const MAX_OUTPUTS: usize = 64;

pub(super) fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "invalid sweetbg subscription message",
    )
}

pub(super) fn number(bytes: &[u8], at: usize) -> io::Result<u32> {
    let raw = bytes.get(at..at + 4).ok_or_else(invalid)?;
    Ok(u32::from_le_bytes(raw.try_into().map_err(|_| invalid())?))
}

#[derive(Default)]
pub(super) struct Snapshot {
    revision: u32,
    outputs: Vec<Output>,
}

impl Snapshot {
    pub fn accept(&mut self, kind: u8, payload: &[u8]) -> io::Result<Option<Event>> {
        let revision = number(payload, 0)?;
        match kind {
            11 => {
                if self.revision != revision {
                    self.outputs.clear();
                    self.revision = revision;
                }
                let name_len = number(payload, 12)? as usize;
                let path_len = number(payload, 16)? as usize;
                if name_len == 0
                    || name_len >= 64
                    || path_len > MAX_PAYLOAD
                    || payload.len() != 20 + name_len + path_len
                    || self.outputs.len() >= MAX_OUTPUTS
                {
                    return Err(invalid());
                }
                let name = name(&payload[20..20 + name_len])?;
                if self.outputs.iter().any(|output| output.name == name) {
                    return Err(invalid());
                }
                let path = &payload[20 + name_len..];
                if path.contains(&0) {
                    return Err(invalid());
                }
                self.outputs.push(Output {
                    name,
                    path: PathBuf::from(OsStr::from_bytes(path)),
                    fit: number(payload, 8)?,
                    generation: number(payload, 4)?,
                });
                Ok(None)
            }
            12 if payload.len() == 4 => {
                let outputs = if self.revision == revision {
                    std::mem::take(&mut self.outputs)
                } else {
                    Vec::new()
                };
                self.revision = 0;
                Ok(Some(Event::Snapshot { revision, outputs }))
            }
            15 if payload.len() > 8 => Ok(Some(Event::Ack {
                revision,
                accepted: number(payload, 4)? == 1,
                name: name(&payload[8..])?,
            })),
            _ => Err(invalid()),
        }
    }
}

fn name(bytes: &[u8]) -> io::Result<String> {
    if bytes.is_empty() || bytes.len() >= 64 || bytes.contains(&0) {
        return Err(invalid());
    }
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|_| invalid())
}
