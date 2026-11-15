{ lib, ... }:

# fcitx5 classicui theme: the input method's candidate window and menu
#
# A user theme lives at ~/.local/share/fcitx5/themes/<name>/theme.conf;
# classicui.conf's `Theme=vogix` selects it (written by the host, not here:
# vogix never touches classicui.conf). Keep classicui's `UseDarkTheme` off,
# or a dark portal swaps in fcitx5's own `DarkTheme`.
#
# Styled as the desktop shell's launcher: background_surface panel, a 1px
# foreground_border frame, background_selection under the chosen candidate.
# Every text colour keeps 4.5:1 (WCAG 1.4.3) on what it is drawn on:
# labels and comments are foreground_text, and the accent marks the chosen
# candidate only where it reaches 4.5:1 on the selection, foreground_bright
# otherwise. No [AccentColorField], so the portal's accent never overrides
# the palette, and no images: without them fcitx5 draws no page arrows
# (paging by key still works) and no menu check marks.
let
  inherit (import ../lib/color.nix { inherit lib; }) contrast;

  # The first of `candidates` that reads at 4.5:1 on `bg`, else the one
  # that reads best
  readableOn = bg: candidates:
    let
      readable = builtins.filter (c: contrast c bg >= 4.5) candidates;
      best = builtins.foldl' (a: c: if contrast c bg > contrast a bg then c else a) (builtins.head candidates) candidates;
    in
    if readable != [ ] then builtins.head readable else best;

  # `selectedText`: the theme's own text-on-selection colour, where it has one
  themeConf = { surface, border, text, bright, selection, accent, selectedText ? null }:
    let
      onSelection = readableOn selection (lib.optional (selectedText != null) selectedText ++ [ bright text ]);
      selectedAccent = readableOn selection [ accent onSelection ];
      margin = n: ''
        Left=${toString n}
        Right=${toString n}
        Top=${toString n}
        Bottom=${toString n}'';
    in
    ''
      [Metadata]
      Name=vogix
      Version=1
      Author=vogix
      Description=The current vogix theme
      ScaleWithDPI=True

      [InputPanel]
      NormalColor=${text}
      HighlightCandidateColor=${onSelection}
      HighlightColor=${selectedAccent}
      HighlightBackgroundColor=${selection}
      CandidateLabelColor=${text}
      HighlightCandidateLabelColor=${selectedAccent}
      CandidateCommentColor=${text}
      HighlightCandidateCommentColor=${onSelection}
      PageButtonAlignment=Last Candidate

      [InputPanel/TextMargin]
      ${margin 5}

      [InputPanel/ContentMargin]
      ${margin 2}

      [InputPanel/Background]
      Color=${surface}
      BorderColor=${border}
      BorderWidth=1

      [InputPanel/Background/Margin]
      ${margin 1}

      [InputPanel/Highlight]
      Color=${selection}

      [InputPanel/Highlight/Margin]
      ${margin 5}

      [Menu]
      NormalColor=${text}
      HighlightCandidateColor=${onSelection}

      [Menu/Background]
      Color=${surface}
      BorderColor=${border}
      BorderWidth=1

      [Menu/Background/Margin]
      ${margin 1}

      [Menu/ContentMargin]
      ${margin 2}

      [Menu/Highlight]
      Color=${selection}

      [Menu/Highlight/Margin]
      ${margin 5}

      [Menu/Separator]
      Color=${border}

      [Menu/TextMargin]
      ${margin 5}
    '';

  base16Panel = colors: themeConf {
    surface = colors.base01;
    border = colors.base04;
    text = colors.base05;
    bright = colors.base07;
    selection = colors.base02;
    accent = colors.base0D;
  };
in
{
  # ~/.local/share/fcitx5/themes/vogix/theme.conf
  dataDir = "fcitx5/themes/vogix";
  configFile = "theme.conf";

  # `fcitx5-remote -r` reloads only the global config; the theme is
  # classicui's, reloaded by its addon. Nothing to do while fcitx5 is not
  # running: it reads the theme when it starts.
  reloadMethod = {
    method = "command";
    command = "if busctl --user status org.fcitx.Fcitx5 >/dev/null 2>&1; then busctl --user call org.fcitx.Fcitx5 /controller org.fcitx.Fcitx.Controller1 ReloadAddonConfig s classicui; fi";
  };

  schemes = {
    vogix16 = colors: themeConf {
      surface = colors.background-surface;
      border = colors.foreground-border;
      text = colors.foreground-text;
      bright = colors.foreground-bright;
      selection = colors.background-selection;
      accent = colors.active;
    };

    base16 = base16Panel;
    base24 = base16Panel;

    ansi16 = colors: themeConf {
      surface = colors.background;
      border = colors.color08;
      text = colors.foreground;
      bright = colors.color15;
      selection = colors.selection_bg;
      accent = colors.color12;
      selectedText = colors.selection_fg;
    };
  };
}
