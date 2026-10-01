use std::collections::BTreeMap;
use std::fmt;

use serde::Deserialize;
use serde::de::value::MapAccessDeserializer;
use serde::de::{self, Deserializer, MapAccess, Visitor};

use super::{OutputConfig, OverlayConfig};
use crate::config::types::{
    Edge, HorizontalAlignment, Layer, MonitorMode, ShowOn, VerticalAlignment, clean_names,
};

/// the [overlay] table, None keeps the default
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct OverlayTable {
    position: Option<Edge>,
    layer: Option<Layer>,
    anchor_margin: Option<u32>,
    margin_left: Option<u32>,
    margin_right: Option<u32>,
    margin_top: Option<u32>,
    margin_bottom: Option<u32>,
    fade_in_ms: Option<u64>,
    fade_out_ms: Option<u64>,
    full_length: Option<bool>,
    width: Option<u32>,
    height: Option<u32>,
    horizontal_alignment: Option<HorizontalAlignment>,
    vertical_alignment: Option<VerticalAlignment>,
    show_on: Option<ShowOn>,
    monitor_mode: Option<MonitorMode>,
    monitors: Option<Vec<String>>,
    outputs: Vec<LegacyOutput>,
}

/// one [[overlay.outputs]] entry
#[derive(Debug)]
struct LegacyOutput(OutputConfig);

impl<'de> Deserialize<'de> for LegacyOutput {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct EntryVisitor;

        impl<'de> Visitor<'de> for EntryVisitor {
            type Value = LegacyOutput;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an [[overlay.outputs]] table")
            }

            // `outputs = ["DP-2"]` is how other programs choose monitors
            fn visit_str<E: de::Error>(self, name: &str) -> Result<LegacyOutput, E> {
                Err(E::custom(format!(
                    "overlay.outputs does not take monitor names; to show the bars on {name:?} write `show_on = [{name:?}]` in [overlay]"
                )))
            }

            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<LegacyOutput, A::Error> {
                OutputConfig::deserialize(MapAccessDeserializer::new(map)).map(LegacyOutput)
            }
        }

        deserializer.deserialize_any(EntryVisitor)
    }
}

macro_rules! apply_set_fields {
    ($from:expr, $to:expr, [$($field:ident),+ $(,)?]) => {
        $(
            if let Some(value) = $from.$field {
                $to.$field = value;
            }
        )+
    };
}

pub fn resolve(
    table: OverlayTable,
    sections: BTreeMap<String, OutputConfig>,
    warnings: &mut Vec<String>,
) -> Result<OverlayConfig, String> {
    let mut overlay = OverlayConfig::default();
    apply_set_fields!(
        table,
        overlay,
        [
            position,
            layer,
            anchor_margin,
            margin_left,
            margin_right,
            margin_top,
            margin_bottom,
            fade_in_ms,
            fade_out_ms,
            full_length,
            width,
            height,
            horizontal_alignment,
            vertical_alignment,
        ]
    );

    let legacy = legacy_outputs(table.outputs, warnings)?;
    overlay.outputs = named_sections(sections, warnings);
    if overlay.outputs.is_empty() {
        overlay.outputs = legacy;
    } else if !legacy.is_empty() {
        warnings.push(
            "[[overlay.outputs]] is ignored because [output.NAME] sections are present".to_owned(),
        );
    }
    for output in &mut overlay.outputs {
        let table = format!("output.{}.visualizer", output.monitor);
        output.visualizer.drop_global_keys(&table, warnings);
        output.visualizer.normalize(&table, warnings);
    }

    let legacy = LegacySelection {
        mode: table.monitor_mode,
        monitors: table
            .monitors
            .map(|names| clean_names(names.iter().map(String::as_str))),
    };
    overlay.show_on = match table.show_on {
        Some(show_on) => {
            legacy.unused(warnings);
            show_on
        }
        None if !overlay.outputs.is_empty() => {
            legacy.unused(warnings);
            ShowOn::Sections
        }
        None => legacy.show_on(warnings),
    };
    Ok(overlay)
}

/// the [output.NAME] sections, named by their key
fn named_sections(
    sections: BTreeMap<String, OutputConfig>,
    warnings: &mut Vec<String>,
) -> Vec<OutputConfig> {
    let mut outputs = Vec::with_capacity(sections.len());
    for (name, mut output) in sections {
        let name = name.trim();
        if name.is_empty() {
            warnings.push("[output.\"\"]: a section needs a monitor name, ignored".to_owned());
            continue;
        }
        if !output.monitor.is_empty() {
            warnings.push(format!(
                "output.{name}.monitor: the section name is the monitor, ignored"
            ));
        }
        output.monitor = name.to_owned();
        outputs.push(output);
    }
    outputs
}

fn legacy_outputs(
    entries: Vec<LegacyOutput>,
    warnings: &mut Vec<String>,
) -> Result<Vec<OutputConfig>, String> {
    let mut outputs = Vec::with_capacity(entries.len());
    for (index, LegacyOutput(mut output)) in entries.into_iter().enumerate() {
        output.monitor = output.monitor.trim().to_owned();
        if output.monitor.is_empty() {
            return Err(format!("overlay.outputs[{index}]: missing `monitor`"));
        }
        outputs.push(output);
    }
    if let Some(first) = outputs.first() {
        warnings.push(format!(
            "[[overlay.outputs]] is deprecated, write each entry as its own section, for example [output.{}]",
            first.monitor
        ));
    }
    Ok(outputs)
}

/// the deprecated monitor_mode and monitors keys
struct LegacySelection {
    mode: Option<MonitorMode>,
    monitors: Option<Vec<String>>,
}

impl LegacySelection {
    fn keys(&self) -> Option<&'static str> {
        match (self.mode.is_some(), self.monitors.is_some()) {
            (true, true) => Some("overlay.monitor_mode, overlay.monitors"),
            (true, false) => Some("overlay.monitor_mode"),
            (false, true) => Some("overlay.monitors"),
            (false, false) => None,
        }
    }

    /// warns that the keys are set but something else chooses the monitors
    fn unused(&self, warnings: &mut Vec<String>) {
        if let Some(keys) = self.keys() {
            warnings.push(format!(
                "{keys}: deprecated and not used here, `show_on` or the [output.NAME] sections choose the monitors"
            ));
        }
    }

    /// what the keys select, with the show_on line that replaces them
    fn show_on(self, warnings: &mut Vec<String>) -> ShowOn {
        let Some(keys) = self.keys() else {
            return ShowOn::Primary;
        };
        let names = self.monitors.unwrap_or_default();
        let quoted = format!("{names:?}");
        let (show_on, replacement) = match self.mode {
            Some(MonitorMode::All) => (ShowOn::All, "\"all\"".to_owned()),
            Some(MonitorMode::List) => (ShowOn::Named(names), quoted),
            Some(MonitorMode::Primary) | None if names.is_empty() => {
                (ShowOn::Primary, "\"primary\"".to_owned())
            }
            // a list without `monitor_mode = "list"` never did anything
            Some(MonitorMode::Primary) | None => {
                warnings.push(format!(
                    "overlay.monitors has no effect without `monitor_mode = \"list\"`, the primary monitor is used; to show the bars on these monitors write `show_on = {quoted}`"
                ));
                return ShowOn::Primary;
            }
        };
        warnings.push(format!(
            "{keys}: deprecated, write `show_on = {replacement}` instead"
        ));
        show_on
    }
}
