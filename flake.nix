{
  description = "eframe devShell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
      in with pkgs; {
        devShells.default = mkShell rec {
          packages = [
            # Rust
            (rust-bin.stable.latest.default.override {
              extensions = [ "rust-src" "rust-analyzer" ];
              targets = ["wasm32-unknown-unknown"];
            })
            trunk
            cargo-watch
            sqlx-cli
          ];

          buildInputs = [
            # misc. libraries
            openssl
            pkg-config

            # GUI libs
            libxkbcommon
            libGL
            fontconfig

            # wayland libraries
            wayland

            # x11 libraries
            xorg.libXcursor
            xorg.libXrandr
            xorg.libXi
            xorg.libX11

          ];

          LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";

          shellHook = ''
            docker start shajarah-dev || \
              docker run \
              --name shajarah-dev \
              -p 5445:5432 \
              -e POSTGRES_PASSWORD=shajarah-dev \
              -d postgres

            grep DATABASE_URL .env || echo "DATABASE_URL=postgres://postgres:shajarah-dev@localhost:5445/postgres" >> .env

            export $(cat .env)
          '';
        };
      });
}
