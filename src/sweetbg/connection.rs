use std::io::{self, Read, Write};
use std::os::fd::{AsFd, OwnedFd};
use std::os::unix::net::UnixStream;
use std::path::Path;

use rustix::net::{AddressFamily, SocketAddrUnix, SocketFlags, SocketType, connect, socket_with};

use super::Event;
use super::wire::{MAX_PAYLOAD, Snapshot, invalid, number};

pub(crate) struct Connection {
    stream: UnixStream,
    connecting: bool,
    received: Vec<u8>,
    sending: Vec<u8>,
    snapshot: Snapshot,
}

impl Connection {
    pub fn open(path: &Path) -> io::Result<Self> {
        let socket = socket_with(
            AddressFamily::UNIX,
            SocketType::STREAM,
            SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
            None,
        )?;
        let connecting = match connect(&socket, &SocketAddrUnix::new(path)?) {
            Ok(()) => false,
            Err(rustix::io::Errno::INPROGRESS) => true,
            Err(error) => return Err(error.into()),
        };
        let mut this = Self {
            stream: socket.into(),
            connecting,
            received: Vec::with_capacity(MAX_PAYLOAD + 8),
            sending: Vec::with_capacity(MAX_PAYLOAD),
            snapshot: Snapshot::default(),
        };
        this.queue(10, &[])?;
        Ok(this)
    }

    pub fn fd(&self) -> io::Result<OwnedFd> {
        self.stream.as_fd().try_clone_to_owned()
    }
    pub fn wants_write(&self) -> bool {
        self.connecting || !self.sending.is_empty()
    }

    pub fn claim(&mut self, revision: u32, name: &str) -> io::Result<()> {
        let mut payload = Vec::with_capacity(4 + name.len());
        payload.extend_from_slice(&revision.to_le_bytes());
        payload.extend_from_slice(name.as_bytes());
        self.queue(13, &payload)
    }

    pub fn release(&mut self, name: &str) -> io::Result<()> {
        self.queue(14, name.as_bytes())
    }

    fn queue(&mut self, kind: u8, payload: &[u8]) -> io::Result<()> {
        if payload.len() > MAX_PAYLOAD || self.sending.len() + 8 + payload.len() > MAX_PAYLOAD {
            return Err(invalid());
        }
        self.sending.extend_from_slice(&[1, kind, 0, 0]);
        self.sending
            .extend_from_slice(&(payload.len() as u32).to_le_bytes());
        self.sending.extend_from_slice(payload);
        Ok(())
    }

    pub fn flush(&mut self) -> io::Result<()> {
        if self.connecting {
            return Ok(());
        }
        while !self.sending.is_empty() {
            match self.stream.write(&self.sending) {
                Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
                Ok(written) => {
                    self.sending.drain(..written);
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    pub fn receive(&mut self) -> io::Result<Vec<Event>> {
        if let Some(error) = self.stream.take_error()? {
            return Err(error);
        }
        self.connecting = false;
        self.flush()?;
        let mut events = Vec::new();
        let mut handled = 0;
        while handled < 128 {
            let need = if self.received.len() < 8 {
                8
            } else {
                let header = self.received.get(..4).ok_or_else(invalid)?;
                if header[0] != 1 || header[2..] != [0, 0] {
                    return Err(invalid());
                }
                let len = number(&self.received, 4)? as usize;
                if len > MAX_PAYLOAD {
                    return Err(invalid());
                }
                8 + len
            };
            if self.received.len() == need {
                let kind = *self.received.get(1).ok_or_else(invalid)?;
                if let Some(event) = self
                    .snapshot
                    .accept(kind, self.received.get(8..).ok_or_else(invalid)?)?
                {
                    events.push(event);
                }
                self.received.clear();
                handled += 1;
                continue;
            }
            let mut bytes = [0; MAX_PAYLOAD + 8];
            match self.stream.read(&mut bytes[..need - self.received.len()]) {
                Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
                Ok(read) => self.received.extend_from_slice(&bytes[..read]),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
        }
        Ok(events)
    }
}
