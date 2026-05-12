{
  description = "vim-niri-nav helper script and nvim RPC client";

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

      rpc = craneLib.buildPackage (common-args
        // {
          inherit cargoArtifacts;
        });

      plugin = pkgs.vimUtils.buildVimPlugin {
        pname = "vim-niri-nav";
        version = "unstable";
        src = ./.;
        nativeBuildInputs = [pkgs.makeWrapper];
        postInstall = ''
          mkdir -p $out/bin
          cp $out/vim-niri-nav $out/bin/vim-niri-nav
          wrapProgram $out/bin/vim-niri-nav \
            --prefix PATH : ${pkgs.lib.makeBinPath [pkgs.jq rpc]}
        '';
        meta = {
          homepage = "https://github.com/lyndeno/vim-niri-nav";
          hydraPlatforms = [];
          mainProgram = "vim-niri-nav";
        };
      };
    in {
      packages = {
        inherit rpc plugin;
        default = plugin;
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
