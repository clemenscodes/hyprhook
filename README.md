# hyprhook

Run commands when Hyprland windows open, close, gain focus, or lose focus.
Each rule matches windows by class and/or title (both are regular expressions, AND-ed together).

```toml
# ~/.config/hyprhook/config.toml

[[window]]
class     = "^gamescope$"
title     = "Counter-Strike 2"
on_open   = [["obs-cli", "start-recording"]]
on_close  = [["obs-cli", "stop-recording"]]
on_focus  = [["hyprctl", "dispatch", "submap", "gaming"]]
on_unfocus = [["hyprctl", "dispatch", "submap", "reset"]]
```

All four event types are optional — omit any you don't need.
Rules without `class` or `title` match all windows.

Each command is an argv list: the first element is the executable, the rest are its arguments.
Use absolute paths when the executable may not be on `PATH`.

Commands are queued and run one at a time, preventing IPC socket floods on rapid focus changes.

## Standalone installation

### Nix (flake)

```nix
inputs.hyprhook.url = "github:clemenscodes/hyprhook";
```

Add to your packages:

```nix
environment.systemPackages = [inputs.hyprhook.packages.${system}.default];
```

Then create `~/.config/hyprhook/config.toml` and launch from Hyprland:

```ini
# hyprland.conf
exec-once = hyprhook
```

### Cargo

```sh
cargo install --git https://github.com/clemenscodes/hyprhook
```

## NixOS module

The flake exposes a NixOS module at `nixosModules.default` that:
- Generates the TOML config from Nix options
- Wraps the binary so it picks up the generated config automatically
- Exposes `services.hyprhook.finalPackage` — the pre-configured binary ready to launch

### Usage

```nix
{inputs, ...}: {
  imports = [inputs.hyprhook.nixosModules.default];

  services.hyprhook = {
    enable = true;
    windows = [
      {
        class     = "^gamescope$";
        title     = "Counter-Strike 2";
        on_focus  = [["hyprctl" "dispatch" "submap" "gaming"]];
        on_unfocus = [["hyprctl" "dispatch" "submap" "reset"]];
      }
    ];
  };
}
```

`services.hyprhook.finalPackage` is the wrapped binary. Launch it from Hyprland:

```ini
exec-once = ${config.services.hyprhook.finalPackage}/bin/hyprhook
```

### Full option reference

| Option | Type | Default | Description |
|---|---|---|---|
| `services.hyprhook.enable` | `bool` | `false` | Enable the module |
| `services.hyprhook.package` | `package` | flake default | The hyprhook package |
| `services.hyprhook.finalPackage` | `package` | (read-only) | Binary pre-configured with generated TOML |
| `services.hyprhook.windows` | `list of window rules` | `[]` | Hook rules (see below) |

Each entry in `windows`:

| Field | Type | Default | Description |
|---|---|---|---|
| `class` | `str \| null` | `null` | Regex for window class; `null` matches any |
| `title` | `str \| null` | `null` | Regex for window title; `null` matches any |
| `on_open` | `list of argv` | `[]` | Commands when window is created |
| `on_close` | `list of argv` | `[]` | Commands when window is destroyed |
| `on_focus` | `list of argv` | `[]` | Commands when window gains focus |
| `on_unfocus` | `list of argv` | `[]` | Commands when window loses focus |

Each `argv` is a non-empty list of strings: `["executable", "arg1", "arg2", ...]`.

## CymenixOS

If you are using [CymenixOS](https://github.com/clemenscodes/cymenixos), hyprhook is available under `modules.io.hyprhook`:

```nix
modules.io = {
  enable = true;
  hyprhook = {
    enable = true;
    windows = [
      {
        class      = "^gamescope$";
        title      = "Counter-Strike 2";
        on_focus   = [["hyprctl" "dispatch" "submap" "gaming"]];
        on_unfocus = [["hyprctl" "dispatch" "submap" "reset"]];
      }
    ];
  };
};
```

The CymenixOS module automatically adds `exec-once` to your Hyprland config.
