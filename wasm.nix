{ system, cargo2nix, nixpkgs, pkgs }:
let
  wasmPkgs = import nixpkgs {
    inherit system;
    crossSystem = {
      config = "wasm32-unknown-wasi-unknown";
      system = "wasm32-wasi";
      useLLVM = true;
    };
    overlays = [ cargo2nix.overlays.default ];
  };

  wasm-bindgen = pkgs.rustPlatform.buildRustPackage rec {
    pname = "wasm-bindgen-cli";
    version = "0.2.83";
    src = pkgs.fetchCrate {
      inherit pname version;
      sha256 = "sha256-+PWxeRL5MkIfJtfN3/DjaDlqRgBgWZMa6dBt1Q+lpd0=";
    };
    cargoSha256 = "sha256-GwLeA6xLt7I+NzRaqjwVpt1pzRex1/snq30DPv4FR+g=";
    nativeBuildInputs = [ pkgs.pkg-config ];
    buildInputs = [ pkgs.openssl ];
    checkInputs = [ pkgs.nodejs ];
    cargoTestFlags = [ "--test=interface-types" ];
  };

  webpage = derivation {
    name = "marble-gravity-webpage";
    builder = pkgs.bash;
    src = ./index.html;
    args = [
      "-c"
      ''
        export PATH="$coreutils/bin:$wasm-bindgen/bin"
        echo $w2
        echo $wasm
        wasm-bindgen --target web $wasm --no-typescript --out-dir $out
        cp $src $out/
      ''
    ];
    inherit system wasm-bindgen;
    wasm = ((wasmPkgs.rustBuilder.makePackageSet {
      rustVersion = "latest";
      packageFun = import ./Cargo.nix;
      target = "wasm32-wasi";
    }).workspace.marble-gravity { }).out;
    coreutils = pkgs.coreutils;
  };
in { inherit webpage wasm-bindgen; }
