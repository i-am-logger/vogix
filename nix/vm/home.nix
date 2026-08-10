{ pkgs, ... }:

{
  home = {
    username = "vogix";
    homeDirectory = "/home/vogix";
    stateVersion = "24.11";

    # Install apps - configs are managed by vogix
    # Note: We don't use programs.alacritty.enable / programs.btop.enable
    # because that conflicts with vogix's config file management.
    packages = with pkgs; [
      alacritty
      btop
      fastfetch
    ];
  };

  programs = {
    # Enable vogix
    vogix = {
      enable = true;
      appearance = {
        theme = "yoga";
        variant = "dark";

        # Themes are auto-discovered, but only these are staged into the store.
        #
        # Discovery finds every bundled scheme collection -- 180 base16, 159
        # base24, 367 ansi16 -- which merges to hundreds of theme-variants,
        # each a derivation, each containing another derivation per themed app.
        # That is thousands of builds per VM, repeated for every test in `nix
        # flake check`, to exercise the four themes below.
        #
        # Every theme a test selects has to be here: `vogix theme set` resolves
        # against this staged tree, so anything left out fails with "theme
        # variant <name> not found".
        prebuiltThemes = [
          "yoga" # the default the tests start from; stress cycles through it
          "nordic" # stress: rapid-switch cycle and polarity combinations
          "matrix" # stress: rapid-switch cycle and polarity combinations
          "desert" # theme-switching, and the stress cycle
          "dracula" # base16, scheme-switching
          "catppuccin" # base16, its frappe variant is read by the palette test
        ];
      };
      # Apps are auto-detected from enabled programs (alacritty, btop, bash, console)
      # You can disable individual apps with: alacritty.enable = false; etc.
      enableDaemon = false; # Disabled for tests - daemon requires home-manager/.config watch path
    };

    # Enable git for convenience
    git = {
      enable = true;
      settings = {
        user.name = "Vogix Test";
        user.email = "test@vogix.local";
      };
    };

    # Enable bash (vogix module will auto-configure shell colors)
    bash = {
      enable = true;
      shellAliases = {
        ll = "ls -l";
        v = "vogix";
        reload-colors = "source ~/.config/shell-colors/colors.sh && source ~/.config/ls-colors/dircolors && echo 'Colors reloaded!'";
        colors = "echo -e '\\033[0;30m Black \\033[0;31m Red \\033[0;32m Green \\033[0;33m Yellow \\033[0;34m Blue \\033[0;35m Magenta \\033[0;36m Cyan \\033[0;37m White \\033[0m' && echo -e '\\033[1;30m Bright Black \\033[1;31m Bright Red \\033[1;32m Bright Green \\033[1;33m Bright Yellow \\033[1;34m Bright Blue \\033[1;35m Bright Magenta \\033[1;36m Bright Cyan \\033[1;37m Bright White \\033[0m'";
        shutdown = "sudo systemctl poweroff";
      };
      sessionVariables = {
        COLORTERM = "truecolor"; # Enable true color support
      };
      initExtra = ''
        # Show welcome message with theme info
        echo ""
        echo "=== Vogix Test VM ==="
        vogix theme status
        echo ""
        echo "Available commands:"
        echo "  vogix theme status              - Show current theme/variant/scheme"
        echo "  vogix theme list                - List all themes"
        echo "  vogix theme list -s base16      - List base16 themes"
        echo "  vogix theme list -s base24      - List base24 themes"
        echo "  vogix theme list -s ansi16      - List ansi16 themes"
        echo "  vogix theme set -t <theme>      - Switch theme"
        echo "  vogix theme set -v dark|light   - Switch variant"
        echo "  vogix theme set -v darker       - Navigate to darker variant"
        echo "  vogix theme set -v lighter      - Navigate to lighter variant"
        echo "  vogix theme set -s <scheme>     - Switch scheme (vogix16/base16/base24/ansi16)"
        echo "  vogix theme set -s base16 -t dracula -v dark  - Combined flags"
        echo ""
        echo "Shell commands:"
        echo "  reload-colors    - Reload shell colors after theme change"
        echo "  colors           - Show ANSI color palette"
        echo "  ls --color       - Test directory colors"
        echo "  shutdown         - Power off VM"
        echo ""
      '';
    };
  };
}
