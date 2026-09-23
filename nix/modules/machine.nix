# Machine surfaces: theme state that belongs to the machine rather than to a
# user session, following one declared person, the machine owner.
#
# The owner's vogix CLI publishes its palette to
# /var/lib/vogix/machine/palette.json after every theme apply; only the
# owner can write there. Two system units apply it:
# - vogix-machine.service (root): the kernel's VT palette and the command
#   devices of vogix.hardware.devices;
# - vogix-openrgb.service: the OpenRGB devices of vogix.hardware.devices,
#   through the OpenRGB SDK server vogix.openrgb runs.
# What they apply is /etc/vogix/machine.json, rendered here and checked with
# the loader the units use (`vogix machine validate`) when the system builds.
{ config
, lib
, pkgs
, options
, ...
}:

let
  inherit (lib)
    attrNames
    concatStringsSep
    elem
    filter
    filterAttrs
    hasPrefix
    head
    literalExpression
    mkDefault
    mkIf
    mkMerge
    mkOption
    optional
    splitString
    types
    ;

  cfg = config.vogix;
  inherit (cfg) machine;
  inherit (cfg.hardware) devices;

  vogixUsers = import ./lib/vogix-users.nix { inherit config options lib; };

  dropZone = "/var/lib/vogix/machine";
  vogix = "${pkgs.vogix}/bin/vogix";

  openrgbDevices = attrNames (filterAttrs (_: device: device.provider ? openrgb) devices);
  commandDevices = filterAttrs (_: device: device.provider ? command) devices;

  surfaces = cfg.enable && machine.owner != null;
  localOwner = surfaces && (machine.console.enable || commandDevices != { });
  openrgbOwner = surfaces && openrgbDevices != [ ];
  ownerUnits =
    optional localOwner "vogix-machine.service"
    ++ optional openrgbOwner "vogix-openrgb.service";

  # The machine config's JSON: exactly the fields src/machine/config.rs
  # accepts. A device's provider is the attrTag value, which is serde's
  # externally tagged enum.
  machineJson = {
    schema = 1;
    inherit (machine) owner;
    inherit dropZone devices;
    console.enable = machine.console.enable;
    openrgb =
      if cfg.openrgb.enable then {
        host = "127.0.0.1";
        inherit (config.services.hardware.openrgb.server) port;
        inherit (cfg.openrgb.client) maxProtocol;
        clientName = "vogix";
      } else null;
  };
  machineJsonFile = config.environment.etc."vogix/machine.json".source;

  machineJsonValid = pkgs.runCommand "vogix-machine-json-valid" { } ''
    ${vogix} machine validate ${machineJsonFile}
    touch $out
  '';

  # argv[0] runs as root, so it must be immutable: a store path, with no
  # `..` that could leave the store.
  inStore = path:
    hasPrefix "${builtins.storeDir}/" path && !(elem ".." (splitString "/" path));
  misplacedPrograms = filter
    (name: !(inStore (head devices.${name}.provider.command.argv)))
    (attrNames commandDevices);
  badNames = filter (name: builtins.match "[A-Za-z0-9_-]{1,64}" name == null) (attrNames devices);

  environment = {
    RUST_LOG = "vogix=${machine.logLevel}";
  };
  kill = "${pkgs.coreutils}/bin/kill";

  sleepTargets = [
    "suspend.target"
    "hibernate.target"
    "hybrid-sleep.target"
    "suspend-then-hibernate.target"
  ];
in
{
  options.vogix.machine = {
    owner = mkOption {
      type = types.nullOr types.str;
      default = if vogixUsers != [ ] then head vogixUsers else null;
      defaultText = literalExpression ''
        the first home-manager user, by name, with programs.vogix.enable;
        null when there is none
      '';
      example = "alice";
      description = ''
        The person the machine surfaces follow: the LEDs and command devices
        of vogix.hardware.devices, the kernel's VT palette, the build-time
        console colours, the plymouth splash and the greeter. Must be a
        home-manager user with programs.vogix.enable. Only this user's theme
        applies publish a palette to /var/lib/vogix/machine; another user's
        theme stays in that user's session. Null leaves the machine
        surfaces off.
      '';
    };

    console.enable = mkOption {
      type = types.bool;
      default = true;
      description = ''
        Whether vogix-machine.service writes the owner's published console
        palette into the kernel's VT palette (every VT), before logins at
        boot and on every published change. It compares first and never
        switches VTs; a VT in graphics mode shows the palette when it next
        draws text. console.colors, from the owner's configured theme,
        stays the palette of early boot.
      '';
    };

    logLevel = mkOption {
      type = types.enum [ "error" "warn" "info" "debug" "trace" ];
      default = "info";
      description = ''
        Log verbosity of the machine owner units, rendered as
        `RUST_LOG=vogix=<level>` on each; their output is in their
        journals (`journalctl -u vogix-machine -u vogix-openrgb`).
      '';
    };
  };

  config = mkMerge [
    {
      assertions = [
        {
          assertion = machine.owner == null || elem machine.owner vogixUsers;
          message = ''
            vogix.machine.owner is "${machine.owner}", which is not a
            home-manager user with programs.vogix.enable (those are:
            ${concatStringsSep ", " vogixUsers}). The machine surfaces
            follow the palette the owner's vogix CLI publishes.
          '';
        }
        {
          assertion = devices == { } || surfaces;
          message = ''
            vogix.hardware.devices (${concatStringsSep ", " (attrNames devices)})
            show the machine owner's theme, which needs vogix.enable and a
            vogix.machine.owner (a home-manager user with
            programs.vogix.enable).
          '';
        }
        {
          assertion = badNames == [ ];
          message = ''
            vogix.hardware.devices names are 1-64 characters of
            [A-Za-z0-9_-]; these are not: ${concatStringsSep ", " badNames}.
          '';
        }
        {
          assertion = misplacedPrograms == [ ];
          message = ''
            vogix-machine.service runs command devices as root, so argv[0]
            must be a path in ${builtins.storeDir} (for example
            "''${pkgs.liquidctl}/bin/liquidctl"); it is not for
            ${concatStringsSep ", " (map (name: "${name} (${head devices.${name}.provider.command.argv})") misplacedPrograms)}.
          '';
        }
        {
          assertion = openrgbDevices == [ ] || cfg.openrgb.enable;
          message = ''
            vogix.hardware.devices ${concatStringsSep ", " openrgbDevices}
            use the openrgb provider, whose server is vogix.openrgb; it is
            disabled.
          '';
        }
      ];

      # An OpenRGB device needs the SDK server.
      vogix.openrgb.enable = mkIf (openrgbDevices != [ ]) (mkDefault true);
    }

    (mkIf surfaces {
      environment.etc."vogix/machine.json".text = builtins.toJSON machineJson;
      system.checks = [ machineJsonValid ];

      # The drop zone: only the owner can create palette.json in it, and the
      # units accept a palette.json only from the directory's owner.
      systemd.tmpfiles.rules = [
        "d /var/lib/vogix 0755 root root -"
        "d ${dropZone} 0755 ${machine.owner} ${config.users.users.${machine.owner}.group} -"
      ];
    })

    # The VT palette and the command devices. READY=1 follows the first
    # console comparison, so gettys and greeters start with the owner's
    # palette.
    (mkIf localOwner {
      systemd.services.vogix-machine = {
        description = "vogix machine surfaces: the VT palette and command devices";
        wantedBy = [ "multi-user.target" ];
        before = [ "systemd-user-sessions.service" ];
        restartTriggers = [ machineJsonFile ];
        unitConfig.RequiresMountsFor = dropZone;
        environment = environment // {
          # Command devices keep their runtime state here (liquidctl).
          XDG_RUNTIME_DIR = "/run/vogix/machine";
        };
        serviceConfig = {
          Type = "notify";
          ExecStart = "${vogix} machine serve local";
          ExecReload = "${kill} -HUP $MAINPID";
          # 78: machine.json or the drop zone is unusable; a restart cannot
          # change that.
          Restart = "on-failure";
          RestartPreventExitStatus = 78;
          RuntimeDirectory = "vogix/machine";
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
        };
      };
    })

    # The OpenRGB devices, through the SDK server. openrgb.service is active
    # exactly when its SDK sockets listen (Type=notify), so Upholds= starts
    # the owner then and BindsTo= stops it with the server; a refused
    # connection or a closed one ends the owner (exit 75), and the server's
    # next start brings it back.
    (mkIf openrgbOwner {
      systemd.services.vogix-openrgb = {
        description = "vogix OpenRGB surfaces following the machine owner's palette";
        bindsTo = [ "openrgb.service" ];
        after = [ "openrgb.service" ];
        upheldBy = [ "openrgb.service" ];
        restartTriggers = [ machineJsonFile ];
        # A switch restarts the owner after activation has installed the new
        # machine.json. Stopped before activation instead (the default),
        # Upholds= would start it again at once, reading the old file.
        stopIfChanged = false;
        inherit environment;
        unitConfig = {
          RequiresMountsFor = dropZone;
          StartLimitBurst = 3;
          StartLimitIntervalSec = 60;
        };
        serviceConfig = {
          Type = "exec";
          ExecStart = "${vogix} machine serve openrgb";
          ExecReload = "${kill} -HUP $MAINPID";
          NotifyAccess = "main";
          Restart = "no";
          RuntimeDirectory = "vogix/openrgb";
          DynamicUser = true;
          CapabilityBoundingSet = "";
          IPAddressDeny = "any";
          IPAddressAllow = "localhost";
          RestrictAddressFamilies = [
            "AF_INET"
            "AF_INET6"
            "AF_UNIX"
          ];
          ProtectSystem = "strict";
          ProtectHome = true;
          PrivateDevices = true;
          NoNewPrivileges = true;
          SystemCallFilter = "@system-service";
        };
      };
    })

    # A device may lose its colour across a sleep: after a resume the owners
    # re-apply everything (SIGHUP).
    (mkIf (ownerUnits != [ ]) {
      systemd.services.vogix-machine-resume = {
        description = "Re-apply the vogix machine surfaces after a resume";
        after = sleepTargets;
        wantedBy = sleepTargets;
        serviceConfig = {
          Type = "oneshot";
          ExecStart = "${config.systemd.package}/bin/systemctl try-reload-or-restart ${concatStringsSep " " ownerUnits}";
        };
      };
    })
  ];
}
