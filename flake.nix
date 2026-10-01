{
  description = "Kwybars — real-time audio visualizer overlay for Wayland";

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
                ./assets/systemd
                ./assets/themes
                ./src
                ./tests
                ./docs/man
              ];
            };

            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            nativeBuildInputs = with pkgs; [
              pkg-config
              # libclang for the pipewire bindings
              rustPlatform.bindgenHook
            ];

            buildInputs = with pkgs; [
              pipewire
            ];

            postInstall = ''
              install -Dm644 assets/examples/*.toml -t "$out/share/kwybars/examples"
              install -Dm644 assets/themes/*.toml -t "$out/share/kwybars/themes"
              install -Dm644 docs/man/*.1 -t "$out/share/man/man1"
              install -Dm644 assets/systemd/kwybars.service -t "$out/lib/systemd/user"
              substituteInPlace "$out/lib/systemd/user/kwybars.service" \
                --replace-fail "ExecStart=/usr/bin/env kwybars" "ExecStart=$out/bin/kwybars"
            '';

            meta = {
              description = "Real-time audio visualizer overlay for Wayland";
              homepage = "https://github.com/naurissteins/Kwybars";
              license = pkgs.lib.licenses.gpl3Plus;
              mainProgram = "kwybars";
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
                  overlay.position = "bottom";
                  overlay.height = 500;
                  visualizer.layout = "wave";
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
              description = "Extra command-line arguments to pass to the kwybars executable.";
            };

            systemd.enable = lib.mkOption {
              type = lib.types.bool;
              default = false;
              description = "Create and enable a user systemd service for kwybars.";
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

            systemd.user.services.kwybars = lib.mkIf cfg.systemd.enable {
              description = "Kwybars audio visualizer overlay";
              after = [ "graphical-session.target" ];
              partOf = [ "graphical-session.target" ];
              wantedBy = [ "graphical-session.target" ];
              unitConfig.ConditionEnvironment = "WAYLAND_DISPLAY";

              environment = lib.mkIf (resolvedPath != null) {
                KWYBARS_CONFIG = toString resolvedPath;
              };

              serviceConfig = {
                ExecStart = "${package}/bin/kwybars ${lib.escapeShellArgs cfg.extraArgs}";
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
          kwybars = {
            type = "app";
            program = "${package}/bin/kwybars";
          };

          default = self.apps.${system}.kwybars;
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
              clippy
              pipewire
              pkg-config
              rustPlatform.bindgenHook
              rustc
              rustfmt
            ];
          };
        }
      );
    };
}
