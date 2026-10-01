<h1 align=center>kwybars</h1>

<div align=center>

![GitHub last commit](https://img.shields.io/github/last-commit/naurissteins/kwybars?style=for-the-badge&labelColor=181825&color=a6e3a1)
![GitHub repo size](https://img.shields.io/github/repo-size/naurissteins/kwybars?style=for-the-badge&labelColor=181825&color=d3bfe6)
![AUR Version](https://img.shields.io/aur/version/kwybars-bin?style=for-the-badge&labelColor=181825&color=b4befe)
![GitHub Repo stars](https://img.shields.io/github/stars/naurissteins/kwybars?style=for-the-badge&labelColor=181825&color=f9e2af)

</div>

Kwybars is a light desktop audio visualizer for GNU/Linux (Wayland) that renders real-time audio bars on your screen.

Think of it like `cava`... but instead of living in the terminal, it becomes a transparent overlay on your desktop. Place visualizer on any screen edge: top, bottom, left, right or center and watch your music bounce in real time. Kwybars is highly customizable with multiple layouts, segmented bars, gradients, themes, and extensive configuration options.

It is a single small binary that reads audio straight from PipeWire. It shows up when something plays, hides when it stops, and does nothing at all in between.

https://github.com/user-attachments/assets/5fe84372-86be-49a8-b9c0-6564e81f1eaa

## 🔥 Features

-   Place visualizer on any screen edge `top | bottom | left | right`
-   Multi-monitor support: show bars on primary, all, or selected monitors
-   Per-monitor overlay and visualizer configuration with `[[overlay.outputs]]`
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
-   Transparent image overlay for Rainmeter-style compositions
-   Smooth fade in/out on audio activity
-   Hot reload config changes (no restart needed)
-   Built-in themes and optional custom theme palettes (`~/.config/kwybars/themes/*.toml`)
-   Shows and hides itself with audio activity, with no work done while silent
-   Fractional scaling and monitor hotplug
-   Built-in checks: `kwybars doctor`, `kwybars validate-config`

<div align=center>

  [Documentation](https://naurissteins.com/kwybars)
  
</div>

## Requirements

-   A Wayland compositor with layer-shell support (Hyprland, Sway, niri, river, KDE Plasma, ...)
-   PipeWire

## Installation
### AUR (Arch Linux)

``` bash
yay -S kwybars-bin

# or build the latest git commit from source
yay -S kwybars-git
```

Run it:
``` bash
kwybars
```

Start it with your session (Hyprland):
```sh
# Recommended (if you use UWSM):
exec-once = uwsm app -- kwybars

# If you are not using UWSM
exec-once = kwybars
```

If you prefer a `systemd` user service:
```bash
systemctl --user enable --now kwybars.service
```

Nothing on screen? `kwybars doctor` checks the config, compositor, monitors, and PipeWire.

### NixOS

Import module:

```nix
{
  inputs.kwybars.url = "github:naurissteins/Kwybars";

  outputs = { nixpkgs, kwybars, ... }: {
    nixosConfigurations.my-host = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        kwybars.nixosModules.default
        {
          programs.kwybars.enable = true;
        }
      ];
    };
  };
}
```

The module installs `kwybars`. Start it from your compositor config (`exec-once = kwybars`) or in a terminal.

Or enable the user service (this is optional):

```nix
{
  programs.kwybars = {
    enable = true;
    systemd.enable = true;

    # Optional. Useful with `kwybars switch-config`.
    # configPath = "/home/your-user/.config/kwybars/current.toml";
  };
}
```

Install the package directly:

```bash
nix profile install github:naurissteins/Kwybars#kwybars
```

Or install it directly in a NixOS system config:

```nix
{
  environment.systemPackages = [
    inputs.kwybars.packages.${pkgs.system}.default
  ];
}
```

### Nix Flake

```bash
nix build github:naurissteins/Kwybars
./result/bin/kwybars

# or run the flake app directly
nix run github:naurissteins/Kwybars
```

### Other distributions

Debian/Ubuntu and Fedora packages, and building from source, are covered in the [documentation](https://naurissteins.com/kwybars).

## Upgrading from 0.2

`kwybars-daemon`, `kwybars-overlay`, and `kwybarsctl` are now the single `kwybars` command, and Cava and GTK are no longer needed. Existing configs keep working. See the upgrade page in the [documentation](https://naurissteins.com/kwybars).

## Docs

Full installation, configuration, preset configs and usage docs:

https://naurissteins.com/kwybars
