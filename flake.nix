{
  description = "vim-niri-nav — navigate niri windows and vim splits with the same keybindings";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";

    pre-commit-hooks-nix = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    crane,
    pre-commit-hooks-nix,
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

      pre-commit-check = hooks:
        pre-commit-hooks-nix.lib.${system}.run {
          src = ./.;
          inherit hooks;
        };
    in {
      packages = {
        inherit vim-niri-nav plugin bin;
        default = vim-niri-nav;
      };

      checks = {
        inherit bin;

        vim-niri-nav-clippy = craneLib.cargoClippy (common-args
          // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

        vim-niri-nav-fmt = craneLib.cargoFmt {
          inherit src;
        };

        vim-niri-nav-test = craneLib.cargoTest (common-args
          // {
            inherit cargoArtifacts;
          });

        pre-commit-check = pre-commit-check {
          alejandra.enable = true;
        };
      };

      devShells.default = let
        hooks = pre-commit-check {
          alejandra.enable = true;
          rustfmt.enable = true;
          clippy.enable = true;
        };
      in
        craneLib.devShell {
          packages = with pkgs; [
            rust-analyzer
            clippy
            rustfmt
          ];
          shellHook = ''
            ${hooks.shellHook}
          '';
        };
    });
}
