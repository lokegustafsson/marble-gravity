{ system, rust-toolchain, cargo2nix, crane, nixpkgs, pkgs, lib }:
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

  webpage = let
    craneLib = (crane.mkLib pkgs).overrideToolchain rust-toolchain;
    mainWasm = craneLib.buildPackage {
      src = lib.cleanSourceWith {
        src = ./.;
        filter = path: type:
          (craneLib.filterCargoSources path type
            || (builtins.match ".*/assets/.*\\.ttf$" path) != null
            || (builtins.match ".*/assets/skybox/.*\\.png$" path) != null
            || (builtins.match ".*/src/.*\\.(frag|vert|wgsl)$" path) != null);
      };
      cargoLock = ./Cargo.lock;
      cargoToml = ./crates/marble-gravity/Cargo.toml;
      cargoExtraArgs =
        "--package marble-gravity --target wasm32-unknown-unknown";
      doCheck = false;
      buildInputs = [ ];
    };
    workerWasm = craneLib.buildPackage {
      src = lib.cleanSourceWith {
        src = ./.;
        filter = craneLib.filterCargoSources;
      };
      cargoLock = ./Cargo.lock;
      cargoToml = ./crates/worker/Cargo.toml;
      cargoExtraArgs =
        "--package worker --target wasm32-unknown-unknown --features inner";
      doCheck = false;
      buildInputs = [ ];
    };
  in derivation {
    name = "marble-gravity";
    builder = "${pkgs.bash}/bin/bash";
    inherit system mainWasm workerWasm;
    args = [
      "-c"
      ''
        export PATH="$coreutils/bin:$wasmbindgen/bin"
        wasm-bindgen --target web $mainWasm/lib/marble_gravity.wasm \
          --no-typescript --out-dir $out/
        wasm-bindgen --target web $workerWasm/lib/worker.wasm \
          --no-typescript --out-dir $out/
        cp $indexhtml $out/index.html
        cp $computejs $out/compute.js
        cp $workermainjs $out/workermain.js
        cp $polyfill $out/module-workers-polyfill.js
      ''
    ];
    indexhtml = ./assets/index.html;
    computejs = ./assets/compute.js;
    workermainjs = ./assets/workermain.js;
    polyfill = ./assets/module-workers-polyfill.js;
    coreutils = pkgs.coreutils;
    wasmbindgen = wasm-bindgen;
  };
in { inherit webpage wasm-bindgen; }
