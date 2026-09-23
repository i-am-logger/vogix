# Reload Mechanism

When a theme or variant is switched, applications need to be notified to reload their configurations. Vogix uses a configuration-driven approach to handle this.

## Configuration-Based Reload

Instead of hardcoding application reload methods, each application defines its reload method in the generated user manifest (`~/.local/state/vogix/config.toml`). Each app's metadata carries a flat `reload_method` field plus method-specific siblings (`reload_signal`, `process_name`, `reload_command`, `theme_file_path`):

```toml
# Example reload configurations (entries under [apps.<name>])

[apps.waybar]
config_path = "/home/user/.config/waybar/colors.css"
reload_method = "signal"
reload_signal = "SIGUSR2"
process_name = "waybar"

[apps.hyprland]
config_path = "/home/user/.config/hypr/colors.conf"
reload_method = "command"
reload_command = "hyprctl reload"

[apps.alacritty]
config_path = "/home/user/.config/alacritty/alacritty.toml"
reload_method = "touch"

[apps.fish]
config_path = "/home/user/.config/fish/colors.fish"
reload_method = "none"
```

## Supported Reload Methods

The `ReloadDispatcher` handles exactly four `reload_method` values. Any other value errors with `unknown reload method`:

1. **`signal`**: Sends a Unix signal to every process named `process_name` (defaulting to the app name). Requires `reload_signal`, which must be one of `SIGUSR1`, `SIGUSR2`, `SIGHUP`, `SIGTERM`, or `SIGINT` — any other signal is rejected as unsupported. An app with no such process is *not running*: there is nothing to reload, it reads the new theme when it starts, and it is not counted as a failure.
2. **`command`**: Runs the shell command in `reload_command` (e.g. `hyprctl reload`).
3. **`touch`**: Touches (or re-creates the symlink for) `config_path`, and `theme_file_path` if set, to trigger the application's own inotify-based auto-reload.
4. **`none`**: No runtime reload; the new theme takes effect on next launch.

## Implementation

The reload system:

1. Reads the reload configuration for each themed application
2. Executes the appropriate reload method
3. Reports each app that failed to reload; an app that is not running is logged at debug level only

Adding support for a new application only requires adding its reload configuration, not modifying the code.

## Apply Hooks

`programs.vogix.themeApply` declares user apply hooks, rendered into `config.toml` as `[hooks."<name>"]` tables:

```toml
[hooks."greeter"]
command = """/nix/store/…-vogix/bin/vogix greeter sync"""
```

On every theme apply (`theme set`, `refresh`, `undo`, `redo`) the CLI runs all hooks in parallel, each as `sh -c` with every `{{slot}}` placeholder (e.g. `{{base01}}`, `{{active}}`) replaced by that slot's colour as `rrggbb`. A hook that cannot start or exits non-zero is reported and does not fail the apply.
