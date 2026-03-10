<h1 align=center>kwybars</h1>

<div align=center>

![GitHub last commit](https://img.shields.io/github/last-commit/naurissteins/kwybars?style=for-the-badge&labelColor=181825&color=a6e3a1)
![GitHub repo size](https://img.shields.io/github/repo-size/naurissteins/kwybars?style=for-the-badge&labelColor=181825&color=d3bfe6)
![AUR Version](https://img.shields.io/aur/version/kwybars-bin?style=for-the-badge&labelColor=181825&color=b4befe)
![GitHub Repo stars](https://img.shields.io/github/stars/naurissteins/kwybars?style=for-the-badge&labelColor=181825&color=f9e2af)

</div>

Kwybars is a GTK4-based desktop audio visualizer for GNU/Linux (Wayland) that renders real-time audio bars on your screen.

Think of it like `cava`... but instead of living in the terminal, it becomes a transparent overlay on your desktop. Place visualizer on any screen edge: top, bottom, left, right or center and watch your music bounce in real time. Kwybars are highly customizable with multiple layouts, segmented bars, gradients, themes, and extensive configuration options.

## 🔥 Features

-   Place visualizer on any screen edge `top | bottom | left | right`
-   Multi-monitor support: show bars on primary, all, or selected monitors
-   Control window layer: `background`, `bottom`, `top`
-   Custom overlay size + alignment
-   Solid or gradient bar colors
-   Segmented bar style (oldschool split blocks)
-   Radial layout (circular)
-   Polygon layout for triangle, square, hexagon, and similar shapes
-   Hot reload config changes (no restart needed)
-   Built-in themes and optional custom theme palettes (`~/.config/kwybars/themes/*.toml`)
-   Optional but recommended `kwybars-daemon` that auto starts/stops overlay based on audio activity

> [!NOTE]
> Kwybars are not heavily tested on all Wayland compositors yet. If you encounter issues, please open an issue and provide details what distro and Wayland compositor you are using.

## Installation
### AUR (Arch Linux)

``` bash
yay -S kwybars-bin

# or build the latest git commit from source
yay -S kwybars-git
```

Start the daemon after install:
``` bash
kwybars-daemon
```

(Optional) Run with a specific config file:
```bash
kwybars-daemon --config ~/.config/kwybars/custom/my_config.toml
```

Start the daemon on boot (Hyprland):
```sh
# Recommended (if you use UWSM):
exec-once = uwsm app -- kwybars-daemon

# If you are not using UWSM
exec-once = kwybars-daemon
```

If you prefer `systemd` service:
```bash
systemctl --user enable --now kwybars-daemon.service
```

## Install from source
Install dependencies:

``` bash
sudo pacman -S --needed rust gtk4 gtk4-layer-shell pipewire cava
# optional: desktop error notifications
sudo pacman -S --needed libnotify
```

### Build and run

``` bash
cargo build --workspace
cargo run -p kwybars-overlay
```

Run daemon mode (auto launch on audio):

``` bash
cargo run -p kwybars-daemon
```

Run with a specific config path:

```bash
cargo run -p kwybars-daemon -- --config ~/.config/kwybars/custom/my_config.toml
```

## Configuration

Kwybars looks for config files in this order:

- `--config /path/to/config.toml`
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
- `~/.config/kwybars/themes/<theme>.toml` (or next to your active `KWYBARS_CONFIG`)
- `/usr/share/kwybars/themes/<theme>.toml` (installed package themes)
- `<cwd>/assets/themes/<theme>.toml` (source checkout fallback)

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
height = 500
anchor_margin = 20
margin_left = 20
margin_right = 20

[visualizer]
layout = "line"
bar_corner_radius = 20
segmented_bars = false
segment_length = 12
segment_gap = 6
radial_inner_radius = 180
radial_start_angle = -90
radial_arc_degrees = 360
radial_rotation_speed = 0
center_offset_x = 0
center_offset_y = 0
polygon_sides = 3
polygon_radius = 220
polygon_rotation = -90
polygon_rotation_speed = 0
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
- `layout`: layout mode: `line|radial|polygon`.
- `bars`: number of bars.
- `bar_width`: base bar thickness in pixels.
- `bar_corner_radius`: bar corner radius in pixels (`0` = square bars).
- `segmented_bars`: split each bar into repeated segments (`true|false`).
- `segment_length`: segment size in pixels along bar growth direction.
- `segment_gap`: empty spacing in pixels between segments.
- `radial_inner_radius`: inner circle radius in pixels for `layout="radial"`.
- `radial_start_angle`: arc start angle in degrees for `layout="radial"` (`-90` starts at the top).
- `radial_arc_degrees`: arc span in degrees for `layout="radial"` (`360` = full ring, `180` = half circle).
- `radial_rotation_speed`: rotation speed in degrees per second for `layout="radial"` (`0` = static, negative reverses direction).
- `center_offset_x`: horizontal center offset in pixels for centered layouts (`radial` and `polygon`), positive moves right.
- `center_offset_y`: vertical center offset in pixels for centered layouts (`radial` and `polygon`), positive moves down.
- `polygon_sides`: number of polygon sides for `layout="polygon"` (`3` = triangle, `4` = square, `6` = hexagon).
- `polygon_radius`: outer polygon radius in pixels for `layout="polygon"`.
- `polygon_rotation`: polygon rotation in degrees for `layout="polygon"` (`-90` points a triangle upward).
- `polygon_rotation_speed`: polygon rotation speed in degrees per second for `layout="polygon"` (`0` = static, negative reverses direction).
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
center_offset_x = 0
center_offset_y = 0
```

Example triangle/polygon layout:

```toml
[visualizer]
layout = "polygon"
polygon_sides = 3
polygon_radius = 220
polygon_rotation = -90
polygon_rotation_speed = 18
center_offset_x = 0
center_offset_y = 0
```

Example square layout:

```toml
[visualizer]
layout = "polygon"
polygon_sides = 4
polygon_radius = 220
polygon_rotation = 0
polygon_rotation_speed = 0
center_offset_x = 0
center_offset_y = 0
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

## Fast Config Switching

`kwybarsctl switch-config` lets you swap the active config file atomically so a running
`kwybars-daemon` or `kwybars-overlay` can reload it without a restart.

Default behavior:
- updates the normal active config path (`~/.config/kwybars/config.toml` or your XDG equivalent)
- replaces that path with a symlink to the selected config
- creates a one-time backup of an existing regular `config.toml` as `config.toml.bak`

Examples:

```bash
kwybarsctl switch-config ~/.config/kwybars/custom/my_radial_config.toml
kwybarsctl switch-config ~/.config/kwybars/custom/my_line_top_config.toml
```

If your overlay/daemon is watching a different active path, use `--active`:

```bash
kwybarsctl switch-config --active ~/.config/kwybars/current.toml ~/.config/kwybars/custom/config3.toml
kwybars-daemon --config ~/.config/kwybars/current.toml
```

Notes:
- The daemon/overlay and `kwybarsctl --active` must use the exact same active path.
- If the switched config changes `[daemon].overlay_command` or `overlay_args`, the daemon restarts the overlay once so the new command takes effect.

Recommended workflow:
- keep one stable active file such as `~/.config/kwybars/current.toml`
- put your real presets in `~/.config/kwybars/custom/*.toml`
- start the daemon against the stable active file
- switch presets by repointing that active file with `kwybarsctl`

Example:

```bash
kwybars-daemon --config ~/.config/kwybars/current.toml
kwybarsctl switch-config --active ~/.config/kwybars/current.toml ~/.config/kwybars/custom/my_radial_config.toml
kwybarsctl switch-config --active ~/.config/kwybars/current.toml ~/.config/kwybars/custom/my_line_top_config.toml
kwybarsctl switch-config --active ~/.config/kwybars/current.toml ~/.config/kwybars/custom/my_segmented_config.toml
```

Example Hyprland binds:

```ini
bind = SUPER ALT, 1, exec, kwybarsctl switch-config --active ~/.config/kwybars/current.toml ~/.config/kwybars/custom/my_radial_config.toml
bind = SUPER ALT, 2, exec, kwybarsctl switch-config --active ~/.config/kwybars/current.toml ~/.config/kwybars/custom/my_line_top_config.toml
bind = SUPER ALT, 3, exec, kwybarsctl switch-config --active ~/.config/kwybars/current.toml ~/.config/kwybars/custom/my_segmented_config.toml
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
