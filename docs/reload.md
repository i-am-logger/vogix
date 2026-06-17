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

1. **`signal`**: Sends a Unix signal to `process_name` (defaulting to the app name). Requires `reload_signal`, which must be one of `SIGUSR1`, `SIGUSR2`, `SIGHUP`, `SIGTERM`, or `SIGINT` — any other signal is rejected as unsupported.
2. **`command`**: Runs the shell command in `reload_command` (e.g. `hyprctl reload`).
3. **`touch`**: Touches (or re-creates the symlink for) `config_path`, and `theme_file_path` if set, to trigger the application's own inotify-based auto-reload.
4. **`none`**: No runtime reload; the new theme takes effect on next launch.

## Implementation

The reload system:

1. Reads the reload configuration for each themed application
2. Checks if the application is running
3. Executes the appropriate reload method
4. Handles failures gracefully with fallbacks and error reporting

Adding support for a new application only requires adding its reload configuration, not modifying the code.

## Fallback Mechanisms

If an application doesn't support runtime reload:

1. The system will note that the configuration has changed
2. The application will use the new theme on next launch
3. Optionally, the user can be notified that manual restart is required

