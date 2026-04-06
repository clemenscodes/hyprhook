# hyprhook

Run shell commands when Hyprland windows open, close, gain focus, or lose focus.
Each rule matches windows by class and/or title (both are Go-compatible regular expressions, AND-ed together).

```toml
# ~/.config/hyprhook/config.toml

[[window]]
class     = "^gamescope$"
title     = "Counter-Strike 2"
on_open   = ["obs-cli start-recording"]
on_close  = ["obs-cli stop-recording"]
on_focus  = ["wootswitch switch CS2"]
on_unfocus = ["wootswitch switch Default"]
```

All four event types are optional — omit any you don't need.
Rules without `class` or `title` match all windows.

Each command runs via `sh -c` with two environment variables set:

| Variable | Value |
|---|---|
| `HYPRHOOK_WINDOW_CLASS` | Window class of the matched window |
| `HYPRHOOK_WINDOW_TITLE` | Window title of the matched window |

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
        on_focus  = ["wootswitch switch CS2"];
        on_unfocus = ["wootswitch switch Default"];
      }
    ];
  };
}
```

`services.hyprhook.finalPackage` is the wrapped binary. Launch it from Hyprland:

```ini
exec-once = ${config.services.hyprhook.finalPackage}/bin/hyprhook
```

Or manage it as a systemd user service (see below).

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
| `on_open` | `list of str` | `[]` | Commands when window is created |
| `on_close` | `list of str` | `[]` | Commands when window is destroyed |
| `on_focus` | `list of str` | `[]` | Commands when window gains focus |
| `on_unfocus` | `list of str` | `[]` | Commands when window loses focus |

### Running as a systemd user service

```nix
systemd.user.services.hyprhook = {
  description = "hyprhook Hyprland window event hook runner";
  wantedBy    = ["graphical-session.target"];
  after       = ["graphical-session.target"];
  serviceConfig = {
    Type       = "simple";
    ExecStart  = "${config.services.hyprhook.finalPackage}/bin/hyprhook";
    Restart    = "on-failure";
    RestartSec = "1s";
  };
};
```

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
        on_focus   = ["wootswitch switch CS2"];
        on_unfocus = ["wootswitch switch Default"];
      }
    ];
  };
};
```

The CymenixOS module automatically sets up the systemd user service.
