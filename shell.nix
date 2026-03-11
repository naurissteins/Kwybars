{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    # Rust toolchain
    cargo
    rustc

    # Required system libraries
    gtk4
    gtk4-layer-shell
    pipewire
    cava
    libnotify  # optional: desktop error notifications

    # pkg-config so cargo's build scripts can find libraries
    pkg-config
  ];
}
