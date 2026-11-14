use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

use crate::cli::report::Report;

pub(super) fn check(report: &mut Report, env: &dyn Fn(&str) -> Option<OsString>) {
    let result = probe(env);
    match result {
        Ok(()) => report.line("sweetbg: native renderer subscription available".to_owned()),
        Err(error) => report.error(format!(
            "sweetbg: {error}; start sweetbgd with renderer subscription support"
        )),
    }
}

fn probe(env: &dyn Fn(&str) -> Option<OsString>) -> io::Result<()> {
    let runtime = env("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "XDG_RUNTIME_DIR is missing"))?;
    let mut socket = UnixStream::connect(runtime.join("sweetbg.sock"))?;
    socket.set_read_timeout(Some(Duration::from_millis(250)))?;
    socket.set_write_timeout(Some(Duration::from_millis(250)))?;
    socket.write_all(&[1, 10, 0, 0, 0, 0, 0, 0])?;
    let mut header = [0; 8];
    socket.read_exact(&mut header)?;
    let length = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);
    let valid_length = match header[1] {
        11 => (21..=4096).contains(&length),
        12 => length == 4,
        _ => false,
    };
    if header[0] != 1 || !valid_length || header[2..4] != [0, 0] {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "daemon does not provide renderer subscriptions",
        ));
    }
    Ok(())
}
