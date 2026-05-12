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
      inherit (pkgs) lib;
      craneLib = crane.mkLib pkgs;

      src = craneLib.cleanCargoSource ./rpc;

      common-args = {
        inherit src;
        strictDeps = true;
      };

      cargoArtifacts = craneLib.buildDepsOnly common-args;

      bin = craneLib.buildPackage (common-args
        // {
          inherit cargoArtifacts;
        });

      plugin = pkgs.vimUtils.buildVimPlugin {
        pname = "vim-niri-nav-plugin";
        version = "unstable";
        src = lib.fileset.toSource {
          root = ./.;
          fileset = lib.fileset.unions [
            ./plugin
          ];
        };
        meta = {
          homepage = "https://github.com/lyndeno/vim-niri-nav";
          hydraPlatforms = [];
        };
      };

      vim-niri-nav = pkgs.symlinkJoin {
        name = "vim-niri-nav-combined";
        meta.mainProgram = "vim-niri-nav";
        paths = [
          plugin
          bin
        ];
      };
    in {
      packages = {
        inherit vim-niri-nav plugin bin;
        default = vim-niri-nav;
      };

      devShells.default = craneLib.devShell {
        packages = with pkgs; [
          rust-analyzer
          clippy
          rustfmt
        ];
      };
    });
}
