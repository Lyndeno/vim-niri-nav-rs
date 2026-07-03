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

    ci.url = "github:Lyndeno/ci";
    ci.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    crane,
    pre-commit-hooks-nix,
    ci,
  }:
    flake-utils.lib.eachSystem (builtins.filter (s: builtins.match ".*-linux" s != null) flake-utils.lib.defaultSystems) (system: let
      pkgs = nixpkgs.legacyPackages.${system}.extend self.overlays.default;
      inherit (pkgs) lib;
      craneLib = crane.mkLib pkgs;

      bin = pkgs.vim-niri-nav-bin;
      plugin = pkgs.vim-niri-nav-plugin;
      vim-niri-nav = pkgs.vim-niri-nav;

      pre-commit-check = hooks:
        pre-commit-hooks-nix.lib.${system}.run {
          src = ./.;
          inherit hooks;
        };
    in {
      packages = {
        inherit vim-niri-nav plugin bin;
        default = vim-niri-nav;
        hydra-spec = ci.lib.mkHydraSpec {
          inherit pkgs;
          owner = "Lyndeno";
          repo = "vim-niri-nav-rs";
        };
        mergify = ci.lib.mkMergifyConfig {
          inherit pkgs;
          projectName = "vim-niri-nav-rs";
          checks = self.checks;
        };
      };

      checks =
        {
          inherit bin;
        }
        // bin.passthru.tests
        // {
          pre-commit-check = pre-commit-check {
            alejandra.enable = true;
          };

          hydra-spec = ci.lib.mkHydraCheck {
            inherit pkgs;
            specPackage = self.packages.${system}.hydra-spec;
            specFile = ./.hydra/spec.json;
          };

          mergify-check = ci.lib.mkMergifyCheck {
            inherit pkgs;
            mergifyPackage = self.packages.${system}.mergify;
            mergifyFile = ./.mergify.yml;
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
    })
    // {
      overlays.default = final: _prev: let
        craneLib = crane.mkLib final;
      in {
        vim-niri-nav-bin = final.callPackage ./nix/packages/bin.nix {inherit craneLib;};
        vim-niri-nav-plugin = final.callPackage ./nix/packages/plugin.nix {};
        vim-niri-nav = final.callPackage ./nix/packages/vim-niri-nav.nix {};
      };
    };
}
