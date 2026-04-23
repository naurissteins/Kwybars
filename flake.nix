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
          workspaceVersion =
            (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version;
        in
        rec {
          kwybars = pkgs.rustPlatform.buildRustPackage {
            pname = "kwybars";
            version = workspaceVersion;

            src = self;

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

              install -Dm644 assets/examples/config.toml "$out/share/kwybars/examples/config.toml"
              install -Dm644 assets/systemd/kwybars-daemon.service \
                "$out/lib/systemd/user/kwybars-daemon.service"
              install -Dm644 assets/themes/*.toml -t "$out/share/kwybars/themes"

              wrapProgram "$out/bin/kwybars-daemon" \
                --set KWYBARS_THEMES_DIR "$out/share/kwybars/themes" \
                --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.cava pkgs.libnotify ]}:$out/bin

              wrapProgram "$out/bin/kwybars-overlay" \
                --set KWYBARS_THEMES_DIR "$out/share/kwybars/themes" \
                --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.cava pkgs.libnotify ]}:$out/bin

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
          configArg = lib.optionalString (
            cfg.configPath != null
          ) " --config ${lib.escapeShellArg (toString cfg.configPath)}";
        in
        {
          options.programs.kwybars = {
            enable = lib.mkEnableOption "Kwybars audio visualizer overlay";

            package = lib.mkOption {
              type = lib.types.package;
              default = self.packages.${pkgs.system}.default;
              defaultText = lib.literalExpression "inputs.kwybars.packages.${pkgs.system}.default";
              description = "Kwybars package to install.";
            };

            configPath = lib.mkOption {
              type = lib.types.nullOr (lib.types.oneOf [
                lib.types.path
                lib.types.str
              ]);
              default = null;
              example = lib.literalExpression "\"/home/alice/.config/kwybars/current.toml\"";
              description = "Optional config path passed to the packaged user service.";
            };

            systemd.enable = lib.mkOption {
              type = lib.types.bool;
              default = false;
              description = "Create and enable a user systemd service for kwybars-daemon.";
            };
          };

          config = lib.mkIf cfg.enable {
            environment.systemPackages = [ package ];

            systemd.user.services.kwybars-daemon = lib.mkIf cfg.systemd.enable {
              description = "Kwybars audio visualizer daemon";
              after = [ "graphical-session.target" ];
              partOf = [ "graphical-session.target" ];
              wantedBy = [ "default.target" ];

              serviceConfig = {
                Type = "simple";
                ExecStart = "${package}/bin/kwybars-daemon${configArg}";
                Restart = "on-failure";
                RestartSec = 2;
              };
            };
          };
        };

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
