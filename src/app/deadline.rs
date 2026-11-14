//! timer replacement policy while calloop dispatches a batch of ready events

use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Change {
    Keep,
    Cancel,
    Arm(Instant),
}

pub(super) fn change(current: Option<Instant>, wanted: Option<Instant>, now: Instant) -> Change {
    // an expired timer may already be in calloop's current event batch. Its
    // callback must consume that event, even when another source removes the deadline
    if current.is_some_and(|at| at <= now || wanted.is_some_and(|wanted| at <= wanted)) {
        return Change::Keep;
    }
    wanted.map_or(Change::Cancel, Change::Arm)
}

#[cfg(test)]
mod tests {
    use super::{Change, change};
    use calloop::timer::{TimeoutAction, Timer};
    use calloop::{EventLoop, RegistrationToken};
    use std::time::{Duration, Instant};

    #[test]
    fn a_future_timer_can_be_cancelled_or_advanced() {
        let now = Instant::now();
        let later = now + Duration::from_secs(2);
        assert_eq!(change(Some(later), None, now), Change::Cancel);
        assert_eq!(change(Some(later), Some(now), now), Change::Arm(now));
    }

    #[test]
    fn another_ready_source_cannot_remove_a_queued_timer() -> Result<(), Box<dyn std::error::Error>>
    {
        struct State {
            timer: Option<(RegistrationToken, Instant)>,
            fired: bool,
        }
        let mut event_loop = EventLoop::<State>::try_new()?;
        let handle = event_loop.handle();
        let earlier = Instant::now() - Duration::from_secs(2);
        let due = earlier + Duration::from_secs(1);
        let cancel = handle.clone();
        // This source represents a Wayland event dispatched before the queued timeout.
        handle
            .insert_source(Timer::from_deadline(earlier), move |_, (), state| {
                if change(state.timer.map(|(_, at)| at), None, Instant::now()) == Change::Cancel
                    && let Some((token, _)) = state.timer.take()
                {
                    cancel.remove(token);
                }
                TimeoutAction::Drop
            })
            .map_err(|error| error.error)?;
        let token = handle
            .insert_source(Timer::from_deadline(due), |_, (), state| {
                state.timer = None;
                state.fired = true;
                TimeoutAction::Drop
            })
            .map_err(|error| error.error)?;
        let mut state = State {
            timer: Some((token, due)),
            fired: false,
        };
        event_loop.dispatch(Duration::ZERO, &mut state)?;
        assert!(
            state.fired,
            "the queued timeout must be consumed before unregistering"
        );
        assert!(state.timer.is_none());
        Ok(())
    }
}
