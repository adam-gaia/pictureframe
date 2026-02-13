{
  flake,
  inputs,
  pkgs,
  ...
}: let
  crateBuilder = inputs.self.lib.mkCrateBuilder pkgs;
  commonArgs = crateBuilder.commonArgs;
  craneLib = crateBuilder.craneLib;

  mkWasm = {
    crateName,
    dirName,
    publicUrl,
    distName,
  }: let
    wasmArgs =
      commonArgs
      // {
        pname = "trunk-wasm-${crateName}";
        cargoExtraArgs = "--package=${crateName}";
        CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
      };

    cargoArtifactsWasm = craneLib.buildDepsOnly (
      wasmArgs
      // {
        doCheck = false;
      }
    );
  in
    craneLib.buildTrunkPackage (
      wasmArgs
      // {
        pname = crateName;
        cargoArtifacts = cargoArtifactsWasm;
        trunkExtraBuildArgs = "--dist ./dist/${distName} --public-url ${publicUrl}";

        preBuild = ''
          cd ./crates/${dirName}
        '';
        postBuild = ''
          mv ./dist ../../
          cd ../../
        '';
        # The version of wasm-bindgen-cli here must match the one from Cargo.lock.
        # When updating to a new version replace the hash values with lib.fakeHash,
        # then try to do a build, which will fail but will print out the correct value
        # for `hash`. Replace the value and then repeat the process but this time the
        # printed value will be for the second `hash` below
        wasm-bindgen-cli = pkgs.wasm-bindgen-cli_0_2_108;
      }
    );

  frontend-viewer = mkWasm {
    crateName = "pictureframe-frontend-viewer";
    dirName = "frontend-viewer";
    publicUrl = "/";
    distName = "viewer";
  };

  frontend-admin = mkWasm {
    crateName = "pictureframe-frontend-admin";
    dirName = "frontend-admin";
    publicUrl = "/admin";
    distName = "admin";
  };
in
  pkgs.symlinkJoin {
    name = "pictureframe-frontend";
    paths = [
      frontend-viewer
      frontend-admin
    ];
  }
