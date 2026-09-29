//! one pipewire connection: context, core, and the monitor capture stream

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use pipewire::context::ContextRc;
use pipewire::core::{CoreRc, Listener as CoreListener, PW_ID_CORE};
use pipewire::keys;
use pipewire::main_loop::MainLoopRc;
use pipewire::properties::properties;
use pipewire::spa::param::ParamType;
use pipewire::spa::pod::Pod;
use pipewire::spa::utils::Direction;
use pipewire::stream::{Stream, StreamFlags, StreamListener, StreamRc, StreamState};
use tracing::{debug, warn};

use super::Shared;
use super::analysis::StreamData;
use super::format;
use super::status::CaptureState;
use super::tuning::SharedTuning;

/// a pipewire session that could not be set up
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error(transparent)]
    Pipewire(#[from] pipewire::Error),
    #[error("{0}")]
    Format(String),
}

/// a live connection; dropping it disconnects
pub struct Session {
    // listeners go first so they are dropped before what they observe
    _stream_listener: StreamListener<StreamData>,
    _core_listener: CoreListener,
    _stream: StreamRc,
    _core: CoreRc,
    _context: ContextRc,
    lost: Rc<Cell<bool>>,
}

impl Session {
    /// connects to pipewire and starts capturing the default sink monitor
    pub fn start(
        mainloop: &MainLoopRc,
        shared: &Arc<Shared>,
        tuning: &SharedTuning,
    ) -> Result<Self, SessionError> {
        let context = ContextRc::new(mainloop, None)?;
        let core = context.connect_rc(None)?;
        let lost = Rc::new(Cell::new(false));

        let core_listener = core
            .add_listener_local()
            .error({
                let (mainloop, lost) = (mainloop.clone(), Rc::clone(&lost));
                move |id, _seq, res, message| {
                    if id == PW_ID_CORE {
                        debug!("pipewire core error {res}: {message}");
                        lost.set(true);
                        mainloop.quit();
                    }
                }
            })
            .register();

        let props = properties! {
            *keys::MEDIA_TYPE => "Audio",
            *keys::MEDIA_CATEGORY => "Capture",
            *keys::MEDIA_ROLE => "Music",
            *keys::STREAM_CAPTURE_SINK => "true",
            // do not keep the sink running when nothing plays
            *keys::NODE_PASSIVE => "true",
            *keys::NODE_NAME => "kwybars",
            *keys::APP_NAME => "Kwybars",
        };
        let stream = StreamRc::new(core.clone(), "kwybars", props)?;
        let data = StreamData::new(Arc::clone(shared), Rc::clone(tuning));
        let stream_listener = listen(&stream, data, mainloop, &lost)?;

        let format = format::enum_format().map_err(SessionError::Format)?;
        let pod = Pod::from_bytes(&format)
            .ok_or_else(|| SessionError::Format("invalid stream format pod".to_owned()))?;
        stream.connect(
            Direction::Input,
            None,
            StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS,
            &mut [pod],
        )?;

        Ok(Self {
            _stream_listener: stream_listener,
            _core_listener: core_listener,
            _stream: stream,
            _core: core,
            _context: context,
            lost,
        })
    }

    /// whether the connection ended because of an error
    pub fn lost(&self) -> bool {
        self.lost.get()
    }
}

/// registers the callbacks that feed `data` from the stream
fn listen(
    stream: &StreamRc,
    data: StreamData,
    mainloop: &MainLoopRc,
    lost: &Rc<Cell<bool>>,
) -> Result<StreamListener<StreamData>, pipewire::Error> {
    stream
        .add_local_listener_with_user_data(data)
        .param_changed(|_, data, id, param| {
            if let Some(param) = param
                && id == ParamType::Format.as_raw()
            {
                data.format_changed(param);
            }
        })
        .state_changed({
            let (mainloop, lost) = (mainloop.clone(), Rc::clone(lost));
            move |_, data, _old, new| {
                if let StreamState::Error(message) = &new {
                    warn!("pipewire stream error: {message}");
                    lost.set(true);
                    mainloop.quit();
                }
                data.state_changed(state_of(&new));
            }
        })
        .process(|stream, data| {
            copy_buffer(stream, data);
            data.tick(Instant::now());
        })
        .register()
}

/// moves the next buffer's samples into the ring
fn copy_buffer(stream: &Stream, data: &mut StreamData) {
    let Some(mut buffer) = stream.dequeue_buffer() else {
        return;
    };
    let Some(chunk) = buffer.datas_mut().first_mut() else {
        return;
    };
    let (offset, size) = (chunk.chunk().offset(), chunk.chunk().size());
    if let Some(bytes) = chunk.data() {
        let end = (offset as usize)
            .saturating_add(size as usize)
            .min(bytes.len());
        let start = (offset as usize).min(end);
        if let Some(samples) = bytes.get(start..end) {
            data.push_le_bytes(samples);
        }
    }
}

fn state_of(state: &StreamState) -> CaptureState {
    match state {
        StreamState::Streaming => CaptureState::Streaming,
        StreamState::Paused => CaptureState::Idle,
        StreamState::Connecting | StreamState::Unconnected | StreamState::Error(_) => {
            CaptureState::Connecting
        }
    }
}
