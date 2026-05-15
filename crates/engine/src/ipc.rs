use std::fmt::Write as _;
use std::io::{self, ErrorKind, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use kwybars_common::spectrum::SpectrumFrame;
use tracing::warn;

use crate::live::LiveFrameStream;

pub const FRAME_SOCKET_ENV: &str = "KWYBARS_FRAME_SOCKET";

pub struct FrameSocketServer {
    path: PathBuf,
    stop: Arc<AtomicBool>,
    framerate: Arc<AtomicU32>,
    _thread: JoinHandle<()>,
}

impl FrameSocketServer {
    pub fn spawn(
        stream: Arc<Mutex<LiveFrameStream>>,
        framerate: u32,
    ) -> io::Result<FrameSocketServer> {
        let path = socket_path();
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path)?;
        listener.set_nonblocking(true)?;

        let framerate = Arc::new(AtomicU32::new(framerate.max(1)));
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let thread_framerate = Arc::clone(&framerate);
        let thread_path = path.clone();
        let thread = thread::spawn(move || {
            publish_frames(listener, stream, thread_framerate, thread_stop);
            let _ = std::fs::remove_file(thread_path);
        });

        Ok(FrameSocketServer {
            path,
            stop,
            framerate,
            _thread: thread,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn set_framerate(&self, framerate: u32) {
        self.framerate.store(framerate.max(1), Ordering::Release);
    }
}

impl Drop for FrameSocketServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let _ = std::fs::remove_file(&self.path);
    }
}

fn publish_frames(
    listener: UnixListener,
    stream: Arc<Mutex<LiveFrameStream>>,
    framerate: Arc<AtomicU32>,
    stop: Arc<AtomicBool>,
) {
    let mut clients = Vec::<UnixStream>::new();
    while !stop.load(Ordering::Acquire) {
        accept_clients(&listener, &mut clients);
        let frame = stream
            .lock()
            .map(|stream| stream.latest_frame())
            .unwrap_or_else(|_| SpectrumFrame::new(Vec::new(), 0));
        let payload = serialize_frame(&frame);

        clients.retain_mut(|client| match client.write_all(payload.as_bytes()) {
            Ok(()) => true,
            Err(err) if err.kind() == ErrorKind::WouldBlock => false,
            Err(err) if err.kind() == ErrorKind::BrokenPipe => false,
            Err(err) => {
                warn!("kwybars: frame socket write failed: {err}");
                false
            }
        });

        let frame_delay = Duration::from_millis(
            (1000_u64 / u64::from(framerate.load(Ordering::Acquire).max(1))).max(1),
        );
        thread::sleep(frame_delay);
    }
}

fn accept_clients(listener: &UnixListener, clients: &mut Vec<UnixStream>) {
    loop {
        match listener.accept() {
            Ok((client, _)) => {
                if let Err(err) = client.set_write_timeout(Some(Duration::from_millis(50))) {
                    warn!("kwybars: could not set frame socket write timeout: {err}");
                }
                clients.push(client);
            }
            Err(err) if err.kind() == ErrorKind::WouldBlock => break,
            Err(err) => {
                warn!("kwybars: frame socket accept failed: {err}");
                break;
            }
        }
    }
}

pub(crate) fn serialize_frame(frame: &SpectrumFrame) -> String {
    let mut output = frame.timestamp_millis.to_string();
    for value in &frame.bars {
        let _ = write!(output, ";{value:.6}");
    }
    output.push('\n');
    output
}

pub(crate) fn parse_frame(line: &str) -> Option<SpectrumFrame> {
    let mut fields = line.trim().split(';');
    let timestamp_millis = fields.next()?.parse::<u64>().ok()?;
    let mut bars = Vec::new();
    for field in fields {
        let value = field.parse::<f32>().ok()?;
        bars.push(value);
    }

    if bars.is_empty() {
        return None;
    }

    Some(SpectrumFrame::new(bars, timestamp_millis))
}

fn socket_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "kwybars-frames-{}-{}.sock",
        std::process::id(),
        crate::live::now_millis()
    ))
}

#[cfg(test)]
mod tests {
    use kwybars_common::spectrum::SpectrumFrame;

    use super::{parse_frame, serialize_frame};

    #[test]
    fn round_trips_frame_line() {
        let frame = SpectrumFrame::new(vec![0.25, 0.5, 1.2], 42);
        let parsed = parse_frame(&serialize_frame(&frame));
        assert!(parsed.is_some());
        let parsed = match parsed {
            Some(parsed) => parsed,
            None => SpectrumFrame::new(Vec::new(), 0),
        };

        assert_eq!(parsed.timestamp_millis, 42);
        assert_eq!(parsed.bars, vec![0.25, 0.5, 1.0]);
        assert_eq!(parsed.peak, 1.0);
    }
}
