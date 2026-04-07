_:

{
  # Wezterm colors file loaded by main config via dofile()
  configFile = "colors.lua";

  # Smart Ctrl+C/V keybindings for Super→Ctrl remap (macOS Command behavior)
  # Injected into programs.wezterm.extraConfig by the home-manager module
  keybindings = ''
    -- Vogix: Smart Ctrl+C/V (Super→Ctrl remap behavior)
    config.keys = {
      {
        key = 'c',
        mods = 'CTRL',
        action = wezterm.action_callback(function(window, pane)
          local sel = window:get_selection_text_for_pane(pane)
          if sel and sel ~= "" then
            window:perform_action(wezterm.action.CopyTo('Clipboard'), pane)
          else
            window:perform_action(wezterm.action.SendKey{key='c', mods='CTRL'}, pane)
          end
        end),
      },
      {
        key = 'v',
        mods = 'CTRL',
        action = wezterm.action.PasteFrom('Clipboard'),
      },
    }

    return config
  '';

  reloadMethod = {
    method = "command";
    command = "touch -h \"$HOME/.config/wezterm/wezterm.lua\" 2>/dev/null || true";
  };

  schemes = {
    vogix16 = colors: ''
      return {
        background = "${colors.background}",
        foreground = "${colors.foreground-text}",
        cursor_bg = "${colors.active}",
        cursor_fg = "${colors.background}",
        cursor_border = "${colors.active}",
        selection_bg = "${colors.background-selection}",
        selection_fg = "${colors.foreground-text}",
        ansi = {"${colors.background}","${colors.danger}","${colors.success}","${colors.warning}","${colors.link}","${colors.highlight}","${colors.active}","${colors.foreground-text}"},
        brights = {"${colors.foreground-comment}","${colors.danger}","${colors.success}","${colors.warning}","${colors.link}","${colors.highlight}","${colors.active}","${colors.foreground-bright}"},
      }
    '';

    base16 = colors: ''
      return {
        background = "${colors.base00}",
        foreground = "${colors.base05}",
        cursor_bg = "${colors.base05}",
        cursor_fg = "${colors.base00}",
        cursor_border = "${colors.base05}",
        selection_bg = "${colors.base02}",
        selection_fg = "${colors.base05}",
        ansi = {"${colors.base00}","${colors.base08}","${colors.base0B}","${colors.base0A}","${colors.base0D}","${colors.base0E}","${colors.base0C}","${colors.base05}"},
        brights = {"${colors.base03}","${colors.base08}","${colors.base0B}","${colors.base0A}","${colors.base0D}","${colors.base0E}","${colors.base0C}","${colors.base07}"},
      }
    '';

    base24 = colors: ''
      return {
        background = "${colors.base00}",
        foreground = "${colors.base05}",
        cursor_bg = "${colors.base05}",
        cursor_fg = "${colors.base00}",
        cursor_border = "${colors.base05}",
        selection_bg = "${colors.base02}",
        selection_fg = "${colors.base05}",
        ansi = {"${colors.base00}","${colors.base08}","${colors.base0B}","${colors.base0A}","${colors.base0D}","${colors.base0E}","${colors.base0C}","${colors.base05}"},
        brights = {"${colors.base03}","${colors.base12}","${colors.base14}","${colors.base13}","${colors.base16}","${colors.base17}","${colors.base15}","${colors.base07}"},
      }
    '';

    ansi16 = colors: ''
      return {
        background = "${colors.background}",
        foreground = "${colors.foreground}",
        cursor_bg = "${colors.cursor_bg}",
        cursor_fg = "${colors.cursor_fg}",
        cursor_border = "${colors.cursor_bg}",
        selection_bg = "${colors.selection_bg}",
        selection_fg = "${colors.selection_fg}",
        ansi = {"${colors.color00}","${colors.color01}","${colors.color02}","${colors.color03}","${colors.color04}","${colors.color05}","${colors.color06}","${colors.color07}"},
        brights = {"${colors.color08}","${colors.color09}","${colors.color10}","${colors.color11}","${colors.color12}","${colors.color13}","${colors.color14}","${colors.color15}"},
      }
    '';
  };
}
