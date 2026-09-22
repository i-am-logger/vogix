# The bar widget registry the shell renders from (desktop/Bar/widgets/
# registry.json), read as data: the widget names a layout may place, and
# the ones that read only horizontally. The shell's Section, these options
# and `vogix desktop check` all read that one file.
let
  inherit (builtins) fromJSON readFile attrNames filter;
  registry = fromJSON (readFile ../../../desktop/Bar/widgets/registry.json);
in
{
  inherit (registry) widgets;
  names = attrNames registry.widgets;
  horizontalOnly = filter (n: registry.widgets.${n}.horizontalOnly or false) (attrNames registry.widgets);
  # A bar places the custom cell `<name>` as `custom/<name>`.
  customPattern = "custom/[A-Za-z0-9_-]+";
}
