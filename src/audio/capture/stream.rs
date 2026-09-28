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
use pipewire::spa::param::audio::AudioInfoRaw;
use pipewire::spa::pod::Pod;
use pipewire::spa::utils::Direction;
use pipewire::stream::{StreamFlags, StreamListener, StreamRc, StreamState};
use tracing::{debug, warn};

use super::format;
use super::ring::SampleRing;
use super::status::CaptureState;
use super::{CaptureSettings, Shared};
use crate::audio::pipeline::Pipeline;

/// seconds of audio the ring holds
const RING_SECONDS: usize = 1;

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
        settings: &CaptureSettings,
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
        let data = StreamData::new(Arc::clone(shared), settings.clone());
        let stream_listener = stream
            .add_local_listener_with_user_data(data)
            .param_changed(|_, data, id, param| {
                if let Some(param) = param
                    && id == ParamType::Format.as_raw()
                {
                    data.format_changed(param);
                }
            })
            .state_changed({
                let (mainloop, lost) = (mainloop.clone(), Rc::clone(&lost));
                move |_, data, _old, new| {
                    if let StreamState::Error(message) = &new {
                        warn!("pipewire stream error: {message}");
                        lost.set(true);
                        mainloop.quit();
                    }
                    data.shared.status.set_state(state_of(&new));
                }
            })
            .process(|stream, data| {
                if let Some(mut buffer) = stream.dequeue_buffer()
                    && let Some(chunk) = buffer.datas_mut().first_mut()
                {
                    let (offset, size) = (chunk.chunk().offset(), chunk.chunk().size());
                    if let Some(bytes) = chunk.data() {
                        let end = (offset as usize)
                            .saturating_add(size as usize)
                            .min(bytes.len());
                        let start = (offset as usize).min(end);
                        if let Some(samples) = bytes.get(start..end) {
                            data.ring.push_le_bytes(samples);
                        }
                    }
                }
                data.tick(Instant::now());
            })
            .register()?;

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

fn state_of(state: &StreamState) -> CaptureState {
    match state {
        StreamState::Streaming => CaptureState::Streaming,
        StreamState::Paused => CaptureState::Idle,
        StreamState::Connecting | StreamState::Unconnected | StreamState::Error(_) => {
            CaptureState::Connecting
        }
    }
}

/// state owned by the stream callbacks, all on the capture thread
struct StreamData {
    shared: Arc<Shared>,
    settings: CaptureSettings,
    format: AudioInfoRaw,
    ring: SampleRing,
    /// created when the format is known
    pipeline: Option<Pipeline>,
    channels: u64,
    /// ring position the last analysis stopped at
    cursor: u64,
    last_tick: Option<Instant>,
    /// silence was already published, nothing to send until sound returns
    silence_published: bool,
}

impl StreamData {
    fn new(shared: Arc<Shared>, settings: CaptureSettings) -> Self {
        Self {
            shared,
            settings,
            format: AudioInfoRaw::default(),
            ring: SampleRing::default(),
            pipeline: None,
            channels: 1,
            cursor: 0,
            last_tick: None,
            silence_published: false,
        }
    }

    /// resizes buffers for a newly negotiated format; not called per buffer
    fn format_changed(&mut self, param: &Pod) {
        if self.format.parse(param).is_err() {
            return;
        }
        let (rate, channels) = (self.format.rate(), self.format.channels());
        debug!("capture format: {rate} Hz, {channels} channels, f32");
        self.shared.status.set_format(rate, channels);
        self.channels = u64::from(channels.max(1));
        self.ring
            .reset(rate as usize * channels as usize * RING_SECONDS);
        self.cursor = 0;
        self.pipeline = (rate > 0 && channels > 0)
            .then(|| Pipeline::new(rate, channels, &self.settings.spectrum));
    }

    /// analyzes new samples at most once per interval; must not allocate
    fn tick(&mut self, now: Instant) {
        if self
            .last_tick
            .is_some_and(|last| now.duration_since(last) < self.settings.interval)
        {
            return;
        }
        self.last_tick = Some(now);

        let mut peak = 0.0_f32;
        let pipeline = &mut self.pipeline;
        self.cursor = self.ring.read_since(self.cursor, |sample| {
            peak = peak.max(sample.abs());
            if let Some(pipeline) = pipeline.as_mut() {
                pipeline.push_interleaved(sample);
            }
        });

        let status = &self.shared.status;
        status.set_peak(peak);
        status.set_frames(self.ring.written() / self.channels);
        let Some(pipeline) = self.pipeline.as_mut() else {
            return;
        };
        match pipeline.analyze(peak) {
            Some(values) => {
                self.shared.frames.publish(values);
                self.silence_published = false;
            }
            None if !self.silence_published => {
                self.shared.frames.publish_silence();
                self.silence_published = true;
            }
            None => {}
        }
    }
}
