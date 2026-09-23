# openrgb.service as vogix.openrgb runs it, against the real OpenRGB
# server in QEMU guests (no USB, no /dev/i2c-*, loopback only: the
# server's only controllers are the Debug devices declared through
# vogix.openrgb.settings).
#
# Evaluated when the check is instantiated (so `nix flake check
# --no-build` gates it too):
# - with vogix.openrgb.enable, openrgb.service is Type=notify with
#   NotifyAccess=main, runs vogix's OpenRGB build and merges the declared
#   settings in ExecStartPre; without it, openrgb.service is not declared;
# - qmkDevices renders as settings.QMKOpenRGBDevices;
# - any server package without passthru.vogixReadiness (stock nixpkgs
#   OpenRGB) trips the readiness assertion, and a plugin wrapper of
#   vogix's build keeps the attribute.
#
# In the guests:
# - `server`: the unit reaches active through READY=1 and is listening
#   the moment a (re)start returns; the declared Debug devices are the
#   controllers the server serves, at SDK protocol 6; the settings merge
#   into OpenRGB.json recursively, keeping keys they do not set, and
#   replace a file that is not one JSON object;
# - `stock`: nixpkgs' OpenRGB under the same Type=notify unit serves the
#   SDK but never becomes active, and the start fails.
{ pkgs, self }:

let
  inherit (pkgs) lib;

  vogixOpenrgb = pkgs.callPackage ../../packages/openrgb.nix { };

  debugDevices = [
    { type = "dram"; name = "ENE DRAM"; }
    { type = "dram"; name = "ENE DRAM"; }
    { type = "keyboard"; }
  ];

  evalHost = openrgb: (pkgs.nixos [
    self.nixosModules.default
    {
      vogix.openrgb = openrgb;
      system.stateVersion = "24.11";
    }
  ]).config;

  # Only failed assertions have their messages evaluated, as NixOS does.
  readinessFails = config: builtins.any
    (a: lib.hasInfix "vogix.lib.openrgbPatched" a.message)
    (builtins.filter (a: !a.assertion) config.assertions);

  enabled = evalHost {
    enable = true;
    settings.DebugDevices.devices = debugDevices;
    qmkDevices = [{ name = "Keychron K2 HE"; vid = "0x3434"; pid = "0x0E20"; }];
  };
  unit = enabled.systemd.services.openrgb.serviceConfig;
  wired =
    unit.Type == "notify"
    && unit.NotifyAccess == "main"
    && enabled.services.hardware.openrgb.package == vogixOpenrgb
    && lib.hasPrefix "${vogixOpenrgb}/bin/openrgb " unit.ExecStart
    && builtins.length unit.ExecStartPre == 1
    && lib.hasInfix "openrgb-vogix-settings" (builtins.head unit.ExecStartPre)
    && !(readinessFails enabled);
  qmkRendered = enabled.vogix.openrgb.settings.QMKOpenRGBDevices.devices
    == [{ name = "Keychron K2 HE"; usb_vid = "0x3434"; usb_pid = "0x0E20"; }];
  notDeclared = !((evalHost { }).systemd.services ? openrgb);

  stockHost = pkgs.nixos [
    self.nixosModules.default
    {
      vogix.openrgb.enable = true;
      services.hardware.openrgb.package = pkgs.openrgb;
      system.stateVersion = "24.11";
    }
  ];
  stockRejected = readinessFails stockHost.config;
  pluginsKeepReadiness = (vogixOpenrgb.withPlugins [ ]).passthru.vogixReadiness;

  # nixpkgs' openrgb.service with the unit settings vogix.openrgb adds,
  # never started at boot. Restart=no and a start timeout make the failed
  # start observable.
  stockNode = {
    environment.systemPackages = [ pkgs.socat ];
    services.hardware.openrgb = {
      enable = true;
      package = pkgs.openrgb;
    };
    systemd.services.openrgb = {
      wantedBy = lib.mkForce [ ];
      serviceConfig = {
        Type = "notify";
        NotifyAccess = "main";
        TimeoutStartSec = "30s";
        Restart = lib.mkForce "no";
      };
    };
  };
in
assert wired || throw "vogix.openrgb.enable does not run vogix's OpenRGB build as a Type=notify, NotifyAccess=main openrgb.service with the settings merge";
assert qmkRendered || throw "vogix.openrgb.qmkDevices is not rendered as settings.QMKOpenRGBDevices";
assert notDeclared || throw "openrgb.service is declared without vogix.openrgb.enable";
assert stockRejected || throw "an OpenRGB package without passthru.vogixReadiness does not trip the readiness assertion";
assert pluginsKeepReadiness || throw "withPlugins on vogix's OpenRGB build drops passthru.vogixReadiness";
pkgs.testers.runNixOSTest {
  name = "vogix-openrgb-readiness";

  nodes.server = {
    imports = [ self.nixosModules.default ];
    environment.systemPackages = [ pkgs.socat pkgs.jq ];
    vogix.openrgb = {
      enable = true;
      settings.DebugDevices.devices = debugDevices;
    };
    # The restart loop below exceeds the default 5-starts-per-10s limit.
    systemd.services.openrgb.unitConfig.StartLimitIntervalSec = 0;
  };

  nodes.stock = stockNode;

  testScript = ''
    import shlex

    CONFIG = "/var/lib/OpenRGB/OpenRGB.json"
    MAGIC = 1111970383  # "ORGB" as a little-endian u32

    def sdk(request):
        # Sends one SDK request and prints the first reply frame's header
        # and first data word as five u32s. Everything after those 20
        # bytes is drained, so socat never writes into a closed pipe; the
        # server closes the connection when the request stream ends.
        return (
            f"printf {shlex.quote(request)} | socat -t 5 - TCP4:127.0.0.1:6742"
            " | { head -c 20 | od -An -tu4 -w20 | tr -s ' ' | sed 's/^ //'; cat > /dev/null; }"
        )

    # REQUEST_PROTOCOL_VERSION (40) offering protocol 6.
    VERSION = r"ORGB\x00\x00\x00\x00\x28\x00\x00\x00\x04\x00\x00\x00\x06\x00\x00\x00"
    # REQUEST_CONTROLLER_COUNT (0); unnegotiated, so the reply is the count alone.
    COUNT = r"ORGB\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00"

    def sdk_ping():
        return sdk(VERSION) + f" | grep -q '^{MAGIC} 0 40 4 '"

    def reply_is(request, words):
        return sdk(request) + f" | grep -qx '{MAGIC} {words}'"

    start_all()

    with subtest("openrgb.service runs vogix's OpenRGB build and reaches active through READY=1"):
        server.wait_for_unit("openrgb.service")
        t = server.succeed("systemctl show -p Type --value openrgb.service").strip()
        assert t == "notify", t
        access = server.succeed("systemctl show -p NotifyAccess --value openrgb.service").strip()
        assert access == "main", access
        exec_start = server.succeed("systemctl show -p ExecStart --value openrgb.service")
        assert "${vogixOpenrgb}/bin/openrgb" in exec_start, exec_start

    with subtest("listening as soon as a (re)start returns"):
        for _ in range(5):
            server.succeed("systemctl restart openrgb.service && ss -Hltn 'sport = :6742' | grep -q LISTEN")
            server.succeed(sdk_ping())

    with subtest("the declared Debug devices are the server's controllers, at protocol 6"):
        server.wait_until_succeeds(reply_is(COUNT, "0 0 4 3"))
        server.succeed(reply_is(VERSION, "0 40 4 6"))

    with subtest("settings merge into OpenRGB.json recursively and keep keys they do not set"):
        server.succeed(f"jq -e 'has(\"Server\") and (.DebugDevices.devices | length) == 3' {CONFIG}")
        server.succeed("systemctl stop openrgb.service")
        server.succeed(
            "jq '.Probe.kept = true | .DebugDevices.extra = 1 | .DebugDevices.devices = []'"
            f" {CONFIG} > /tmp/seeded.json && mv /tmp/seeded.json {CONFIG}"
        )
        server.succeed("systemctl start openrgb.service")
        server.succeed(
            "jq -e '.Probe.kept == true and .DebugDevices.extra == 1"
            f" and (.DebugDevices.devices | length) == 3 and has(\"Server\")' {CONFIG}"
        )
        server.wait_until_succeeds(reply_is(COUNT, "0 0 4 3"))

    with subtest("a file that is not one JSON object is replaced by the declared settings"):
        server.succeed("systemctl stop openrgb.service")
        server.succeed(f"printf '{{\"Server\": ' > {CONFIG}")
        server.succeed("systemctl start openrgb.service")
        server.succeed(
            "jq -e '(.DebugDevices.devices | length) == 3 and (has(\"Probe\") | not)"
            f" and (.DebugDevices | has(\"extra\") | not)' {CONFIG}"
        )
        server.succeed("journalctl -u openrgb.service | grep -q 'is not one JSON object'")
        server.wait_until_succeeds(reply_is(COUNT, "0 0 4 3"))

    with subtest("stock nixpkgs OpenRGB under Type=notify serves the SDK but never becomes active"):
        stock.wait_for_unit("multi-user.target")
        stock.succeed("systemctl start --no-block openrgb.service")
        stock.wait_until_succeeds("ss -Hltn 'sport = :6742' | grep -q LISTEN")
        print(stock.succeed(sdk(VERSION)))
        stock.succeed(sdk_ping())
        state = stock.succeed("systemctl show -p ActiveState --value openrgb.service").strip()
        assert state == "activating", state
        stock.fail("systemctl start openrgb.service")
        props = stock.succeed("systemctl show -p Result -p ActiveEnterTimestampMonotonic openrgb.service")
        print(props)
        assert "Result=timeout" in props, props
        assert "ActiveEnterTimestampMonotonic=0" in props, props
  '';
}
