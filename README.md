# Kwybars

Kwybars is a desktop audio visualizer for GNU/Linux (Wayland).

Think of it like `cava`... but instead of living in the terminal, it becomes a transparent overlay on your desktop.

You can pin it to the top, bottom, left, or right of your screen and watch your music bounce in real time.

https://github.com/user-attachments/assets/65c97990-bc8a-490a-bc07-9c68bc214678

## 🔥 Features

-   Place visualizer on any screen edge `top | bottom | left | right`
-   Control window layer: `background`, `bottom`, `top`
-   Custom overlay size + alignment
-   Solid or gradient bar colors
-   Optional segmented bar style (oldschool split blocks)
-   Optional radial layout centered on the monitor
-   Hot reload config changes (no restart needed!)
-   Optional custom theme palettes (`~/.config/kwybars/themes/*.toml`)
-   Multiple audio backends: `cava` (default), `pipewire`, `dummy` (test animation), `auto`
-   Optional `kwybars-daemon` that auto starts/stops overlay based on audio activity

## Installation
### AUR (Arch Linux)

``` bash
yay -S kwybars # Not available yet via AUR
```

### Install from source
Install dependencies:

``` bash
sudo pacman -S --needed rust gtk4 gtk4-layer-shell pipewire cava libnotify
```

## Build and run

``` bash
cargo build --workspace
cargo run -p kwybars-overlay
```

Run daemon mode (auto launch on audio):

``` bash
cargo run -p kwybars-daemon
```

## Configuration

Kwybars looks for config files in this order:

- `KWYBARS_CONFIG` environment variable
- `$XDG_CONFIG_HOME/kwybars/config.toml`
- `~/.config/kwybars/config.toml` (recommended)
- `./kwybars.toml`

*config files auto reload while the app is running*

If no config file exists, Kwybars uses the built-in defaults.

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

## Matugen (optional)
Change bars colors `color_rgba` and `color2_rgba` with Matugen

1. Create a new file `kwybars-colors.toml` in `~/.config/matugen/templates`
2. Add the following content to `kwybars-colors.toml`:
```toml
[visualizer]
color_rgba = "{{colors.primary.default.rgba | set_alpha: 0.7}}"
color2_rgba = "{{colors.secondary.default.rgba | set_alpha: 0.7}}"
```


3. Then add the following to your matugen config file `~/.config/matugen/config.toml`:
```toml
[templates.kwybars]
input_path = '~/.config/matugen/templates/kwybars-colors.json'
output_path = '~/.config/kwybars/colors.toml'
```

## Themes

- Active theme is selected with `theme` in `config.toml` (optional).
- `theme_opacity` multiplies the theme alpha for all bars.

Theme lookup order for `<theme>.toml`:
`~/.config/kwybars/themes/<theme>.toml` (or next to your active `KWYBARS_CONFIG`)

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

## Default Config

``` toml
[overlay]
monitor_mode = "primary"
layer = "background"
position = "bottom"
full_length = true
height = 620
anchor_margin = 20
margin_left = 20
margin_right = 20

[visualizer]
backend = "cava"
layout = "line"
bar_corner_radius = 20
segmented_bars = false
segment_length = 12
segment_gap = 6
radial_inner_radius = 180
radial_start_angle = -90
radial_arc_degrees = 360
bars = 50
bar_width = 8
gap = 20
framerate = 60
color_mode = "gradient"
color_rgba = "rgba(175, 198, 255, 0.7)"
color2_rgba = "rgba(191, 198, 220, 0.7)"

# By default daemon is already enabled and configured for you. 
# Use in your config only if you need to customize.
[daemon]
enabled = true
poll_interval_ms = 90
activity_threshold = 0.035
activate_delay_ms = 180
deactivate_delay_ms = 2200
stop_on_silence = true
notify_on_error = true
notify_cooldown_seconds = 45
overlay_command = "kwybars-overlay"
overlay_args = []
```

## Config Reference

`[overlay]`
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
- `monitor_mode`: monitor targeting: `primary|all|list` (default: `primary`)
- `monitors`: monitor selector list (connector names like `DP-1` or 1-based indices like `"1"`), used when `monitor_mode="list"`. (`monitors = ["DP-1", "HDMI-A-1"]`)

`[visualizer]`
- `layout`: layout mode: `line|radial`.
- `bars`: number of bars.
- `bar_width`: base bar thickness in pixels.
- `bar_corner_radius`: bar corner radius in pixels (`0` = square bars).
- `segmented_bars`: split each bar into repeated segments (`true|false`).
- `segment_length`: segment size in pixels along bar growth direction.
- `segment_gap`: empty spacing in pixels between segments.
- `radial_inner_radius`: inner circle radius in pixels for `layout="radial"`.
- `radial_start_angle`: arc start angle in degrees for `layout="radial"` (`-90` starts at the top).
- `radial_arc_degrees`: arc span in degrees for `layout="radial"` (`360` = full ring, `180` = half circle).
- `gap`: gap between bars in pixels.
- `framerate`: render update rate (default: `60`).
- `color_mode`: `solid|gradient` (default: `gradient`). Solid color mode uses `color_rgba`, gradient mode uses both `color_rgba` and `color2_rgba`.
- `color_rgba`: primary bar color (default: `rgba(175, 198, 255, 0.7)`)
- `color2_rgba`: secondary color for gradient mode (default: `rgba(191, 198, 220, 0.7)`)
- `theme`: optional theme name to load from `~/.config/kwybars/themes/<theme>.toml` or built-in themes. Available themes: `ayu-dark`, `catppuccin-mocha`, `dracula`, `everforest`, `gruvbox`, `nord`, `rose-pine` and `tokyo-night` (default: `none`).
- `theme_opacity`: theme alpha multiplier `0.0..1.0` (default: `1.0`).

Example half-circle radial layout:

```toml
[visualizer]
layout = "radial"
radial_inner_radius = 160
radial_start_angle = -180
radial_arc_degrees = 180
```

`[daemon]`
- `enabled`: run daemon logic (`true|false`).
- `poll_interval_ms`: daemon poll period in milliseconds.
- `activity_threshold`: peak level threshold `0.0..1.0` for "audio active".
- `activate_delay_ms`: active signal must stay above threshold for this long before launch.
- `deactivate_delay_ms`: active signal must stay below threshold for this long before stop.
- `stop_on_silence`: if `true`, daemon stops overlay after silence delay.
- `notify_on_error`: enable desktop notifications (`notify-send`) for important runtime errors.
- `notify_cooldown_seconds`: minimum seconds between repeated notifications for the same error.
- `overlay_command`: command used to launch overlay (`kwybars-overlay` by default).
- `overlay_args`: optional command arguments list.

Config parse errors include line numbers (for example: `line 42: unknown overlay key: ...`).

For local development without installing binaries:

```toml
[daemon]
overlay_command = "cargo"
overlay_args = ["run", "-p", "kwybars-overlay"]
```

## Logging

- Both `kwybars-overlay` and `kwybars-daemon` write logs to stderr and to a file.
- Default log files:
  - `~/.local/state/kwybars/overlay.log`
  - `~/.local/state/kwybars/daemon.log`
  - (`$XDG_STATE_HOME/kwybars/*.log` if `XDG_STATE_HOME` is set)
- You can set log level with `KWYBARS_LOG` (or `RUST_LOG`), for example:
  - `KWYBARS_LOG=debug cargo run -p kwybars-daemon`
- Override log file location with `KWYBARS_LOG_FILE=/path/to/kwybars.log`
