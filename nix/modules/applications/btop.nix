_:

{
  # Btop is a HYBRID app: generates theme file + merges settings
  # Two files need to be generated:
  # 1. themes/vogix.theme - the theme file
  # 2. btop.conf - btop config with theme reference (merged with user settings)

  # Theme file path relative to ~/.config/btop/
  themeFile = "themes/vogix.theme";

  # Config file path relative to ~/.config/btop/
  configFile = "btop.conf";

  # Format for config file (not used for hybrid apps - settings are merged directly)
  format = "toml";

  # Settings path in home-manager config
  settingsPath = "programs.btop.settings";

  # Reload method: SIGUSR2 triggers btop's config + theme reload
  reloadMethod = {
    method = "signal";
    signal = "SIGUSR2";
    process_name = "btop";
  };

  # Global settings applied to all schemes (merged with scheme-specific settings)
  settings = {
    color_theme = "vogix";
  };

  # Generators for each color scheme
  # Returns: { themeFile = "..."; } (settings merged globally)
  schemes = {
    # Vogix16: semantic color names
    vogix16 = colors: {
      themeFile = ''
        # Vogix16 theme for btop
        # Main interface - monochromatic base
        theme[main_bg]="${colors.background}"
        theme[main_fg]="${colors.foreground-text}"
        theme[title]="${colors.foreground-heading}"
        theme[hi_fg]="${colors.foreground-bright}"
        theme[selected_bg]="${colors.background-selection}"
        theme[selected_fg]="${colors.active}"
        theme[inactive_fg]="${colors.foreground-comment}"
        theme[proc_misc]="${colors.foreground-text}"

        # Box borders - all same color, no semantic meaning
        theme[cpu_box]="${colors.foreground-border}"
        theme[mem_box]="${colors.foreground-border}"
        theme[net_box]="${colors.foreground-border}"
        theme[proc_box]="${colors.foreground-border}"
        theme[div_line]="${colors.foreground-border}"

        # Graphs - monochromatic base
        theme[graph_text]="${colors.foreground-text}"
        theme[meter_bg]="${colors.background-surface}"

        # Semantic gradients (low → warning → danger)
        theme[temp_start]="${colors.foreground-comment}"
        theme[temp_mid]="${colors.warning}"
        theme[temp_end]="${colors.danger}"

        theme[cpu_start]="${colors.foreground-comment}"
        theme[cpu_mid]="${colors.warning}"
        theme[cpu_end]="${colors.danger}"

        theme[used_start]="${colors.foreground-comment}"
        theme[used_mid]="${colors.warning}"
        theme[used_end]="${colors.danger}"

        theme[available_start]="${colors.danger}"
        theme[available_mid]="${colors.warning}"
        theme[available_end]="${colors.foreground-comment}"

        theme[free_start]="${colors.danger}"
        theme[free_mid]="${colors.warning}"
        theme[free_end]="${colors.foreground-comment}"

        theme[cached_start]="${colors.foreground-comment}"
        theme[cached_mid]="${colors.warning}"
        theme[cached_end]="${colors.danger}"

        theme[download_start]="${colors.foreground-comment}"
        theme[download_mid]="${colors.warning}"
        theme[download_end]="${colors.danger}"

        theme[upload_start]="${colors.foreground-comment}"
        theme[upload_mid]="${colors.warning}"
        theme[upload_end]="${colors.danger}"

        theme[process_start]="${colors.foreground-comment}"
        theme[process_mid]="${colors.warning}"
        theme[process_end]="${colors.danger}"
      '';
    };

    # Base16: raw base00-base0F colors
    base16 = colors: {
      themeFile = ''
        # Base16 theme for btop
        theme[main_bg]="${colors.base00}"
        theme[main_fg]="${colors.base05}"
        theme[title]="${colors.base07}"
        theme[hi_fg]="${colors.base07}"
        theme[selected_bg]="${colors.base02}"
        theme[selected_fg]="${colors.base0D}"
        theme[inactive_fg]="${colors.base03}"
        theme[proc_misc]="${colors.base05}"

        theme[cpu_box]="${colors.base04}"
        theme[mem_box]="${colors.base04}"
        theme[net_box]="${colors.base04}"
        theme[proc_box]="${colors.base04}"
        theme[div_line]="${colors.base04}"

        theme[graph_text]="${colors.base05}"
        theme[meter_bg]="${colors.base01}"

        theme[temp_start]="${colors.base0B}"
        theme[temp_mid]="${colors.base0A}"
        theme[temp_end]="${colors.base08}"

        theme[cpu_start]="${colors.base0B}"
        theme[cpu_mid]="${colors.base0A}"
        theme[cpu_end]="${colors.base08}"

        theme[used_start]="${colors.base0B}"
        theme[used_mid]="${colors.base0A}"
        theme[used_end]="${colors.base08}"

        theme[available_start]="${colors.base08}"
        theme[available_mid]="${colors.base0A}"
        theme[available_end]="${colors.base0B}"

        theme[free_start]="${colors.base08}"
        theme[free_mid]="${colors.base0A}"
        theme[free_end]="${colors.base0B}"

        theme[cached_start]="${colors.base0C}"
        theme[cached_mid]="${colors.base0D}"
        theme[cached_end]="${colors.base0E}"

        theme[download_start]="${colors.base0C}"
        theme[download_mid]="${colors.base0D}"
        theme[download_end]="${colors.base0E}"

        theme[upload_start]="${colors.base0B}"
        theme[upload_mid]="${colors.base0A}"
        theme[upload_end]="${colors.base09}"

        theme[process_start]="${colors.base0B}"
        theme[process_mid]="${colors.base0A}"
        theme[process_end]="${colors.base08}"
      '';
    };

    # Base24: base00-base17 with true bright colors
    base24 = colors: {
      themeFile = ''
        # Base24 theme for btop
        theme[main_bg]="${colors.base00}"
        theme[main_fg]="${colors.base05}"
        theme[title]="${colors.base07}"
        theme[hi_fg]="${colors.base07}"
        theme[selected_bg]="${colors.base02}"
        theme[selected_fg]="${colors.base16}"
        theme[inactive_fg]="${colors.base03}"
        theme[proc_misc]="${colors.base05}"

        theme[cpu_box]="${colors.base04}"
        theme[mem_box]="${colors.base04}"
        theme[net_box]="${colors.base04}"
        theme[proc_box]="${colors.base04}"
        theme[div_line]="${colors.base04}"

        theme[graph_text]="${colors.base05}"
        theme[meter_bg]="${colors.base01}"

        theme[temp_start]="${colors.base14}"
        theme[temp_mid]="${colors.base13}"
        theme[temp_end]="${colors.base12}"

        theme[cpu_start]="${colors.base14}"
        theme[cpu_mid]="${colors.base13}"
        theme[cpu_end]="${colors.base12}"

        theme[used_start]="${colors.base14}"
        theme[used_mid]="${colors.base13}"
        theme[used_end]="${colors.base12}"

        theme[available_start]="${colors.base12}"
        theme[available_mid]="${colors.base13}"
        theme[available_end]="${colors.base14}"

        theme[free_start]="${colors.base12}"
        theme[free_mid]="${colors.base13}"
        theme[free_end]="${colors.base14}"

        theme[cached_start]="${colors.base15}"
        theme[cached_mid]="${colors.base16}"
        theme[cached_end]="${colors.base17}"

        theme[download_start]="${colors.base15}"
        theme[download_mid]="${colors.base16}"
        theme[download_end]="${colors.base17}"

        theme[upload_start]="${colors.base14}"
        theme[upload_mid]="${colors.base13}"
        theme[upload_end]="${colors.base09}"

        theme[process_start]="${colors.base14}"
        theme[process_mid]="${colors.base13}"
        theme[process_end]="${colors.base12}"
      '';
    };

    # ANSI16: direct terminal colors
    ansi16 = colors: {
      themeFile = ''
        # ANSI16 theme for btop
        theme[main_bg]="${colors.background}"
        theme[main_fg]="${colors.foreground}"
        theme[title]="${colors.color15}"
        theme[hi_fg]="${colors.color15}"
        theme[selected_bg]="${colors.selection_bg}"
        theme[selected_fg]="${colors.color12}"
        theme[inactive_fg]="${colors.color08}"
        theme[proc_misc]="${colors.foreground}"

        theme[cpu_box]="${colors.color08}"
        theme[mem_box]="${colors.color08}"
        theme[net_box]="${colors.color08}"
        theme[proc_box]="${colors.color08}"
        theme[div_line]="${colors.color08}"

        theme[graph_text]="${colors.foreground}"
        theme[meter_bg]="${colors.color00}"

        theme[temp_start]="${colors.color10}"
        theme[temp_mid]="${colors.color11}"
        theme[temp_end]="${colors.color09}"

        theme[cpu_start]="${colors.color10}"
        theme[cpu_mid]="${colors.color11}"
        theme[cpu_end]="${colors.color09}"

        theme[used_start]="${colors.color10}"
        theme[used_mid]="${colors.color11}"
        theme[used_end]="${colors.color09}"

        theme[available_start]="${colors.color09}"
        theme[available_mid]="${colors.color11}"
        theme[available_end]="${colors.color10}"

        theme[free_start]="${colors.color09}"
        theme[free_mid]="${colors.color11}"
        theme[free_end]="${colors.color10}"

        theme[cached_start]="${colors.color14}"
        theme[cached_mid]="${colors.color12}"
        theme[cached_end]="${colors.color13}"

        theme[download_start]="${colors.color14}"
        theme[download_mid]="${colors.color12}"
        theme[download_end]="${colors.color13}"

        theme[upload_start]="${colors.color10}"
        theme[upload_mid]="${colors.color11}"
        theme[upload_end]="${colors.color01}"

        theme[process_start]="${colors.color10}"
        theme[process_mid]="${colors.color11}"
        theme[process_end]="${colors.color09}"
      '';
    };
  };
}
