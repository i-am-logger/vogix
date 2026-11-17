# The host nixpkgs' quickshell with Wayland activation tokens on
# notification actions: invoking an action (a click on a card) requests an
# xdg-activation token for that input and sends ActivationToken(id, token)
# before ActionInvoked, as the Desktop Notifications spec allows, so the
# sender can bring its window forward (baud does).
#
# - 0001: upstream 3c5323b, the activation helper (in quickshell after
#   v0.3.2), author kept;
# - 0002: NotificationServer's ActivationToken signal and
#   NotificationAction::invoke() requesting the token, with its QtTest test.
#
# The series is made against v0.3.2 and applied only to it: another version
# is passed through unpatched, with a warning, rather than failing to build.
# passthru.vogixActivationToken marks the patched build. Drop this when
# quickshell and nixpkgs carry the change.
{ lib
, quickshell
}:

let
  patches = [
    ./quickshell/0001-service-tray-forward-Wayland-activation-tokens-on-pr.patch
    ./quickshell/0002-service-notifications-forward-Wayland-activation-tok.patch
  ];
in
if quickshell.version == "0.3.2" then
  quickshell.overrideAttrs
    (old: {
      patches = (old.patches or [ ]) ++ patches;
      passthru = (old.passthru or { }) // { vogixActivationToken = true; };
    })
else
  lib.warn "vogix: quickshell ${quickshell.version} is not 0.3.2, so notification actions send no activation token" quickshell
