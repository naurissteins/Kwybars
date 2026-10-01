//! a one-off look at the compositor, for `kwybars doctor`

use smithay_client_toolkit::output::{OutputHandler, OutputInfo, OutputState};
use smithay_client_toolkit::reexports::client::globals::registry_queue_init;
use smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput;
use smithay_client_toolkit::reexports::client::{Connection, QueueHandle};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::{delegate_dispatch2, delegate_registry, registry_handlers};

use super::WaylandError;
use super::selection;
use crate::config::OverlayConfig;

const LAYER_SHELL: &str = "zwlr_layer_shell_v1";
const FRACTIONAL_SCALE: &str = "wp_fractional_scale_manager_v1";
const VIEWPORTER: &str = "wp_viewporter";

/// what the compositor offers of the things Kwybars uses
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Probe {
    pub layer_shell: bool,
    pub fractional_scale: bool,
    pub outputs: Vec<ProbedOutput>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProbedOutput {
    pub name: Option<String>,
    pub label: String,
    pub mode: Option<(i32, i32)>,
    pub logical_size: Option<(i32, i32)>,
    pub scale_factor: i32,
}

impl ProbedOutput {
    /// physical over logical pixels, the integer scale when sizes are unknown
    pub fn scale(&self) -> f64 {
        let longest = |(width, height): (i32, i32)| f64::from(width.max(height));
        match (self.mode.map(longest), self.logical_size.map(longest)) {
            (Some(physical), Some(logical)) if physical > 0.0 && logical > 0.0 => {
                physical / logical
            }
            _ => f64::from(self.scale_factor.max(1)),
        }
    }
}

impl Probe {
    pub fn selected(&self, overlay: &OverlayConfig) -> (Vec<bool>, Vec<String>) {
        let names: Vec<Option<&str>> = self
            .outputs
            .iter()
            .map(|output| output.name.as_deref())
            .collect();
        let selection = selection::select(overlay, &names);
        let chosen = (0..self.outputs.len())
            .map(|index| {
                selection
                    .targets
                    .iter()
                    .any(|target| target.output == index)
            })
            .collect();
        (chosen, selection.warnings)
    }
}

/// connects, lists globals and outputs, and disconnects
pub fn probe() -> Result<Probe, WaylandError> {
    let connection = Connection::connect_to_env()?;
    let (globals, mut queue) = registry_queue_init::<State>(&connection)?;
    let qh = queue.handle();
    let mut state = State {
        registry: RegistryState::new(&globals),
        outputs: OutputState::new(&globals, &qh),
    };
    // outputs describe themselves in answer to being bound, their xdg-output
    // part in answer to a request sent while handling that
    queue.roundtrip(&mut state)?;
    queue.roundtrip(&mut state)?;

    let has = |interface: &str| {
        globals
            .contents()
            .with_list(|list| list.iter().any(|global| global.interface == interface))
    };
    let outputs = state
        .outputs
        .outputs()
        .filter_map(|output| state.outputs.info(&output))
        .map(probed)
        .collect();
    Ok(Probe {
        layer_shell: has(LAYER_SHELL),
        fractional_scale: has(FRACTIONAL_SCALE) && has(VIEWPORTER),
        outputs,
    })
}

fn probed(info: OutputInfo) -> ProbedOutput {
    let label = match &info.name {
        Some(name) if !name.is_empty() => name.clone(),
        _ => format!("{} {}", info.make, info.model).trim().to_owned(),
    };
    ProbedOutput {
        label,
        mode: info
            .modes
            .iter()
            .find(|mode| mode.current)
            .map(|mode| mode.dimensions),
        logical_size: info.logical_size,
        scale_factor: info.scale_factor,
        name: info.name,
    }
}

struct State {
    registry: RegistryState,
    outputs: OutputState,
}

impl OutputHandler for State {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.outputs
    }

    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {}

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {}

    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {}
}

impl ProvidesRegistryState for State {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }

    registry_handlers![OutputState];
}

delegate_registry!(State);
delegate_dispatch2!(State);

#[cfg(test)]
mod tests {
    use super::{Probe, ProbedOutput};
    use crate::config::{MonitorMode, OverlayConfig};

    fn output(name: &str, mode: (i32, i32), logical: (i32, i32)) -> ProbedOutput {
        ProbedOutput {
            name: Some(name.to_owned()),
            label: name.to_owned(),
            mode: Some(mode),
            logical_size: Some(logical),
            scale_factor: 2,
        }
    }

    #[test]
    fn scale_is_physical_over_logical_even_when_rotated() {
        assert_eq!(output("A", (3840, 2160), (2560, 1440)).scale(), 1.5);
        assert_eq!(output("A", (3840, 2160), (1440, 2560)).scale(), 1.5);
        let unknown = ProbedOutput {
            scale_factor: 2,
            ..ProbedOutput::default()
        };
        assert_eq!(unknown.scale(), 2.0);
    }

    #[test]
    fn selection_follows_the_overlay_config() {
        let probe = Probe {
            layer_shell: true,
            fractional_scale: true,
            outputs: vec![
                output("DP-1", (1920, 1080), (1920, 1080)),
                output("DP-4", (1920, 1080), (1920, 1080)),
            ],
        };
        let primary = OverlayConfig::default();
        assert_eq!(probe.selected(&primary).0, vec![true, false]);
        let listed = OverlayConfig {
            monitor_mode: MonitorMode::List,
            monitors: vec!["DP-4".to_owned()],
            ..OverlayConfig::default()
        };
        assert_eq!(probe.selected(&listed), (vec![false, true], Vec::new()));
    }
}
