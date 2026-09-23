# vogix-machine.service's owner, `vogix machine serve local`, in a NixOS VM
# with the kernel's real VT and a QEMU xHCI controller.
#
# The node runs the owner under a unit written the way the machine owner
# unit runs (Type=notify, root, CAP_SYS_TTY_CONFIG only, closed device
# policy with /dev/tty0 and hidraw allowed, a private network with
# AF_UNIX/AF_NETLINK, a read-only file system but its runtime directory),
# fed by a machine config with the console on and one command device: a
# recorder that appends its colour argument to /run/vogix-test/cmd and is
# re-run on hidraw 0627:0001 (QEMU's usb-kbd).
#
# The palette is published by the real CLI of the declared owner, `vogix`;
# `other` is a second vogix user whose applies must not reach the machine.
# Colours are checked against sources vogix does not compute: the kernel's
# /sys/module/vt/parameters/default_{red,grn,blu}, the theme package's
# console/palette file, and the vogix16 theme data.
{ pkgs
, vogix16Themes
, home-manager
, self
,
}:

let
  inherit (pkgs) lib;

  testLib = import ./lib.nix {
    inherit pkgs home-manager self vogix16Themes;
  };

  inherit (self.packages.${pkgs.stdenv.hostPlatform.system}) vogix;

  recorder = pkgs.writeShellScript "vogix-test-recorder" ''
    printf '%s\n' "$1" >> /run/vogix-test/cmd
  '';

  machineJson = builtins.toJSON {
    schema = 1;
    owner = "vogix";
    dropZone = "/var/lib/vogix/machine";
    console.enable = true;
    openrgb = null;
    devices.probe = {
      slot = "base01";
      provider.command = {
        argv = [ "${recorder}" "{{color}}" ];
        hotplug.hidraw = {
          vendorId = "0627";
          productId = "0001";
        };
      };
    };
  };
in
pkgs.testers.nixosTest {
  name = "vogix-machine-local";

  nodes.machine = _: {
    imports = [ testLib.machineConfig ];

    virtualisation.qemu.options = [ "-device qemu-xhci,id=xhci" ];

    # No login may run before the checks: a login shell's theme refresh
    # would publish the palette.
    services.getty.autologinUser = lib.mkForce null;

    users.users.other = {
      isNormalUser = true;
      home = "/home/other";
    };
    home-manager.users.other = {
      imports = [ ../home.nix ];
      home = {
        username = lib.mkForce "other";
        homeDirectory = lib.mkForce "/home/other";
      };
    };

    environment.systemPackages = [ pkgs.jq ];
    environment.etc."vogix/machine.json".text = machineJson;

    systemd.tmpfiles.rules = [
      "d /var/lib/vogix 0755 root root -"
      "d /var/lib/vogix/machine 0755 vogix users -"
      "d /run/vogix-test 0755 root root -"
    ];

    systemd.services.vogix-machine = {
      description = "vogix machine surfaces: the VT palette and command devices";
      wantedBy = [ "multi-user.target" ];
      before = [ "systemd-user-sessions.service" ];
      unitConfig.RequiresMountsFor = "/var/lib/vogix/machine";
      serviceConfig = {
        Type = "notify";
        ExecStart = "${vogix}/bin/vogix machine serve local";
        ExecReload = "${pkgs.coreutils}/bin/kill -HUP $MAINPID";
        Restart = "on-failure";
        RestartPreventExitStatus = 78;
        RuntimeDirectory = "vogix/machine";
        Environment = [
          "RUST_LOG=vogix=info"
          "XDG_RUNTIME_DIR=/run/vogix/machine"
        ];
        ProtectSystem = "strict";
        ProtectHome = true;
        PrivateNetwork = true;
        NoNewPrivileges = true;
        CapabilityBoundingSet = "CAP_SYS_TTY_CONFIG";
        DevicePolicy = "closed";
        DeviceAllow = [
          "/dev/tty0 rw"
          "char-hidraw rw"
          "char-usb_device rw"
        ];
        RestrictAddressFamilies = [
          "AF_UNIX"
          "AF_NETLINK"
        ];
        ReadWritePaths = [ "/run/vogix-test" ];
      };
    };
  };

  testScript = ''
    import json

    all_themes = json.loads(r"""${testLib.themesJSON}""")

    zone = "/var/lib/vogix/machine"
    palette_path = f"{zone}/palette.json"
    status_path = "/run/vogix/machine/status.json"
    themes = "/home/vogix/.local/share/vogix/themes"


    def sysfs_palette():
        channels = [
            machine.succeed(f"cat /sys/module/vt/parameters/default_{c}").strip().split(",")
            for c in ("red", "grn", "blu")
        ]
        return ["#%02x%02x%02x" % (int(r), int(g), int(b)) for r, g, b in zip(*channels)]


    def theme_console(theme_variant):
        text = machine.succeed(f"cat {themes}/{theme_variant}/console/palette")
        return [line.strip().lower() for line in text.splitlines() if line.strip()]


    def status():
        return json.loads(machine.succeed(f"cat {status_path}"))


    def published():
        return json.loads(machine.succeed(f"cat {palette_path}"))


    def recorded():
        out = machine.succeed("cat /run/vogix-test/cmd 2>/dev/null || true")
        return out.split()


    def journal(unit):
        return machine.succeed(f"journalctl -b -o cat -u {unit}")


    machine.start()
    machine.wait_for_unit("multi-user.target")

    with subtest("before any publish the owner is ready and waits; the VT keeps the build-time palette"):
        machine.wait_for_unit("vogix-machine.service")
        s = status()
        assert s["phase"] == "waiting-for-palette", s
        assert s["console"]["state"] == "waiting", s
        assert s["devices"]["probe"]["state"] == "waiting", s
        machine.fail(f"test -e {palette_path}")
        assert recorded() == [], recorded()
        build_time = sysfs_palette()
        active_vt = machine.succeed("cat /sys/class/tty/tty0/active").strip()
        out = machine.succeed("vogix machine status")
        assert "palette:  none published yet" in out, out

    with subtest("the owner's theme set publishes after the commit, and the VT palette follows"):
        machine.succeed("su - vogix -c 'vogix theme set -t nordic'")
        assert machine.succeed(f"stat -c '%U %a' {palette_path}").strip() == "vogix 644"
        p = published()
        assert p["theme"]["name"] == "nordic", p
        variant = p["theme"]["variant"]
        state = machine.succeed("cat /home/vogix/.local/state/vogix/state.toml")
        assert 'current_theme = "nordic"' in state, state
        machine.wait_until_succeeds(
            f"jq -e '.theme.name == \"nordic\" and .console.state == \"confirmed\"' {status_path}"
        )
        nordic_console = theme_console(f"nordic-{variant}")
        assert nordic_console != build_time, "nordic must differ from the build-time palette"
        assert [c.lower() for c in p["console"]] == nordic_console, (p["console"], nordic_console)
        assert sysfs_palette() == nordic_console, (sysfs_palette(), nordic_console)
        assert machine.succeed("cat /sys/class/tty/tty0/active").strip() == active_vt
        assert "console: applied the vogix16 nordic " + variant + " palette" in journal("vogix-machine")

    with subtest("the command device runs with its slot's colour"):
        base01 = all_themes["nordic"][variant]["base01"].lstrip("#").lower()
        assert p["slots"]["base01"] == "#" + base01, p["slots"]
        machine.wait_until_succeeds(
            f"jq -e '.devices.probe.state == \"confirmed\"' {status_path}"
        )
        assert recorded()[-1] == base01, recorded()
        out = machine.succeed("vogix machine status")
        assert "vogix-machine.service: ready" in out, out
        assert "  probe: confirmed" in out, out
        out = machine.succeed("vogix machine validate --palette")
        assert f"probe: base01 = #{base01}" in out, out

    with subtest("republishing identical bytes is skipped"):
        inode = machine.succeed(f"stat -c %i {palette_path}").strip()
        runs = len(recorded())
        machine.succeed("su - vogix -c 'vogix theme refresh'")
        assert machine.succeed(f"stat -c %i {palette_path}").strip() == inode
        assert len(recorded()) == runs, recorded()

    with subtest("another user's apply does not reach the machine"):
        out = machine.succeed("su - other -c 'vogix theme set -t desert 2>&1'")
        assert "Machine surfaces follow 'vogix'" in out, out
        assert machine.succeed(f"stat -c %i {palette_path}").strip() == inode
        assert published()["theme"]["name"] == "nordic"
        assert sysfs_palette() == nordic_console

    with subtest("a hidraw node of the declared USB ids re-runs the device"):
        runs = len(recorded())
        machine.send_monitor_command("device_add usb-kbd,id=vk,bus=xhci.0")
        machine.wait_until_succeeds(f"test $(wc -l < /run/vogix-test/cmd) -gt {runs}")
        assert recorded()[-1] == base01, recorded()
        # QEMU's tablet carries the same ids and was present from boot; the
        # re-run names the keyboard's node.
        uevent = machine.succeed("grep -l 'HID_NAME=QEMU QEMU USB Keyboard' /sys/class/hidraw/*/device/uevent").strip()
        node = uevent.split("/")[4]
        assert f"probe: hidraw {node} (0627:0001) appeared; re-running" in journal("vogix-machine"), node

    with subtest("SIGHUP re-runs the devices and compares the console without writing it"):
        runs = len(recorded())
        machine.succeed("systemctl reload vogix-machine.service")
        machine.wait_until_succeeds(f"test $(wc -l < /run/vogix-test/cmd) -gt {runs}")
        log = journal("vogix-machine")
        assert "forced re-apply (SIGHUP)" in log, log
        assert "console: the vogix16 nordic " + variant + " palette is already current" in log, log
        assert sysfs_palette() == nordic_console

    with subtest("a palette the drop zone's owner did not write is rejected and reported"):
        machine.succeed(
            f"cp {palette_path} /tmp/p.json && install -m 0644 /tmp/p.json {zone}/.planted"
            f" && mv {zone}/.planted {palette_path}"
        )
        machine.wait_until_succeeds(f"jq -e '.phase == \"waiting-for-palette\"' {status_path}")
        assert "is owned by uid 0" in status()["detail"], status()
        rc, out = machine.execute("vogix machine status")
        assert rc == 1, (rc, out)
        assert "the published palette is rejected" in out, out
        machine.succeed("su - vogix -c 'vogix theme refresh'")
        assert machine.succeed(f"stat -c '%U' {palette_path}").strip() == "vogix"
        machine.wait_until_succeeds(f"jq -e '.phase == \"ready\"' {status_path}")
        machine.succeed("vogix machine status")

    with subtest("after a reboot the owner applies the palette before user sessions start"):
        machine.shutdown()
        machine.start()
        machine.wait_for_unit("vogix-machine.service")
        machine.wait_for_unit("systemd-user-sessions.service")
        assert sysfs_palette() == nordic_console, (sysfs_palette(), nordic_console)
        ready_at = int(machine.succeed(
            "systemctl show -p ActiveEnterTimestampMonotonic --value vogix-machine.service"
        ))
        sessions_at = int(machine.succeed(
            "systemctl show -p ActiveEnterTimestampMonotonic --value systemd-user-sessions.service"
        ))
        assert 0 < ready_at <= sessions_at, (ready_at, sessions_at)
        assert status()["console"]["state"] == "confirmed", status()
        assert machine.succeed("cat /sys/class/tty/tty0/active").strip() == active_vt
  '';
}
