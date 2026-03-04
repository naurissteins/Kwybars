# Kwybars

Kwybars is a Wayland-first desktop audio visualizer for Linux.

It is inspired by terminal visualizers like `cava`, but the target UX is a real
transparent desktop overlay anchored to a screen edge.

Current status: GTK4 + layer-shell + live audio backend scaffold.

## Current Features

- GTK4 application window for visualizer rendering.
- Wayland layer-shell anchoring via `gtk4-layer-shell`.
- Edge placement from config: `bottom`, `top`, `left`, `right`.
- Configurable overlay size and alignment.
- Live frame backend with selectable input:
  - `cava`: primary/default backend
  - `pipewire`: fallback or explicit backend
  - `dummy`: synthetic animation
  - `auto`: same priority as default (`cava -> pipewire -> dummy`)

Not implemented yet:

- Direct in-process PipeWire client (without external `pw-cat`).
- Multi-monitor control.
- User theming controls.

## Requirements (Arch Linux)

```bash
sudo pacman -S --needed rust gtk4 gtk4-layer-shell pipewire cava
```

## Build

```bash
cd /home/ns/Projects/Kwybars/Kwybars
cargo build --workspace
```

## Run

```bash
cd /home/ns/Projects/Kwybars/Kwybars
cargo run -p kwybars-overlay
```

Run this in a graphical Wayland session. Without a display server, GTK exits
with `Failed to open display`.

## Configuration

Config path resolution order:

1. `KWYBARS_CONFIG`
2. `$XDG_CONFIG_HOME/kwybars/config.toml`
3. `~/.config/kwybars/config.toml`
4. `./kwybars.toml`

### Full-width bottom visualizer (default style)

```toml
[overlay]
position = "bottom"
anchor_margin = 12
full_length = true
height = 120

[visualizer]
backend = "cava"
bars = 48
bar_width = 6
gap = 3
framerate = 60
```

### Fixed-width bottom visualizer, centered

```toml
[overlay]
position = "bottom"
full_length = false
width = 1200
height = 120
horizontal_alignment = "center" # left | center | right
```

### Fixed-width bottom visualizer, right aligned

```toml
[overlay]
position = "bottom"
full_length = false
width = 900
height = 120
horizontal_alignment = "right"
```

### PipeWire tuning (used only for PipeWire backend)

```toml
[visualizer]
pipewire_attack = 0.14
pipewire_decay = 0.975
pipewire_gain = 1.20
pipewire_curve = 0.95
pipewire_neighbor_mix = 0.24
```

## Workspace Layout

- `crates/common`: shared config and frame model.
- `crates/engine`: visualizer frame pipeline and live source backends.
- `crates/overlay`: GTK overlay app (windowing + rendering).
