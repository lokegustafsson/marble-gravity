{ system, rust-toolchain, crane, nixpkgs, pkgs, lib }:
let
  wasm-bindgen = pkgs.rustPlatform.buildRustPackage rec {
    pname = "wasm-bindgen-cli";
    version = "0.2.92";
    src = pkgs.fetchCrate {
      inherit pname version;
      sha256 = "sha256-1VwY8vQy7soKEgbki4LD+v259751kKxSxmo/gqE6yV0=";
    };
    cargoSha256 = "sha256-aACJ+lYNEU8FFBs158G1/JG8sc6Rq080PeKCMnwdpH0=";
    nativeBuildInputs = [ pkgs.pkg-config ];
    buildInputs = [ pkgs.openssl ];
    checkInputs = [ pkgs.nodejs ];
    doCheck = false;
    #cargoTestFlags = [ "--test=interface-types" ];
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
  in pkgs.runCommand "marble-gravity" {
    inherit mainWasm workerWasm;
    indexhtml = ./assets/index.html;
    computejs = ./assets/compute.js;
    workermainjs = ./assets/workermain.js;
    polyfill = ./assets/module-workers-polyfill.js;
    buildInputs = [ wasm-bindgen ];
  } ''
    set -v
    mkdir -p $out
    wasm-bindgen --target web $mainWasm/lib/marble_gravity.wasm \
      --no-typescript --out-dir $out/
    wasm-bindgen --target web $workerWasm/lib/worker.wasm \
      --no-typescript --out-dir $out/
    cp $indexhtml $out/index.html
    cp $computejs $out/compute.js
    cp $workermainjs $out/workermain.js
    cp $polyfill $out/module-workers-polyfill.js
  '';
in { inherit webpage wasm-bindgen; }
