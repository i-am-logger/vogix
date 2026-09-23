# vogix-openrgb (`vogix machine serve openrgb`) against the real OpenRGB
# server of vogix's OpenRGB build, headless under systemd in a QEMU guest.
#
# The server's controllers come from vogix.openrgb.settings:
# - Debug devices: two "ENE DRAM" DRAM sticks and a keyboard with key and
#   underglow matrix zones (per-LED Direct mode only);
# - a DDP device named "ENE DRAM Wire" sending to 127.0.0.1:4048 (per-LED
#   Direct, brightness 100);
# - a Govee device at 127.0.0.1 (mode-specific Static, one colour,
#   brightness 100), sending its commands to UDP 4003.
# machine.json selects them with three OpenRGB devices: `dram` (every
# "ENE DRAM", Direct, base01), `govee` ("Govee", Static, base0D) and
# `keychron-k2-he`, which matches nothing. The owner and its drop zone are
# wired here directly; the NixOS module that renders them is separate.
#
# The colours are observed independently of vogix's decoder: what
# OpenRGB's own DDP and Govee drivers send on the wire, captured by socat.
#
# Gates:
# - boot: the owner applies the published palette while OpenRGB detects,
#   confirms every selected controller at protocol 6 (status.json and
#   STATUS=), and warns about the absent device only after
#   DETECTION_COMPLETE; the DDP datagrams and the Govee colour command
#   carry the palette's colours;
# - a published theme change, and five rapid ones, land as the last
#   palette;
# - SIGHUP (systemctl reload) forces a re-apply that is confirmed again;
# - `vogix machine inspect` lists the controllers read-only and captures
#   their raw payloads at protocols 6 and 5 (copied to $out/capture);
# - `systemctl restart openrgb`: the owner stops cleanly first (BindsTo),
#   and Upholds starts a new one that re-confirms; the server never
#   refused a connection;
# - `systemctl kill -s KILL openrgb`: the owner sees the connection close
#   and exits 75; openrgb's own restart and Upholds bring both back;
# - maxProtocol 5 (a specialisation; restartTriggers restart the owner):
#   the owner speaks protocol 5 to the protocol 6 server, reports `sent`,
#   and the observers show the colours.
{ pkgs, self }:

let
  inherit (pkgs) lib;

  inherit (self.packages.${pkgs.stdenv.hostPlatform.system}) vogix;

  zone = "/var/lib/vogix/machine";

  palettes = {
    nordic = { base01 = "#3b4252"; base0D = "#81a1c1"; };
    gruvbox = { base01 = "#3c3836"; base0D = "#83a598"; };
    solarized = { base01 = "#073642"; base0D = "#268bd2"; };
  };

  paletteFiles = lib.mapAttrs
    (name: slots: pkgs.writeText "palette-${name}.json" (builtins.toJSON {
      schema = 1;
      theme = { scheme = "vogix16"; inherit name; variant = "dark"; };
      inherit slots;
      console = null;
    }))
    palettes;

  machineJson = maxProtocol: builtins.toJSON {
    schema = 1;
    owner = "vogix";
    dropZone = zone;
    console.enable = false;
    openrgb = {
      host = "127.0.0.1";
      port = 6742;
      inherit maxProtocol;
      clientName = "vogix";
    };
    devices = {
      dram = {
        slot = "base01";
        provider.openrgb = { nameContains = "ENE DRAM"; mode = "Direct"; };
      };
      govee = {
        slot = "base0D";
        provider.openrgb = { nameContains = "Govee"; mode = "Static"; };
      };
      keychron-k2-he = {
        slot = "base01";
        provider.openrgb = { nameContains = "Keychron K2 HE"; mode = "Static"; };
      };
    };
  };

  # socat writes every datagram it receives, as one write, to the file.
  observer = port: {
    description = "Record the UDP datagrams OpenRGB sends to port ${toString port}";
    wantedBy = [ "multi-user.target" ];
    before = [ "openrgb.service" ];
    serviceConfig = {
      ExecStart = "${pkgs.socat}/bin/socat -u UDP4-RECV:${toString port},bind=127.0.0.1 OPEN:/run/observe-${toString port}/datagrams,creat,append";
      RuntimeDirectory = "observe-${toString port}";
    };
  };
in
pkgs.testers.runNixOSTest {
  name = "vogix-openrgb-owner";

  nodes.machine = { config, ... }: {
    imports = [ self.nixosModules.default ];

    virtualisation.memorySize = 2048;
    environment.systemPackages = [ vogix pkgs.jq pkgs.socat ];

    users.users.vogix.isNormalUser = true;

    vogix.openrgb = {
      enable = true;
      settings = {
        DebugDevices.devices = [
          { type = "dram"; name = "ENE DRAM"; }
          { type = "dram"; name = "ENE DRAM"; }
          { type = "keyboard"; keyboard = true; underglow = true; }
        ];
        DDPDevices.devices = [
          { name = "ENE DRAM Wire"; ip = "127.0.0.1"; port = 4048; num_leds = 8; }
        ];
        GoveeDevices.devices = [{ ip = "127.0.0.1"; }];
      };
    };

    environment.etc."vogix/machine.json".text = machineJson 6;

    systemd.tmpfiles.rules = [
      "d /var/lib/vogix 0755 root root -"
      "d ${zone} 0755 vogix users -"
    ];

    # The owner published nordic in an earlier session; the drop zone keeps
    # it across boots.
    systemd.services.seed-palette = {
      wantedBy = [ "multi-user.target" ];
      before = [ "openrgb.service" ];
      after = [ "systemd-tmpfiles-setup.service" ];
      serviceConfig = {
        Type = "oneshot";
        User = "vogix";
        ExecStart = [
          "${pkgs.coreutils}/bin/install -m 0644 ${paletteFiles.nordic} ${zone}/.palette.json.seed"
          "${pkgs.coreutils}/bin/mv -f ${zone}/.palette.json.seed ${zone}/palette.json"
        ];
      };
    };

    systemd.services.observe-ddp = observer 4048;
    systemd.services.observe-govee = observer 4003;

    systemd.services.vogix-openrgb = {
      description = "vogix OpenRGB surfaces following the machine owner's palette";
      bindsTo = [ "openrgb.service" ];
      after = [ "openrgb.service" ];
      upheldBy = [ "openrgb.service" ];
      restartTriggers = [ config.environment.etc."vogix/machine.json".source ];
      # A switch restarts the owner after activation has installed the new
      # machine.json. Stopped before activation instead (the default), the
      # owner would be started again at once by Upholds= and read the old file.
      stopIfChanged = false;
      environment.RUST_LOG = "vogix=debug";
      unitConfig = {
        StartLimitBurst = 3;
        StartLimitIntervalSec = 60;
      };
      serviceConfig = {
        Type = "exec";
        ExecStart = "${vogix}/bin/vogix machine serve openrgb";
        ExecReload = "${pkgs.coreutils}/bin/kill -HUP $MAINPID";
        NotifyAccess = "main";
        Restart = "no";
        RuntimeDirectory = "vogix/openrgb";
        DynamicUser = true;
        CapabilityBoundingSet = "";
        IPAddressDeny = "any";
        IPAddressAllow = "localhost";
        RestrictAddressFamilies = [ "AF_INET" "AF_INET6" "AF_UNIX" ];
        ProtectSystem = "strict";
        ProtectHome = true;
        PrivateDevices = true;
        NoNewPrivileges = true;
        SystemCallFilter = "@system-service";
      };
    };

    specialisation.protocol5.configuration = {
      environment.etc."vogix/machine.json".text = lib.mkForce (machineJson 5);
    };
  };

  testScript = ''
    import json

    ZONE = "${zone}"
    STATUS = "/run/vogix/openrgb/status.json"
    PALETTES = json.loads(r"""${builtins.toJSON palettes}""")
    FILES = json.loads(r"""${builtins.toJSON paletteFiles}""")

    def publish(*names):
        # The publisher's shape: a temp file renamed onto palette.json.
        machine.succeed(" && ".join(
            f"runuser -u vogix -- install -m 0644 {FILES[name]} {ZONE}/.palette.json.test"
            f" && runuser -u vogix -- mv -f {ZONE}/.palette.json.test {ZONE}/palette.json"
            for name in names
        ))

    def hexrgb(colour):
        return colour.lstrip("#").lower()

    def ddp_shows(colour):
        # The last DDP datagram: a 10-byte header, then R,G,B for 8 LEDs.
        return (
            "test \"$(tail -c 24 /run/observe-4048/datagrams | od -An -tx1 | tr -d ' \\n')\""
            f" = {hexrgb(colour) * 8}"
        )

    def govee_shows(colour):
        h = hexrgb(colour)
        rgb = {"r": int(h[0:2], 16), "g": int(h[2:4], 16), "b": int(h[4:6], 16)}
        return (
            "tr '\\000' '\\n' < /run/observe-4003/datagrams | grep colorwc | tail -n 1"
            f" | jq -e '.msg.data.color == {json.dumps(rgb, sort_keys=True)}'"
        )

    def status_is(theme, state, protocol):
        return (
            f"jq -e '.phase == \"ready\" and .protocol == {protocol}"
            f" and .theme.name == \"{theme}\""
            f" and .devices.dram.state == \"{state}\" and .devices.dram.controllers == 3"
            f" and .devices.govee.state == \"{state}\" and .devices.govee.controllers == 1"
            " and .devices.\"keychron-k2-he\".state == \"absent\"' " + STATUS
        )

    def settled(theme, state="confirmed", protocol=6):
        machine.wait_until_succeeds(status_is(theme, state, protocol))
        machine.wait_until_succeeds(ddp_shows(PALETTES[theme]["base01"]))
        machine.wait_until_succeeds(govee_shows(PALETTES[theme]["base0D"]))

    def invocation():
        return machine.succeed("systemctl show -p InvocationID --value vogix-openrgb.service").strip()

    def owner_log():
        return machine.succeed("journalctl -o cat -u vogix-openrgb.service")

    def count(text, needle):
        return sum(1 for line in text.splitlines() if needle in line)

    machine.start()

    with subtest("the owner applies the published palette while OpenRGB detects"):
        machine.wait_for_unit("openrgb.service")
        machine.wait_for_unit("vogix-openrgb.service")
        settled("nordic")
        log = owner_log()
        assert "OpenRGB offers SDK protocol 6; speaking protocol 6" in log, log
        assert "palette: vogix16 nordic dark" in log, log
        assert count(log, "dram: ENE DRAM") >= 2, log
        assert count(log, "dram: ENE DRAM Wire") >= 1, log
        assert "confirmed #3b4252" in log, log
        assert "confirmed #81a1c1" in log, log
        user = machine.succeed("systemctl show -p User --value vogix-openrgb.service").strip()
        print("the owner runs as", user or "a dynamic user")

    with subtest("the absent device is warned about once, after detection completes"):
        # At protocol 6 absence is a warning once per DETECTION_COMPLETE; the
        # presence transitions before it are debug lines.
        WARNING = "[WARN ] keychron-k2-he: no controller matches \"Keychron K2 HE\""
        machine.wait_until_succeeds(
            f"journalctl -o cat -u vogix-openrgb.service | grep -qF '{WARNING}'"
        )
        lines = owner_log().splitlines()
        done = [i for i, l in enumerate(lines) if "OpenRGB detection complete" in l]
        warned = [i for i, l in enumerate(lines) if l.startswith(WARNING)]
        assert len(done) == 1 and len(warned) == 1 and warned[0] > done[0], (done, warned)
        present = lines[warned[0]].split("(present: ", 1)[1].rstrip(")").split(", ")
        assert sorted(present) == sorted(["ENE DRAM Wire", "ENE DRAM", "ENE DRAM", "Debug Keyboard", "Govee "]), present

    with subtest("STATUS= and status.json carry the state"):
        text = machine.succeed("systemctl show -p StatusText --value vogix-openrgb.service").strip()
        assert text == "protocol 6; 5 controllers; dram confirmed x3; govee confirmed; keychron-k2-he absent", text
        status = json.loads(machine.succeed(f"cat {STATUS}"))
        print(json.dumps(status, indent=2))
        assert status["owner"] == "openrgb", status
        assert status["controllerCount"] == 5, status
        assert status["devices"]["keychron-k2-he"]["detail"] == "no controller matches \"Keychron K2 HE\"", status
        mode = machine.succeed(f"stat -c %a {STATUS}").strip()
        assert mode == "644", mode

    with subtest("vogix machine status reads the owner's status and exits 0"):
        # The config declares OpenRGB devices only, so vogix-openrgb is the
        # one owner unit expected; an absent device is not a problem.
        out = machine.succeed("vogix machine status")
        print(out)
        assert "owner:    vogix (drop zone /var/lib/vogix/machine)" in out, out
        assert "palette:  vogix16 nordic dark" in out, out
        assert "vogix-openrgb.service: ready" in out, out
        assert "server:   protocol 6" in out and "5 controllers" in out, out
        assert "  dram: confirmed (3 controllers)" in out, out
        assert "  govee: confirmed (1 controller)" in out, out
        assert "  keychron-k2-he: absent" in out, out
        assert "vogix-machine.service" not in out and "problems:" not in out, out

    with subtest("a published theme change lands"):
        publish("gruvbox")
        settled("gruvbox")

    with subtest("five rapid theme changes end at the last one"):
        publish("solarized", "nordic", "gruvbox", "solarized", "nordic")
        settled("nordic")

    with subtest("SIGHUP forces a re-apply that is confirmed again"):
        before = count(owner_log(), "confirmed #")
        machine.succeed("systemctl reload vogix-openrgb.service")
        machine.wait_until_succeeds(
            f"test $(journalctl -o cat -u vogix-openrgb.service | grep -c 'confirmed #') -ge {before + 4}"
        )
        assert "SIGHUP: re-reading the palette; forced re-apply of every device" in owner_log()
        settled("nordic")

    with subtest("inspect lists the controllers read-only and captures their payloads"):
        text = machine.succeed("vogix machine inspect")
        print(text)
        assert "SDK protocol 6" in text, text
        assert "Debug Keyboard" in text, text
        listed = json.loads(machine.succeed("vogix machine inspect --json"))
        names = [c["displayName"] for c in listed["controllers"]]
        assert sorted(names) == sorted(["ENE DRAM", "ENE DRAM", "Debug Keyboard", "ENE DRAM Wire", "Govee "]), names
        for c in listed["controllers"]:
            if c["displayName"].startswith("ENE DRAM"):
                assert set(c["description"]["colors"]) == {"#3b4252"}, c["description"]["colors"]
        machine.succeed("vogix machine inspect --capture /tmp/capture/v6 > /dev/null")
        machine.succeed("vogix machine inspect --max-protocol 5 --capture /tmp/capture/v5 > /dev/null")
        for version in (6, 5):
            manifest = json.loads(machine.succeed(f"cat /tmp/capture/v{version}/manifest.json"))
            assert manifest["protocol"] == version, manifest
            assert len(manifest["controllers"]) == 5, manifest
        machine.copy_from_machine("/tmp/capture")
        settled("nordic")

    with subtest("restarting openrgb stops the owner cleanly and Upholds starts a new one"):
        first = invocation()
        machine.succeed("systemctl reset-failed vogix-openrgb.service")
        machine.succeed("systemctl restart openrgb.service")
        machine.wait_until_succeeds(
            f"test \"$(systemctl show -p InvocationID --value vogix-openrgb.service)\" != {first}"
            " && systemctl is-active vogix-openrgb.service"
        )
        settled("nordic")
        journal = machine.succeed("journalctl -u vogix-openrgb.service")
        assert "stopping: closing the OpenRGB connection once it drains" in journal, journal
        assert "vogix-openrgb.service: Deactivated successfully" in journal, journal
        assert "refused" not in journal, journal

    with subtest("a killed openrgb closes the connection: the owner exits 75 and comes back"):
        second = invocation()
        machine.succeed("systemctl reset-failed vogix-openrgb.service")
        machine.succeed("systemctl kill -s KILL openrgb.service")
        machine.wait_until_succeeds(
            f"test \"$(systemctl show -p InvocationID --value vogix-openrgb.service)\" != {second}"
            " && systemctl is-active vogix-openrgb.service"
        )
        settled("nordic")
        journal = machine.succeed("journalctl -u vogix-openrgb.service")
        assert "OpenRGB closed the connection" in journal, journal
        assert "status=75/TEMPFAIL" in journal, journal
        assert "refused" not in journal, journal

    with subtest("maxProtocol 5: the owner speaks protocol 5 to the protocol 6 server"):
        third = invocation()
        machine.succeed("systemctl reset-failed vogix-openrgb.service")
        machine.succeed("/run/current-system/specialisation/protocol5/bin/switch-to-configuration test")
        machine.wait_until_succeeds(
            f"test \"$(systemctl show -p InvocationID --value vogix-openrgb.service)\" != {third}"
            " && systemctl is-active vogix-openrgb.service"
        )
        settled("nordic", state="sent", protocol=5)
        log = owner_log()
        assert "OpenRGB offers SDK protocol 6; speaking protocol 5" in log, log
        assert "sent #3b4252; protocol 5 cannot confirm it" in log, log
        assert "[INFO ] keychron-k2-he: no controller matches \"Keychron K2 HE\"" in log, log
        publish("gruvbox")
        settled("gruvbox", state="sent", protocol=5)
  '';
}
