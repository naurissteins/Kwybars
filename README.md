# Kwybars

Kwybars is a desktop audio visualizer for GNU/Linux (Wayland).

Think of it like `cava`... but instead of living in the terminal, it becomes a transparent overlay on your desktop.

You can pin it to the top, bottom, left, or right of your screen and watch your music bounce in real time.

## 🔥 Features

-   Place visualizer on any screen edge `top | bottom | left | right`
-   Control window layer: `background`, `bottom`, `top`
-   Custom overlay size + alignment
-   Solid or gradient bar colors
-   Hot reload config changes (no restart needed!)
-   Optional theme palettes (`assets/themes/*.toml`)
-   Multiple audio backends: `cava` (default), `pipewire`, `dummy` (test animation), `auto` → `cava → pipewire → dummy`
-   Optional `kwybars-daemon` that auto starts/stops overlay based on audio activity

## Requirements

Install dependencies:

``` bash
sudo pacman -S --needed rust gtk4 gtk4-layer-shell pipewire cava
```

## Build and run

``` bash
cargo build --workspace
cargo run -p kwybars-overlay
```
*you must run this inside a Wayland graphical session*

Run daemon mode (auto launch on audio):

``` bash
cargo run -p kwybars-daemon
```

## Configuration

Kwybars looks for config files in this order:

- `KWYBARS_CONFIG` environment variable\
- `$XDG_CONFIG_HOME/kwybars/config.toml`\
- `~/.config/kwybars/config.toml`\
- `./kwybars.toml`

*config files auto reload while the app is running*

### Optional Color Overrides

- `colors.toml` in the same directory as the active `config.toml`.
- Example default path: `~/.config/kwybars/colors.toml`.
- Precedence: `colors.toml` overrides `config.toml` for `color_rgba` and `color2_rgba`
  only when those keys are present in `colors.toml`.
  Useful for customizing appearance with your own colorscheme using matugen

You can place a `colors.toml` next to your config file.

Example:

``` toml
[visualizer]
color_rgba = "rgba(122, 162, 247, 0.95)"
color2_rgba = "rgba(187, 154, 247, 0.95)"
```

`color_rgba` and `color2_rgba` accept:
- CSS-like string: `"rgba(31, 224, 173, 0.90)"`
- plain comma string: `"31,224,173,0.90"` or `"0.12,0.88,0.68,0.90"`

## Themes

- Active theme is selected with `theme` in `config.toml` (optional).
- `theme_opacity` multiplies the theme alpha for all bars.

Theme lookup order for `<theme>.toml`:
1. `~/.config/kwybars/themes/<theme>.toml` (or next to your active `KWYBARS_CONFIG`)
2. Built-in `assets/themes/<theme>.toml`

Available built-in themes:
- `ayu-dark`
- `catppuccin-mocha`
- `dracula`
- `everforest`
- `gruvbox`
- `nord`
- `rose-pine`
- `tokyo-night`

Custom theme example (`~/.config/kwybars/themes/your-theme.toml`):

``` toml
name = "your-theme"

red = "#ea6c73"
green = "#7fd962"
yellow = "#f9af4f"
blue = "#53bdfa"
magenta = "#cda1fa"
cyan = "#90e1c6"
```

Enable one in your config:

``` toml
theme = "catppuccin-mocha"
theme_opacity = 0.85
```

## Example Config

``` toml
[overlay]
monitor_mode = "primary"
layer = "background"
position = "bottom"
full_length = true
height = 620
anchor_margin = 20

[visualizer]
backend = "cava"
bar_corner_radius = 20
bars = 50
bar_width = 8
gap = 20
framerate = 60
color_mode = "gradient"
color_rgba = "rgba(175, 198, 255, 0.7)"
color2_rgba = "rgba(191, 198, 220, 0.7)"

[daemon]
enabled = true
poll_interval_ms = 90
activity_threshold = 0.035
activate_delay_ms = 180
deactivate_delay_ms = 2200
stop_on_silence = true
overlay_command = "kwybars-overlay"
overlay_args = []
```

### Monitor Selection

`primary` uses the first monitor reported by GDK.
For `monitor_mode = "list"`, each monitor entry can be:
- Connector name (recommended), e.g. `"DP-1"`
- 1-based index string, e.g. `"1"`, `"2"`, or `"index:1"`

## Config Reference

Root keys:
- `theme`: optional theme name (same as `[visualizer].theme`).
- `theme_opacity`: theme alpha multiplier `0.0..1.0` (same as `[visualizer].theme_opacity`).

`[overlay]` keys:
- `position`: overlay edge: `bottom|top|left|right`.
- `layer`: stacking layer: `background|bottom|top`.
- `anchor_margin`: margin on the anchored edge.
- `margin_left`: extra left margin.
- `margin_right`: extra right margin.
- `margin_top`: extra top margin.
- `margin_bottom`: extra bottom margin.
- `full_length`: stretch across full edge length.
- `width`: fixed width for horizontal overlays or thickness for vertical overlays.
- `height`: fixed height for vertical overlays or thickness for horizontal overlays.
- `horizontal_alignment`: alignment for top/bottom when `full_length=false`: `left|center|right`.
- `vertical_alignment`: alignment for left/right when `full_length=false`: `top|center|bottom`.
- `monitor_mode`: monitor targeting: `primary|all|list`.
- `monitors`: monitor selector list (connector names like `DP-1` or 1-based indices like `"1"` or or `"index:1"`), used when `monitor_mode="list"`. (`monitors = ["DP-1", "HDMI-A-1"]`)

`[visualizer]` keys:
- `backend`: input backend: `cava|pipewire|auto|dummy`.
- `bars`: number of bars.
- `bar_width`: base bar thickness in pixels.
- `bar_corner_radius`: bar corner radius in pixels (`0` = square bars).
- `gap`: gap between bars in pixels.
- `framerate`: render update rate.
- `color_mode`: `solid|gradient`.
- `color_rgba`: primary bar color.
- `color2_rgba`: secondary color for gradient mode.
- `theme`: optional theme name to load from `assets/themes/<theme>.toml`.
- `theme_opacity`: theme alpha multiplier `0.0..1.0`.
- `pipewire_attack`: PipeWire rise speed tuning.
- `pipewire_decay`: PipeWire fall smoothing.
- `pipewire_gain`: PipeWire sensitivity gain.
- `pipewire_curve`: PipeWire response curve shaping.
- `pipewire_neighbor_mix`: PipeWire neighbor bar smoothing amount.

`[daemon]` keys:
- `enabled`: run daemon logic (`true|false`).
- `poll_interval_ms`: daemon poll period in milliseconds.
- `activity_threshold`: peak level threshold `0.0..1.0` for "audio active".
- `activate_delay_ms`: active signal must stay above threshold for this long before launch.
- `deactivate_delay_ms`: active signal must stay below threshold for this long before stop.
- `stop_on_silence`: if `true`, daemon stops overlay after silence delay.
- `overlay_command`: command used to launch overlay (`kwybars-overlay` by default).
- `overlay_args`: optional command arguments list.

`colors.toml` supported keys:
- `color_rgba`: overrides `[visualizer].color_rgba` when present.
- `color2_rgba`: overrides `[visualizer].color2_rgba` when present.

For local development without installing binaries:

```toml
[daemon]
overlay_command = "cargo"
overlay_args = ["run", "-p", "kwybars-overlay"]
```


Not implemented yet:
-   Direct **PipeWire client** (without `pw-cat`)
