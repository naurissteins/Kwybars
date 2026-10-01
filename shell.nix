{
  pkgs ? import <nixpkgs> { },
}:

pkgs.mkShell {
  buildInputs = with pkgs; [
    # Rust toolchain
    cargo
    rustc

    # Required system libraries
    pipewire

    # pkg-config so cargo's build scripts can find libraries
    pkg-config
    # libclang for the pipewire bindings
    rustPlatform.bindgenHook
  ];
}
