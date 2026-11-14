use super::Connection;
use super::Event;
use super::wire::Snapshot;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

struct Peer {
    stream: UnixStream,
    path: PathBuf,
}

impl Drop for Peer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn connection() -> std::io::Result<(Connection, Peer)> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "kwybars-follow-{}-{}.sock",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let listener = UnixListener::bind(&path)?;
    let mut connection = Connection::open(&path)?;
    let (mut stream, _) = listener.accept()?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(1)))?;
    connection.receive()?;
    let mut subscription = [0; 8];
    stream.read_exact(&mut subscription)?;
    assert_eq!(subscription, [1, 10, 0, 0, 0, 0, 0, 0]);
    assert!(!connection.wants_write());
    Ok((connection, Peer { stream, path }))
}

fn framed(kind: u8, payload: &[u8]) -> Vec<u8> {
    let mut bytes = vec![1, kind, 0, 0];
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(payload);
    bytes
}

fn state(revision: u32, name: &str, path: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [revision, 7, 0, name.len() as u32, path.len() as u32] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(name.as_bytes());
    bytes.extend_from_slice(path);
    bytes
}

#[test]
fn stages_whole_snapshots_and_discards_superseded_revisions() -> std::io::Result<()> {
    let mut staged = Snapshot::default();
    assert!(staged.accept(11, &state(1, "DP-1", b"/old.png"))?.is_none());
    staged.accept(11, &state(2, "DP-2", b"/new.png"))?;
    let Some(Event::Snapshot { revision, outputs }) = staged.accept(12, &2_u32.to_le_bytes())?
    else {
        panic!("snapshot")
    };
    assert_eq!(revision, 2);
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].name, "DP-2");
    Ok(())
}

#[test]
fn rejects_duplicates_lengths_nul_and_unknown_messages() -> std::io::Result<()> {
    let bytes = state(1, "DP-1", b"/image.png");
    let mut snapshot = Snapshot::default();
    snapshot.accept(11, &bytes)?;
    assert!(snapshot.accept(11, &bytes).is_err());
    assert!(
        Snapshot::default()
            .accept(11, &bytes[..bytes.len() - 1])
            .is_err()
    );
    assert!(
        Snapshot::default()
            .accept(11, &state(1, "DP-1", b"/a\0b"))
            .is_err()
    );
    assert!(
        Snapshot::default()
            .accept(99, &1_u32.to_le_bytes())
            .is_err()
    );
    Ok(())
}

#[test]
fn nonblocking_stream_handles_fragmented_snapshots_and_claims() -> std::io::Result<()> {
    let (mut client, mut peer) = connection()?;
    let bytes = framed(11, &state(3, "DP-1", b"/image.png"));
    for fragment in bytes.chunks(3) {
        peer.stream.write_all(fragment)?;
        assert!(client.receive()?.is_empty());
    }
    peer.stream.write_all(&framed(12, &3_u32.to_le_bytes()))?;
    let events = client.receive()?;
    assert!(
        matches!(events.as_slice(), [Event::Snapshot { revision: 3, outputs }] if outputs.len() == 1)
    );
    client.claim(3, "DP-1")?;
    assert!(client.wants_write());
    client.flush()?;
    let mut claim = [0; 16];
    peer.stream.read_exact(&mut claim)?;
    assert_eq!(
        claim.to_vec(),
        framed(13, &[3, 0, 0, 0, b'D', b'P', b'-', b'1'])
    );
    peer.stream.write_all(&framed(
        15,
        &[3, 0, 0, 0, 1, 0, 0, 0, b'D', b'P', b'-', b'1'],
    ))?;
    assert!(
        matches!(client.receive()?.as_slice(), [Event::Ack { revision: 3, name, accepted: true }] if name == "DP-1")
    );
    client.release("DP-1")?;
    client.flush()?;
    let mut release = [0; 12];
    peer.stream.read_exact(&mut release)?;
    assert_eq!(release.to_vec(), framed(14, b"DP-1"));
    drop(peer);
    assert_eq!(
        client.receive().err().map(|e| e.kind()),
        Some(std::io::ErrorKind::UnexpectedEof)
    );
    Ok(())
}

#[test]
fn bounds_stream_frames_and_snapshot_output_count() -> std::io::Result<()> {
    let (mut client, mut peer) = connection()?;
    peer.stream.write_all(&[1, 11, 0, 0, 1, 16, 0, 0])?;
    assert_eq!(
        client.receive().err().map(|e| e.kind()),
        Some(std::io::ErrorKind::InvalidData)
    );
    let mut snapshot = Snapshot::default();
    for i in 0..64 {
        snapshot.accept(11, &state(1, &format!("DP-{i}"), b"/image.png"))?;
    }
    assert!(
        snapshot
            .accept(11, &state(1, "DP-65", b"/image.png"))
            .is_err()
    );
    let mut oversized_path = state(1, "DP-1", b"/image.png");
    oversized_path[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(Snapshot::default().accept(11, &oversized_path).is_err());
    Ok(())
}
