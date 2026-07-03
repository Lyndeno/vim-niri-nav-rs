{
  symlinkJoin,
  vim-niri-nav-bin,
  vim-niri-nav-plugin,
  ...
}:
symlinkJoin {
  name = "vim-niri-nav-combined";
  meta.mainProgram = "vim-niri-nav";
  paths = [
    vim-niri-nav-plugin
    vim-niri-nav-bin
  ];
}
