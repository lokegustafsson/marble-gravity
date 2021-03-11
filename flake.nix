{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let pkgs = nixpkgs.legacyPackages.${system};
      in {
        devShell = pkgs.mkShell {
          NIX_SHELL = "marble-gravity nixshell";
          nativeBuildInputs = with pkgs; [
            # x11 crate build dependency
            xorg.libX11
            pkg-config
            # shaderc crate build dependency
            shaderc
            # Run time dependency
            xorg.libXcursor
            xorg.libXrandr
            xorg.libXi
          ];
          SHADERC_LIB_DIR = "${nixpkgs.lib.getLib pkgs.shaderc}/lib";
          LD_LIBRARY_PATH = "${pkgs.vulkan-loader}/lib";
        };
      });
}
