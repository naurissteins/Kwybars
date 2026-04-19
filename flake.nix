{
  description = "Kwybars — GTK4 real-time audio visualizer overlay for Wayland";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        workspaceVersion =
          (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version;
      in {
        packages = rec {
          kwybars = pkgs.rustPlatform.buildRustPackage {
            pname = "kwybars";
            version = workspaceVersion;

            src = pkgs.lib.cleanSource ./.;
            cargoLock.lockFile = ./Cargo.lock;

            cargoBuildFlags = [ "--workspace" ];
            cargoCheckFlags = [ "--workspace" ];

            nativeBuildInputs = with pkgs; [
              pkg-config
              makeWrapper
            ];

            buildInputs = with pkgs; [
              gdk-pixbuf
              gtk4
              gtk4-layer-shell
              pipewire
              libnotify
            ];

            installPhase = ''
              runHook preInstall

              install -Dm755 target/release/kwybars-daemon $out/bin/kwybars-daemon
              install -Dm755 target/release/kwybars-overlay $out/bin/kwybars-overlay
              install -Dm755 target/release/kwybarsctl $out/bin/kwybarsctl

              install -Dm644 assets/examples/config.toml $out/share/kwybars/examples/config.toml
              install -Dm644 assets/systemd/kwybars-daemon.service \
                $out/lib/systemd/user/kwybars-daemon.service
              install -Dm644 assets/themes/*.toml -t $out/share/kwybars/themes

              wrapProgram $out/bin/kwybars-daemon \
                --set KWYBARS_THEMES_DIR $out/share/kwybars/themes \
                --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.cava pkgs.libnotify ]}:$out/bin
              wrapProgram $out/bin/kwybars-overlay \
                --set KWYBARS_THEMES_DIR $out/share/kwybars/themes \
                --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.cava pkgs.libnotify ]}:$out/bin
              wrapProgram $out/bin/kwybarsctl \
                --set KWYBARS_THEMES_DIR $out/share/kwybars/themes

              runHook postInstall
            '';

            meta = {
              description = "GTK4 real-time audio visualizer overlay for Wayland";
              homepage = "https://github.com/naurissteins/Kwybars";
              license = pkgs.lib.licenses.gpl3Plus;
              platforms = pkgs.lib.platforms.linux;
              mainProgram = "kwybars-daemon";
            };
          };

          default = kwybars;
        };

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
