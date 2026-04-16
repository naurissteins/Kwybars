<h1 align=center>kwybars</h1>

<div align=center>

![GitHub last commit](https://img.shields.io/github/last-commit/naurissteins/kwybars?style=for-the-badge&labelColor=181825&color=a6e3a1)
![GitHub repo size](https://img.shields.io/github/repo-size/naurissteins/kwybars?style=for-the-badge&labelColor=181825&color=d3bfe6)
![AUR Version](https://img.shields.io/aur/version/kwybars-bin?style=for-the-badge&labelColor=181825&color=b4befe)
![GitHub Repo stars](https://img.shields.io/github/stars/naurissteins/kwybars?style=for-the-badge&labelColor=181825&color=f9e2af)

</div>

Kwybars is a GTK4-based desktop audio visualizer for GNU/Linux (Wayland) that renders real-time audio bars on your screen.

Think of it like `cava`... but instead of living in the terminal, it becomes a transparent overlay on your desktop. Place visualizer on any screen edge: top, bottom, left, right or center and watch your music bounce in real time. Kwybars are highly customizable with multiple layouts, segmented bars, gradients, themes, and extensive configuration options.

https://github.com/user-attachments/assets/5fe84372-86be-49a8-b9c0-6564e81f1eaa

## 🔥 Features

-   Place visualizer on any screen edge `top | bottom | left | right`
-   Multi-monitor support: show bars on primary, all, or selected monitors
-   Control window layer: `background`, `bottom`, `top`
-   Custom overlay size + alignment
-   Solid or gradient bar colors
-   Segmented bar style (oldschool split blocks)
-   Mirror layout for centered horizontal or vertical mirrored lines
-   Wave layout for continuous non-bar visualizers
-   Radial layout (circular)
-   Particle layout (pulsating dots)
-   Frame layout for top+bottom, left+right, or all monitor edges at once
-   Polygon layout for triangle, square, hexagon, and similar shapes
-   Optional transparent image overlay for Rainmeter-style compositions
-   Hot reload config changes (no restart needed)
-   Built-in themes and optional custom theme palettes (`~/.config/kwybars/themes/*.toml`)
-   Optional but recommended `kwybars-daemon` that auto starts/stops overlay based on audio activity

> [!NOTE]
> Kwybars are not heavily tested on all Wayland compositors yet. If you encounter issues, please open an issue and provide details what distro and Wayland compositor you are using.

<div align=center>

  [Documentation](https://https://naurissteins.com/veila)
  
</div>

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
sudo pacman -S --needed rust gdk-pixbuf2 gtk4 gtk4-layer-shell pipewire cava
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

## Docs

For full installation, configuration, theming, and usage docs, visit:

https://naurissteins.com/kwybars
