//! the capture thread: owns the pipewire loop and reconnects when needed

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use pipewire::channel::Receiver;
use pipewire::main_loop::MainLoopRc;
use tracing::{debug, error, info, warn};

use super::status::CaptureState;
use super::stream::Session;
use super::tuning::Tuning;
use super::{CaptureSettings, Shared};
use crate::audio::frame::FrameSlot;

const FIRST_RETRY: Duration = Duration::from_secs(1);
const MAX_RETRY: Duration = Duration::from_secs(30);

/// requests from the owning thread
pub enum Command {
    Stop,
    Reconfigure {
        settings: CaptureSettings,
        frames: Arc<FrameSlot>,
    },
}

/// runs until [`Command::Stop`]; pipewire outages are retried with backoff
pub(super) fn run(
    commands: Receiver<Command>,
    shared: Arc<Shared>,
    settings: CaptureSettings,
    frames: Arc<FrameSlot>,
) {
    let status = &shared.status;
    pipewire::init();
    let mainloop = match MainLoopRc::new(None) {
        Ok(mainloop) => mainloop,
        Err(err) => {
            error!("could not create the pipewire main loop: {err}");
            status.set_state(CaptureState::Failed);
            return;
        }
    };

    let tuning = Rc::new(RefCell::new(Tuning {
        settings,
        frames,
        generation: 0,
    }));
    let stop = Rc::new(Cell::new(false));
    let _commands = commands.attach(mainloop.loop_(), {
        let (mainloop, stop, tuning) = (mainloop.clone(), Rc::clone(&stop), Rc::clone(&tuning));
        move |command| obey(command, &mainloop, &stop, &tuning)
    });

    let mut retry = FIRST_RETRY;
    let mut reported_unavailable = false;
    while !stop.get() {
        status.set_state(CaptureState::Connecting);
        match Session::start(&mainloop, &shared, &tuning) {
            Ok(session) => {
                info!("capturing the default output through pipewire");
                reported_unavailable = false;
                let started = Instant::now();
                mainloop.run();
                // only a session that held up resets the backoff, one that
                // fails right after connecting keeps backing off
                if started.elapsed() >= MAX_RETRY {
                    retry = FIRST_RETRY;
                }
                if session.lost() && !stop.get() {
                    warn!("lost the pipewire connection, reconnecting");
                }
                // no stream, no sound, whether or not a state change said so
                shared.publish(&tuning.borrow().frames, None, 0.0);
            }
            Err(err) => {
                status.set_state(CaptureState::Unavailable);
                if reported_unavailable {
                    debug!("pipewire still unavailable: {err}");
                } else {
                    warn!("pipewire is not available ({err}), retrying in the background");
                    reported_unavailable = true;
                }
            }
        }
        if !stop.get() {
            wait(&mainloop, retry);
            retry = (retry * 2).min(MAX_RETRY);
        }
    }
    status.set_state(CaptureState::Stopped);
}

fn obey(command: Command, mainloop: &MainLoopRc, stop: &Cell<bool>, tuning: &RefCell<Tuning>) {
    match command {
        Command::Stop => {
            stop.set(true);
            mainloop.quit();
        }
        Command::Reconfigure { settings, frames } => {
            debug!("capture: new analysis settings {settings:?}");
            let mut tuning = tuning.borrow_mut();
            tuning.settings = settings;
            tuning.frames = frames;
            tuning.generation += 1;
        }
    }
}

/// runs the loop for `delay` so a stop request still gets through
fn wait(mainloop: &MainLoopRc, delay: Duration) {
    let quit = mainloop.clone();
    let timer = mainloop.loop_().add_timer(move |_| quit.quit());
    if let Err(err) = timer.update_timer(Some(delay), None).into_result() {
        warn!("could not arm the pipewire retry timer: {err}");
        std::thread::sleep(delay);
        return;
    }
    mainloop.run();
}
