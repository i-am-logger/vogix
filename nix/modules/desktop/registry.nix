# The bar widget registry the shell renders from (desktop/Bar/widgets/
# registry.json), read as data: the widget names a layout may place, and
# which bar orientation each renders on. The shell's Section, these
# options and `vogix desktop check` all read that one file.
let
  inherit (builtins) all attrNames elem fromJSON readFile filter listToAttrs;
  registry = fromJSON (readFile ../../../desktop/Bar/widgets/registry.json);
  names = attrNames registry.widgets;
  orientations = [ "horizontal" "vertical" ];
  # The one bar orientation a widget renders on; null, it renders on both.
  orientationOf = n: registry.widgets.${n}.orientation or null;
in
assert all (n: orientationOf n == null || elem (orientationOf n) orientations) names
  || throw "desktop/Bar/widgets/registry.json: an orientation is neither \"horizontal\" nor \"vertical\"";
{
  inherit (registry) widgets;
  inherit names;
  # The names a bar of each orientation may place: every widget the
  # registry does not confine to the other one.
  placeable = listToAttrs (map
    (o: {
      name = o;
      value = filter (n: elem (orientationOf n) [ null o ]) names;
    })
    orientations);
  # The orientation of the bar on a screen edge.
  edgeOrientation = edge: if edge == "left" || edge == "right" then "vertical" else "horizontal";
  # A bar places the custom cell `<name>` as `custom/<name>`.
  customPattern = "custom/[A-Za-z0-9_-]+";
}
