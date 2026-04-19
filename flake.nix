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
        packages.kwybars = pkgs.rustPlatform.buildRustPackage rec {
            pname = "kwybars";
            version = "0.1.6";

            src = pkgs.fetchFromGitHub {
                owner = "naurissteins";
                repo = "Kwybars";
                rev = version;
                hash = "sha256-NAy8dA5iGNBtdoZ38LC72yStCQ9whGgT+rajyHYeZkA=";
            };

            cargoHash = "sha256-3wehddHWMy83NUTtAmTtbLS3k8jf75HI09qAHEQ5adc=";

            buildInputs = with pkgs; [
                cargo
                rustc
                gtk4
                gtk4-layer-shell
                pipewire
                cava
                libnotify
            ];

            nativeBuildInputs = with pkgs; [
                pkg-config
                makeWrapper
            ];

            postInstall = ''
                wrapProgram $out/bin/kwybars-daemon --prefix PATH : ${pkgs.cava}/bin:${pkgs.libnotify}/bin:$out/bin
                wrapProgram $out/bin/kwybars-overlay --prefix PATH : ${pkgs.cava}/bin:${pkgs.libnotify}/bin:$out/bin
            '';
        };
        defaultPackage = self.packages.${system}.kwybars;
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
