{
  description = "kwybars — GTK4 real-time audio visualizer overlay for Wayland";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust toolchain
            cargo
            rustc
            # Required system libraries
            gdk-pixbuf
            gtk4
            gtk4-layer-shell
            pipewire
            cava
            libnotify
            # pkg-config so cargo's build scripts can find libraries
            pkg-config
          ];
        };
      });
}
