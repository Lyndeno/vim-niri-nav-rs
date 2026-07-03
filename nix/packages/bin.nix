{
  craneLib,
  installShellFiles,
  ...
}: let
  src = craneLib.cleanCargoSource ../..;

  common-args = {
    inherit src;
    strictDeps = true;

    nativeBuildInputs = [installShellFiles];

    postInstall = ''
      installShellCompletion --cmd vim-niri-nav \
        --bash ./target/release/build/vim-niri-nav-*/out/vim-niri-nav.bash \
        --fish ./target/release/build/vim-niri-nav-*/out/vim-niri-nav.fish \
        --zsh ./target/release/build/vim-niri-nav-*/out/_vim-niri-nav
      installManPage ./target/release/build/vim-niri-nav-*/out/vim-niri-nav.1
    '';
  };

  cargoArtifacts = craneLib.buildDepsOnly common-args;
in
  craneLib.buildPackage (common-args
    // {
      inherit cargoArtifacts;

      passthru.tests = {
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
      };
    })
