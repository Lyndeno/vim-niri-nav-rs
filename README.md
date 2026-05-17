# vim-niri-nav

Seamless navigation between [niri](https://github.com/YaLTeR/niri) windows and (Neo)Vim splits using the same keybindings.

Forked from [andergrim/vim-niri-nav](https://github.com/andergrim/vim-niri-nav), which was modified from [vim-sway-nav](https://jasoncarloscox.com/creations/vim-sway-nav/), inspired by [vim-tmux-navigator](https://github.com/christoomey/vim-tmux-navigator).

## How it works

A small Rust binary talks directly to the niri IPC socket to find the focused window. If the focused window is running Vim or Neovim, it attempts to navigate within the editor first. If the cursor is already at the edge of the splits, or no editor is focused, it falls back to a normal niri focus action.

## Requirements

- Vim built with `+clientserver` (`vim --version | grep clientserver`), or Neovim
- niri running (the binary communicates via `$NIRI_SOCKET`)

## Installation

### Nix flake (recommended)

The flake exposes three packages:

| Package | Contents |
|---------|----------|
| `vim-niri-nav` *(default)* | Binary + Vim plugin combined |
| `bin` | Rust binary only |
| `plugin` | Vim plugin only |

Add the flake as an input:

```nix
inputs.vim-niri-nav.url = "github:lyndeno/vim-niri-nav-rs";
```

#### NixOS / Home Manager

To install the combined package (binary on `$PATH` + plugin available to your editor):

```nix
environment.systemPackages = [
  inputs.vim-niri-nav.packages.${system}.default
];
```

Or split them if your editor config manages plugins separately:

```nix
# Binary
environment.systemPackages = [
  inputs.vim-niri-nav.packages.${system}.bin
];

# Plugin (example with Home Manager + Neovim)
programs.neovim.plugins = [
  inputs.vim-niri-nav.packages.${system}.plugin
];
```

#### niri IPC compatibility

The binary communicates with niri directly via its IPC socket by default. If you are running a version of niri that is incompatible with the bundled `niri-ipc` crate, you can build the fallback version which shells out to `niri msg` instead:

```nix
inputs.vim-niri-nav.packages.${system}.default.override {
  cargoExtraArgs = "--no-default-features";
}
```

### Manual

Build the binary and place it on your `$PATH`:

```sh
cargo build --release --manifest-path rpc/Cargo.toml
cp rpc/target/release/vim-niri-nav ~/.local/bin/
```

Then install the Vim plugin from the `plugin/` directory using your plugin manager. For example with [vim-plug](https://github.com/junegunn/vim-plug):

```vim
Plug 'lyndeno/vim-niri-nav-rs'
```

## niri configuration

Replace your normal focus bindings with `vim-niri-nav`:

```kdl
Mod+Left  { spawn "vim-niri-nav" "left"; }
Mod+Down  { spawn "vim-niri-nav" "down"; }
Mod+Up    { spawn "vim-niri-nav" "up"; }
Mod+Right { spawn "vim-niri-nav" "right"; }
```

### Workspace traversal

Pass `w` as a second argument to fall through to `focus-window-or-workspace-[up|down]` when at the edge of niri windows:

```kdl
Mod+Down { spawn "vim-niri-nav" "down" "w"; }
Mod+Up   { spawn "vim-niri-nav" "up"   "w"; }
```

### Monitor traversal

Pass `m` to fall through to `focus-window-or-monitor-[up|down]` or `focus-column-or-monitor-[left|right]`:

```kdl
Mod+Left  { spawn "vim-niri-nav" "left"  "m"; }
Mod+Down  { spawn "vim-niri-nav" "down"  "m"; }
Mod+Up    { spawn "vim-niri-nav" "up"    "m"; }
Mod+Right { spawn "vim-niri-nav" "right" "m"; }
```

## Configuration

### Timeout

The binary applies a timeout when communicating with Vim/Neovim, falling back to a niri focus action if the editor does not respond in time. The default is `0.1s`. Override it with the `VIM_NIRI_NAV_TIMEOUT` environment variable (value in seconds):

```sh
# In your shell profile or niri environment config
export VIM_NIRI_NAV_TIMEOUT=0.2   # 200ms
export VIM_NIRI_NAV_TIMEOUT=0     # disable timeout
```

## Contributing

Contributions are welcome. Bug reports and pull requests can be sent through GitHub.
