{
  inputs = {
    systems.url = "github:nix-systems/default";
    flake-utils = {
      url = "github:numtide/flake-utils";
      inputs.systems.follows = "systems";
    };
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.flake-utils.follows = "flake-utils";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixGL = {
      url = "github:nix-community/nixGL";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs:
    inputs.flake-utils.lib.eachSystem
    [ inputs.flake-utils.lib.system.x86_64-linux ] (system:
      let
        pkgs = import inputs.nixpkgs {
          inherit system;
          overlays =
            [ inputs.rust-overlay.overlays.default inputs.nixGL.overlay ];
        };
        lib = inputs.nixpkgs.lib;
        cargoNix = import ./Cargo.nix { inherit pkgs; };
        marble-gravity = cargoNix.workspaceMembers.marble-gravity.build;

        rust-toolchain = pkgs.rust-bin.nightly."2024-04-11".default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
          targets = [ "wasm32-unknown-unknown" "x86_64-unknown-linux-gnu" ];
        };
        wasm = import ./wasm.nix {
          inherit (inputs) nixpkgs crane;
          inherit system rust-toolchain pkgs lib;
        };
      in {
        formatter = pkgs.writeShellApplication {
          name = "format";
          runtimeInputs = [ pkgs.rust-bin.stable.latest.default pkgs.nixfmt ];
          text = ''
            set -v
            cargo fmt
            find . -name '*.nix' | grep -v Cargo.nix | xargs nixfmt'';
        };

        devShells.default = pkgs.mkShell {
          packages = let p = pkgs;
          in [
            p.cargo-flamegraph
            p.cargo-outdated
            p.cmake
            p.crate2nix
            p.fontconfig
            p.fontforge-gtk
            p.nixgl.nixGLIntel
            p.pkg-config
            rust-toolchain
            wasm.wasm-bindgen
            (p.writeShellScriptBin "cargo-udeps" ''
              export RUSTC="${rust-toolchain}/bin/rustc"
              export CARGO="${rust-toolchain}/bin/cargo"
              exec "${p.cargo-udeps}/bin/cargo-udeps" "$@"
            '')
          ];
          PKG_CONFIG_PATH_FOR_TARGET =
            "${pkgs.fontconfig.dev}/lib/pkgconfig:${pkgs.freetype.dev}/lib/pkgconfig";
          shellHook = ''
            git rev-parse --is-inside-work-tree > /dev/null && [ -n "$CARGO_TARGET_DIR_PREFIX" ] && \
            export CARGO_TARGET_DIR="$CARGO_TARGET_DIR_PREFIX$(git rev-parse --show-toplevel)"
            exec nixGLIntel zsh
          '';
        };

        packages = rec {
          default = marbleNixGLIntel;
          marbleNixGLIntel =
            pkgs.writeShellScriptBin "marble-gravity-nixGLIntel" ''
              exec ${lib.getExe pkgs.nixgl.nixGLIntel} ${
                lib.getExe marble-gravity
              }
            '';
          inherit marble-gravity;
          webpage = wasm.webpage;
        };
        apps = rec {
          serve = {
            type = "app";
            program = let
              script = pkgs.writeShellScript "serve" ''
                python serve.py ${wasm.webpage}
              '';
            in "${script}";
          };
          default = serve;
        };
      });
}
