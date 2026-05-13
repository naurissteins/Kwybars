{
  description = "Kwybars — GTK4 real-time audio visualizer overlay for Wayland";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
      pkgsFor = system: import nixpkgs { inherit system; };
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
          workspaceVersion = (fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version;
        in
        rec {
          kwybars = pkgs.rustPlatform.buildRustPackage {
            pname = "kwybars";
            version = workspaceVersion;

            src = pkgs.lib.fileset.toSource {
              root = ./.;
              fileset = pkgs.lib.fileset.unions [
                ./Cargo.toml
                ./Cargo.lock
                ./assets/examples
                ./assets/themes
                ./crates
              ];
            };

            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            cargoBuildFlags = [ "--workspace" ];
            cargoCheckFlags = [ "--workspace" ];

            nativeBuildInputs = with pkgs; [
              makeWrapper
              pkg-config
            ];

            buildInputs = with pkgs; [
              gdk-pixbuf
              gtk4
              gtk4-layer-shell
              libnotify
              pipewire
            ];

            installPhase = ''
              runHook preInstall

              install_binary() {
                name="$1"
                binary="$(find target -type f -path "*/release/$name" -print -quit)"

                if [ -z "$binary" ]; then
                  echo "failed to find release binary: $name" >&2
                  find target -maxdepth 4 -type f -perm -0100 -print >&2
                  exit 1
                fi

                install -Dm755 "$binary" "$out/bin/$name"
              }

              install_binary kwybars-daemon
              install_binary kwybars-overlay
              install_binary kwybarsctl

              install -Dm644 assets/examples/*.toml -t "$out/share/kwybars/examples"
              install -Dm644 assets/themes/*.toml -t "$out/share/kwybars/themes"

              wrapProgram "$out/bin/kwybars-daemon" \
                --set KWYBARS_THEMES_DIR "$out/share/kwybars/themes" \
                --prefix PATH : ${
                  pkgs.lib.makeBinPath [
                    pkgs.cava
                    pkgs.libnotify
                  ]
                }:$out/bin

              wrapProgram "$out/bin/kwybars-overlay" \
                --set KWYBARS_THEMES_DIR "$out/share/kwybars/themes" \
                --prefix PATH : ${
                  pkgs.lib.makeBinPath [
                    pkgs.cava
                    pkgs.libnotify
                  ]
                }:$out/bin

              wrapProgram "$out/bin/kwybarsctl" \
                --set KWYBARS_THEMES_DIR "$out/share/kwybars/themes"

              runHook postInstall
            '';

            meta = {
              description = "GTK4 real-time audio visualizer overlay for Wayland";
              homepage = "https://github.com/naurissteins/Kwybars";
              license = pkgs.lib.licenses.gpl3Plus;
              mainProgram = "kwybars-daemon";
              platforms = pkgs.lib.platforms.linux;
            };
          };

          default = kwybars;
        }
      );

      nixosModules.default =
        {
          config,
          lib,
          pkgs,
          ...
        }:
        let
          cfg = config.programs.kwybars;
          package = cfg.package;

          resolvedPath =
            if cfg.settings != { } then
              (pkgs.formats.toml { }).generate "kwybars.toml" cfg.settings

            else if cfg.preset != null then
              "${package}/share/kwybars/examples/${cfg.preset}.toml"
            else
              cfg.configPath; # may be null → no --config flag

          activeSources = lib.count lib.id [
            (cfg.settings != { })
            (cfg.configPath != null)
            (cfg.preset != null)
          ];
        in
        {
          options.programs.kwybars = {
            enable = lib.mkEnableOption "Kwybars audio visualizer overlay";

            package = lib.mkOption {
              type = lib.types.package;
              default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
              defaultText = lib.literalExpression "inputs.kwybars.packages.${pkgs.stdenv.hostPlatform.system}.default";
              description = "Kwybars package to install.";
            };

            settings = lib.mkOption {
              type = lib.types.nullOr (pkgs.formats.toml { }).type;
              default = { };
              example = lib.literalExpression ''
                {
                	overlay.position 	= "bottom"
                	overlay.width 		= 800
                	overlay.height 		= 500
                }
              '';
              description = ''
                Kwybars configuration as a Nix attribute set, serialised to TOML
                and passed via --config. Mutually exclusive with
                <option>configPath</option> and <option>preset</option>.
              '';
            };

            configPath = lib.mkOption {
              type = lib.types.nullOr (
                lib.types.oneOf [
                  lib.types.path
                  lib.types.str
                ]
              );
              default = null;
              example = lib.literalExpression ''"/home/alice/.config/kwybars/current.toml"'';
              description = ''
                Path to an existing TOML config file passed via --config.
                Mutually exclusive with <option>settings</option> and
                <option>preset</option>.
              '';
            };

            preset = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
              example = lib.literalExpression ''"config"'';
              description = ''
                Name of a bundled example config (without the .toml extension)
                shipped under <literal>''${package}/share/kwybars/examples/</literal>.
                For example, <literal>"config"</literal> resolves to
                <literal>''${package}/share/kwybars/examples/config.toml</literal>.
                Mutually exclusive with <option>settings</option> and
                <option>configPath</option>.
              '';
            };

            extraArgs = lib.mkOption {
              type = lib.types.listOf lib.types.str;
              default = [ ];
              example = [ "--verbose" ];
              description = "Extra command-line arguments to pass to the kwybars-daemon executable.";
            };

            systemd.enable = lib.mkOption {
              type = lib.types.bool;
              default = false;
              description = "Create and enable a user systemd service for kwybars-daemon.";
            };
          };

          config = lib.mkIf cfg.enable {
            assertions = [
              {
                assertion = activeSources <= 1;
                message = ''
                  programs.kwybars: at most one of `settings`, `configPath`, or `preset`
                  may be set at a time (${toString activeSources} are currently set).
                '';
              }
              {
                assertion =
                  cfg.preset == null || builtins.pathExists "${package}/share/kwybars/examples/${cfg.preset}.toml";
                message = ''
                  programs.kwybars.preset: "${cfg.preset}.toml" was not found in
                  ${package}/share/kwybars/examples/.
                '';
              }
            ];

            environment.systemPackages = [ package ];

            systemd.user.services.kwybars-daemon = lib.mkIf cfg.systemd.enable {
              description = "Kwybars audio visualizer daemon";
              after = [ "graphical-session.target" ];
              partOf = [ "graphical-session.target" ];
              wantedBy = [ "default.target" ];

              environment = lib.mkIf (resolvedPath != null) {
                KWYBARS_CONFIG = toString resolvedPath;
              };

              serviceConfig = {
                Type = "simple";
                ExecStart = "${package}/bin/kwybars-daemon ${lib.escapeShellArgs cfg.extraArgs}";
                Restart = "on-failure";
                RestartSec = 2;
              };
            };
          };
        };

      formatter = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
        in
        pkgs.nixfmt
      );

      apps = forAllSystems (
        system:
        let
          package = self.packages.${system}.kwybars;
        in
        {
          kwybars-daemon = {
            type = "app";
            program = "${package}/bin/kwybars-daemon";
          };

          kwybars-overlay = {
            type = "app";
            program = "${package}/bin/kwybars-overlay";
          };

          kwybarsctl = {
            type = "app";
            program = "${package}/bin/kwybarsctl";
          };

          default = self.apps.${system}.kwybars-daemon;
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              cargo
              cava
              clippy
              gdk-pixbuf
              gtk4
              gtk4-layer-shell
              libnotify
              pipewire
              pkg-config
              rustc
              rustfmt
            ];
          };
        }
      );
    };
}
