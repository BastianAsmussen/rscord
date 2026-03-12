{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs = inputs @ {flake-parts, ...}:
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux"];
      perSystem = {pkgs, ...}: {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            pkg-config
            wrapGAppsHook4
            cargo
            cargo-tauri
            nodejs
            at-spi2-atk.dev
            gtk3.dev
            webkitgtk_4_1.dev
          ];

          buildInputs = with pkgs; [
            cargo
            rustc
            rustfmt
            clippy
            rust-analyzer

            librsvg
            webkitgtk_4_1
          ];

          env.RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
          shellHook = ''
            export PATH="''${CARGO_HOME:-~/.cargo}/bin":"$PATH"
            export PATH="''${RUSTUP_HOME:-~/.rustup}/toolchains/$RUSTC_VERSION-${pkgs.stdenv.hostPlatform.rust.rustcTarget}/bin":"$PATH"

            # Needed on Wayland to report the correct display scale.
            export XDG_DATA_DIRS="$GSETTINGS_SCHEMAS_PATH"
          '';
        };
      };
    };
}
