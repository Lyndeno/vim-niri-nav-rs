{
  description = "vim-niri-nav — navigate niri windows and vim splits with the same keybindings";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    crane,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};
      craneLib = crane.mkLib pkgs;

      src = craneLib.cleanCargoSource ./rpc;

      common-args = {
        inherit src;
        strictDeps = true;
      };

      cargoArtifacts = craneLib.buildDepsOnly common-args;

      vim-niri-nav = craneLib.buildPackage (common-args
        // {
          inherit cargoArtifacts;
        });

      plugin = pkgs.vimUtils.buildVimPlugin {
        pname = "vim-niri-nav";
        version = "unstable";
        src = ./.;
        meta = {
          homepage = "https://github.com/lyndeno/vim-niri-nav";
          hydraPlatforms = [];
        };
      };
    in {
      packages = {
        inherit vim-niri-nav plugin;
        default = vim-niri-nav;
      };

      devShells.default = craneLib.devShell {
        packages = with pkgs; [
          rust-analyzer
          clippy
          rustfmt
          jq
        ];
      };
    });
}
