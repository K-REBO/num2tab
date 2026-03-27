{
  description = "num2tab WASM build environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f system);
    in {
      devShells = forAllSystems (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };
          rustToolchain = pkgs.rust-bin.stable.latest.default.override {
            targets = [ "wasm32-unknown-unknown" ];
          };
        in {
          default = pkgs.mkShell {
            packages = [
              rustToolchain
              pkgs.wasm-pack
              pkgs.binaryen  # provides wasm-opt for wasm-pack optimization
            ];
            shellHook = ''
              echo "num2tab WASM build environment"
              echo "  Run: ./www/build.sh"
            '';
          };
        }
      );
    };
}
