# The console palette: a theme's 16 VT colours, ANSI 0-15.
#
# The mapping is templates/<scheme>/console.palette.vogix, read here as data:
# line N names the colour of ANSI N as `{{ colors.<name> }}`, in the names
# the runtime renderer gives that scheme (vogix16's semantic names in
# snake_case; baseXX for base16/base24; colorNN for ansi16). The runtime
# renders that template; the theme packages' console/palette and the NixOS
# console.colors are built from the same lines, so the three agree.
{ lib }:

let
  vogix16Lib = import ./vogix16.nix { inherit lib; };

  schemes = [ "vogix16" "base16" "base24" "ansi16" ];

  templateFile = scheme: ../../../templates + "/${scheme}/console.palette.vogix";

  # The colour name on one template line.
  lineSlot = scheme: line:
    let
      m = builtins.match "[[:space:]]*\\{\\{[[:space:]]*colors\\.([A-Za-z0-9_]+)[[:space:]]*}}[[:space:]]*" line;
    in
    if m == null
    then throw "templates/${scheme}/console.palette.vogix: \"${line}\" is not one {{ colors.<name> }}"
    else builtins.head m;

  # The colour names of ANSI 0-15 for a scheme.
  slotsOf = scheme:
    let
      lines = builtins.filter (line: builtins.match "[[:space:]]*" line == null)
        (lib.splitString "\n" (builtins.readFile (templateFile scheme)));
      names = map (lineSlot scheme) lines;
    in
    if builtins.length names == 16
    then names
    else throw "templates/${scheme}/console.palette.vogix names ${toString (builtins.length names)} colours, not 16";

  slots = lib.genAttrs schemes slotsOf;

  # A theme variant's own colours (vogix16/base16/base24: base00-..;
  # ansi16: color00-color15 and the rest) named as the template names them.
  templateColors = scheme: colors:
    if scheme == "vogix16" then vogix16Lib.semanticColorsSnake colors else colors;

  # The 16 colours, as the colours are written ("#rrggbb"), for colours
  # already named as the template names them.
  fromTemplateColors = scheme: colors:
    map
      (name: colors.${name} or (throw "console palette: the ${scheme} colours have no ${name}"))
      (slots.${scheme} or (throw "console palette: no console template for scheme ${scheme}"));

  # The 16 colours of a theme variant's own colours.
  palette = scheme: colors: fromTemplateColors scheme (templateColors scheme colors);
in
{
  inherit slots palette fromTemplateColors;
}
