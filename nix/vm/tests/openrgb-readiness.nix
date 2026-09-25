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
#   the moment a (re)start returns; a client that sends its version
#   request once a rescan's first DEVICE_LIST_UPDATED has reached it gets
#   the reply behind that frame, and every SDK check finds its reply by
#   packet id behind such frames; the declared Debug devices are the
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

  # Reads an SDK byte stream on stdin and prints one line per frame:
  # dev_id, pkt_id, pkt_size and the payload's first u32 ("-" when the
  # payload is shorter). Fails on a frame without the "ORGB" magic. A
  # frame the stream ends inside is not printed: when the client has
  # half-closed, the server shuts the connection down without waiting for
  # a send in progress on another thread, so the last frame can be cut
  # between its header and its payload.
  sdkFrames = pkgs.writeShellScript "sdk-frames" ''
    ${pkgs.coreutils}/bin/od -An -v -tu1 | ${pkgs.gawk}/bin/awk '
      function u32(at) { return b[at] + 256 * b[at + 1] + 65536 * b[at + 2] + 16777216 * b[at + 3] }
      { for (f = 1; f <= NF; f++) b[n++] = $f }
      END {
        for (i = 0; i < n; i += 16 + size) {
          if (i + 16 > n) exit
          if (b[i] != 79 || b[i + 1] != 82 || b[i + 2] != 71 || b[i + 3] != 66) exit 1
          size = u32(i + 12)
          if (i + 16 + size > n) exit
          print u32(i + 4), u32(i + 8), size, (size >= 4 ? u32(i + 16) : "-")
        }
      }'
  '';

  # An SDK client that connects, waits for the first frame the server
  # sends unasked, then sends REQUEST_PROTOCOL_VERSION offering protocol 6
  # and closes its sending side. socat carries the connection between two
  # FIFOs, so closing the request FIFO is what ends the request stream.
  # Every byte the server sends lands in /tmp/idle.bin; the script exits
  # with socat's status once the server closes the connection.
  idleClient = pkgs.writeShellScript "sdk-idle-client" ''
    set -eu
    dir=$(${pkgs.coreutils}/bin/mktemp -d)
    ${pkgs.coreutils}/bin/mkfifo "$dir/request" "$dir/replies"
    ${pkgs.socat}/bin/socat -t 30 - TCP4:127.0.0.1:6742 < "$dir/request" > "$dir/replies" &
    socat_pid=$!
    exec 3> "$dir/request" 4< "$dir/replies"
    ${pkgs.coreutils}/bin/dd bs=16 count=1 iflag=fullblock status=none <&4 > /tmp/idle.bin
    printf 'ORGB\x00\x00\x00\x00\x28\x00\x00\x00\x04\x00\x00\x00\x06\x00\x00\x00' >&3
    exec 3>&-
    ${pkgs.coreutils}/bin/cat <&4 >> /tmp/idle.bin
    wait "$socat_pid"
  '';

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

    FRAMES = "${sdkFrames}"

    def send(request):
        # Sends one SDK request and prints the bytes the server sends on
        # that connection. The server closes the connection once the
        # request stream ends.
        return f"printf {shlex.quote(request)} | socat -t 5 - TCP4:127.0.0.1:6742"

    def sdk(request):
        # The frames of send(request), one line each (see sdkFrames).
        return f"set -o pipefail; {send(request)} | {FRAMES}"

    # REQUEST_PROTOCOL_VERSION (40) offering protocol 6.
    VERSION = r"ORGB\x00\x00\x00\x00\x28\x00\x00\x00\x04\x00\x00\x00\x06\x00\x00\x00"
    # REQUEST_CONTROLLER_COUNT (0); unnegotiated, so the reply is the count alone.
    COUNT = r"ORGB\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00"
    # REQUEST_RESCAN_DEVICES (140).
    RESCAN = r"ORGB\x00\x00\x00\x00\x8c\x00\x00\x00\x00\x00\x00\x00"

    def reply(request, pkt_id):
        # Prints the first frame with the reply's pkt_id. The server sends
        # DEVICE_LIST_UPDATED (100) to every connected client whenever
        # detection registers a controller, negotiated or not, so other
        # frames can come before the reply.
        return sdk(request) + f" | awk '$2 == {pkt_id} && !seen++'"

    def reply_is(request, words):
        # The reply is exactly `words` (dev_id pkt_id pkt_size first_u32).
        return reply(request, words.split()[1]) + f" | grep -qx '{words}'"

    def sdk_ping():
        # A PROTOCOL_VERSION reply at any version.
        return reply(VERSION, 40) + " | grep -q '^0 40 4 '"

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

    with subtest("the version reply is found behind the device list updates of a rescan"):
        # A connected client that has not asked for anything receives a
        # DEVICE_LIST_UPDATED for each controller a detection clears or
        # registers. The idle client sends its version request only once
        # the first of them has arrived. A rescan the server ignores (one
        # is already running) or that runs before the idle client is
        # accepted leaves /tmp/idle.bin empty, and the rescan is repeated.
        server.succeed("rm -f /tmp/idle.bin && systemd-run --unit=sdk-idle -p RemainAfterExit=yes ${idleClient}")
        server.wait_until_succeeds(f"{send(RESCAN)} > /dev/null && test -s /tmp/idle.bin")
        server.wait_until_fails("systemctl show -p SubState --value sdk-idle.service | grep -qx running")
        result = server.succeed("systemctl show -p Result --value sdk-idle.service").strip()
        assert result == "success", result
        frames = server.succeed(f"{FRAMES} < /tmp/idle.bin").splitlines()
        ids = [frame.split()[1] for frame in frames]
        assert "40" in ids, frames
        version_at = ids.index("40")
        assert frames[version_at] == "0 40 4 6", frames
        assert "100" in ids[:version_at], frames
        # The first 20 bytes of that stream are not the version reply.
        server.fail(
            "head -c 20 /tmp/idle.bin | od -An -tu4 -w20 | tr -s ' ' | sed 's/^ //'"
            f" | grep -q '^{MAGIC} 0 40 4 '"
        )
        server.succeed("systemctl stop sdk-idle.service")

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
