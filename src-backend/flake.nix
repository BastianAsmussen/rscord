{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

  outputs = {
    self,
    nixpkgs,
    ...
  }: let
    system = "x86_64-linux";
    pkgs = import nixpkgs {inherit system;};
  in {
    devShell.${system} = let
      overrides = builtins.fromTOML (builtins.readFile ./rust-toolchain.toml);
    in
      pkgs.callPackage (
        {
          stdenv,
          mkShell,
          rustup,
          rustPlatform,
        }:
          mkShell {
            strictDeps = true;
            nativeBuildInputs = [
              rustup
              rustPlatform.bindgenHook
            ];

            buildInputs = with pkgs; [postgresql_18.lib];

            RUSTC_VERSION = overrides.toolchain.channel;
            shellHook = ''
              export PATH="''${CARGO_HOME:-~/.cargo}/bin":"$PATH"
              export PATH="''${RUSTUP_HOME:-~/.rustup}/toolchains/$RUSTC_VERSION-${stdenv.hostPlatform.rust.rustcTarget}/bin":"$PATH"
            '';
          }
      ) {};
  };
}
