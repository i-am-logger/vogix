{ lib, ... }:

# baud theme files, one per polarity
#
# baud follows the desktop's light/dark itself (`theme = "auto"`), drawing
# with `[palette] dark` or `light`. A vogix theme therefore gives baud BOTH
# halves: this variant for its own polarity and the theme's default variant
# for the other, so a portal flip keeps the vogix palette instead of falling
# back to baud's built-in half.
#
# The files are kitty themes (color0-15, foreground, background, cursor,
# selection): baud recognises a theme by its content, and kitty's keys carry
# every colour the wezterm module sets, with the same ANSI mapping, so baud
# and wezterm draw the same 16 colours.
#
# A vogix switch changes current-theme, not the links in
# ~/.config/baud/themes, so baud's own watch (the themes directory, the
# files' bytes compared after each event) hears nothing until the reload
# touches the links: IN_ATTRIB on that directory, and baud reads the new
# bytes through them.
let
  kittyTheme =
    { background
    , foreground
    , cursor
    , cursorText ? background
    , selection
    , selectionText ? foreground
    , ansi
    }:
    lib.concatStringsSep "\n" (
      [
        "background ${background}"
        "foreground ${foreground}"
        "cursor ${cursor}"
        "cursor_text_color ${cursorText}"
        "selection_background ${selection}"
        "selection_foreground ${selectionText}"
      ]
      ++ lib.imap0 (i: c: "color${toString i} ${c}") ansi
    );

  # Both halves from one scheme's colours -> theme mapping
  both = toTheme: { palettes, ... }: {
    files = {
      "themes/vogix-dark" = kittyTheme (toTheme palettes.dark);
      "themes/vogix-light" = kittyTheme (toTheme palettes.light);
    };
  };
in
{
  files = [ "themes/vogix-dark" "themes/vogix-light" ];

  reloadMethod = {
    method = "command";
    command = "touch -h \"\${XDG_CONFIG_HOME:-$HOME/.config}/baud/themes/vogix-dark\" \"\${XDG_CONFIG_HOME:-$HOME/.config}/baud/themes/vogix-light\" 2>/dev/null || true";
  };

  schemes = {
    vogix16 = both (colors: {
      inherit (colors) background;
      foreground = colors.foreground-text;
      cursor = colors.active;
      selection = colors.background-selection;
      ansi = [
        colors.background
        colors.danger
        colors.success
        colors.warning
        colors.link
        colors.highlight
        colors.active
        colors.foreground-text
        colors.foreground-comment
        colors.danger
        colors.success
        colors.warning
        colors.link
        colors.highlight
        colors.active
        colors.foreground-bright
      ];
    });

    base16 = both (colors: {
      background = colors.base00;
      foreground = colors.base05;
      cursor = colors.base05;
      selection = colors.base02;
      ansi = [
        colors.base00
        colors.base08
        colors.base0B
        colors.base0A
        colors.base0D
        colors.base0E
        colors.base0C
        colors.base05
        colors.base03
        colors.base08
        colors.base0B
        colors.base0A
        colors.base0D
        colors.base0E
        colors.base0C
        colors.base07
      ];
    });

    base24 = both (colors: {
      background = colors.base00;
      foreground = colors.base05;
      cursor = colors.base05;
      selection = colors.base02;
      ansi = [
        colors.base00
        colors.base08
        colors.base0B
        colors.base0A
        colors.base0D
        colors.base0E
        colors.base0C
        colors.base05
        colors.base03
        colors.base12
        colors.base14
        colors.base13
        colors.base16
        colors.base17
        colors.base15
        colors.base07
      ];
    });

    ansi16 = both (colors: {
      inherit (colors) background foreground;
      cursor = colors.cursor_bg;
      cursorText = colors.cursor_fg;
      selection = colors.selection_bg;
      selectionText = colors.selection_fg;
      ansi = map (i: colors."color${lib.fixedWidthNumber 2 i}") (lib.range 0 15);
    });
  };
}
