{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-20.03";

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
    in {
      defaultPackage.${system} = pkgs.mkShell {
        name = "nixshell";
        buildInputs = with pkgs; [
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
    };
}
