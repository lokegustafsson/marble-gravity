{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
    cargo2nix = {
      url = "github:cargo2nix/cargo2nix";
      inputs.rust-overlay.follows = "rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, cargo2nix }:
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
      in {
        devShells.default = rust.rustPkgs.workspaceShell {
          packages = let p = pkgs;
          in [
            cargo2nix.outputs.packages.${system}.cargo2nix
            p.cargo-outdated
            p.rust-bin.stable.latest.clippy
            p.rust-bin.stable.latest.default
          ]; # ++ builtins.attrValues rust.packages;
        };

        packages = rust.packages // { default = rust.packages.marble-gravity; };
      });
}
