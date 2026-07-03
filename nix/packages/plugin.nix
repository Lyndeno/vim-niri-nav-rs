{
  vimUtils,
  lib,
  ...
}:
vimUtils.buildVimPlugin {
  pname = "vim-niri-nav-plugin";
  version = "unstable";
  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ../../plugin
    ];
  };
  meta = {
    homepage = "https://github.com/lyndeno/vim-niri-nav";
    hydraPlatforms = [];
  };
}
