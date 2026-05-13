{
  description = "larry - structured command output tool";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        larry = pkgs.rustPlatform.buildRustPackage {
          pname = "larry";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          meta = with pkgs.lib; {
            description = "Execute commands and capture structured output";
            license = licenses.bsd0;
            mainProgram = "larry";
          };
        };
      in
      {
        packages.default = larry;

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustc
            cargo
          ];
        };

        apps.default = {
          type = "app";
          program = "${larry}/bin/larry";
        };
      }
    );
}
