{
  inputs = {
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.flake-utils.follows = "flake-utils";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    cargo2nix = {
      url = "github:cargo2nix/cargo2nix";
      inputs.flake-compat.follows = "flake-compat";
      inputs.flake-utils.follows = "flake-utils";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-overlay.follows = "rust-overlay";
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs.flake-compat.follows = "flake-compat";
      inputs.flake-utils.follows = "flake-utils";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-overlay.follows = "rust-overlay";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, cargo2nix, crane, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ cargo2nix.overlays.default ];
          config.allowUnfree = true;
        };
        lib = nixpkgs.lib;
        rust = import ./rust.nix {
          inherit lib pkgs;
          workspace-binaries = {
            marble-gravity = {
              rpath = p: [ ];
              run_time_ld_library_path = p: [
                # xorg
                p.xorg.libX11
                p.xorg.libXcursor
                p.xorg.libXi
                p.xorg.libXrandr
                # vulkan
                p.vulkan-loader
                # opengl
                p.libglvnd
              ];
            };
          };
          extra-overrides = { mkNativeDep, mkEnvDep, p }: [
            (mkNativeDep "x11" [ p.pkg-config p.xorg.libX11 ])
            (mkNativeDep "shaderc-sys" [ p.cmake p.git ])
            (mkNativeDep "freetype-sys" [ p.cmake ])
            (mkNativeDep "expat-sys" [ p.cmake ])
            (mkNativeDep "servo-fontconfig-sys" [ p.pkg-config ])
            (mkEnvDep "servo-fontconfig-sys" {
              PKG_CONFIG_PATH_FOR_TARGET =
                "${pkgs.fontconfig.dev}/lib/pkgconfig:${pkgs.freetype.dev}/lib/pkgconfig";
            })
          ];
        };
        rust-toolchain = (pkgs.rust-bin.nightly."2022-12-08".default.override {
          extensions = [ "rust-src" ];
          targets = [ "wasm32-unknown-unknown" "x86_64-unknown-linux-gnu" ];
        });
        wasm = import ./wasm.nix {
          inherit system rust-toolchain cargo2nix crane nixpkgs pkgs lib;
        };
      in {
        devShells.default = rust.rustPkgs.workspaceShell {
          packages = let p = pkgs;
          in [
            rust-toolchain
            cargo2nix.outputs.packages.${system}.cargo2nix
            p.cargo-flamegraph
            p.cargo-outdated
            p.fontforge-gtk
            p.rust-bin.stable.latest.clippy
            wasm.wasm-bindgen
            (p.writeShellScriptBin "cargo-udeps" ''
              export RUSTC="${rust-toolchain}/bin/rustc"
              export CARGO="${rust-toolchain}/bin/cargo"
              exec "${p.cargo-udeps}/bin/cargo-udeps" "$@"
            '')
          ];
        };

        packages = rust.packages // {
          default = rust.packages.marble-gravity;
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
