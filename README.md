# Kwybars

Kwybars is a Wayland-first desktop audio visualizer for Linux.

It is inspired by terminal visualizers like `cava`, but the target UX is a real
transparent desktop overlay anchored to a screen edge.

Current status: GTK4 + layer-shell + live audio backend scaffold.

## Current Features

- GTK4 application window for visualizer rendering.
- Wayland layer-shell anchoring via `gtk4-layer-shell`.
- Edge placement from config: `bottom`, `top`, `left`, `right`.
- Configurable overlay layer mode: `background`, `bottom`, `top`.
- Configurable overlay size and alignment.
- Configurable bar color via solid or gradient RGBA styles.
- Hot reload of config file changes while app is running.
- Optional theme palettes from `assets/themes/*.toml`.
- Live frame backend with selectable input:
  - `cava`: primary/default backend
  - `pipewire`: fallback or explicit backend
  - `dummy`: synthetic animation
  - `auto`: same priority as default (`cava -> pipewire -> dummy`)

Not implemented yet:

- Direct in-process PipeWire client (without external `pw-cat`).
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

Config is hot-reloaded automatically while the app is running. You do not need
to restart after editing `config.toml`.

Optional color override file:
- `colors.toml` in the same directory as the active `config.toml`.
- Example default path: `~/.config/kwybars/colors.toml`.
- Precedence: `colors.toml` overrides `config.toml` for `color_rgba` and `color2_rgba`
  only when those keys are present in `colors.toml`.

`colors.toml` example:

```toml
[visualizer]
color_rgba = "rgba(122, 162, 247, 0.95)"
color2_rgba = "rgba(187, 154, 247, 0.95)"
```

Theme files:
- Built-in theme directory: `assets/themes`.
- Active theme is selected with `theme` in `config.toml` (optional).
- `theme_opacity` multiplies the theme alpha for all bars.

### Full-width bottom visualizer behind windows (default style)

```toml
[overlay]
position = "bottom"
layer = "background" # background | bottom | top
anchor_margin = 12
margin_left = 24
margin_right = 24
full_length = true
height = 120
monitor_mode = "primary" # primary | all | list

[visualizer]
backend = "cava"
bars = 48
bar_width = 6
gap = 3
framerate = 60
color_mode = "solid" # solid | gradient
color_rgba = "rgba(31, 224, 173, 0.90)"
color2_rgba = "rgba(31, 224, 173, 0.90)" # used when color_mode = "gradient"
theme = "" # optional, e.g. "catppuccin-mocha"
theme_opacity = 1.0 # 0.0..1.0
```

`color_rgba` and `color2_rgba` accept:
- CSS-like string: `"rgba(31, 224, 173, 0.90)"`
- plain comma string: `"31,224,173,0.90"` or `"0.12,0.88,0.68,0.90"`

### Enable a 6-color theme palette

```toml
theme = "catppuccin-mocha"
theme_opacity = 0.85
```

You can place theme keys at root (shown above) or under `[visualizer]`.
When theme loading succeeds, theme colors are used for per-bar coloring and
regular `color_rgba`/`color2_rgba` are ignored.

### Gradient bars

```toml
[visualizer]
color_mode = "gradient"
color_rgba = "rgba(31, 224, 173, 0.95)"
color2_rgba = "rgba(53, 144, 255, 0.95)"
```

### Keep visualizer above windows

```toml
[overlay]
layer = "top"
```

### Select monitor targets

```toml
[overlay]
monitor_mode = "all"
```

```toml
[overlay]
monitor_mode = "list"
monitors = ["DP-1", "HDMI-A-1"] # connector names
```

`primary` uses the first monitor reported by GDK.
For `monitor_mode = "list"`, each monitor entry can be:
- Connector name (recommended), e.g. `"DP-1"`
- 1-based index string, e.g. `"1"`, `"2"`, or `"index:1"`

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

### Add left/right margins while keeping full width

```toml
[overlay]
position = "bottom"
full_length = true
margin_left = 80
margin_right = 260
```

`anchor_margin` controls the primary anchored edge:
- `bottom` position -> bottom margin
- `top` position -> top margin
- `left` position -> left margin
- `right` position -> right margin

Per-edge margins control the cross-axis:
- `margin_left`, `margin_right`, `margin_top`, `margin_bottom`
- Example: for `position = "bottom"`, use `margin_left`/`margin_right`.

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
