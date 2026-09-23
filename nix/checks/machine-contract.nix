# The machine surfaces' NixOS side, checked three ways:
#
# - `contract`, at evaluation (it throws while the check is instantiated,
#   so `nix flake check --no-build` runs it): what the module renders into
#   /etc/vogix/machine.json for the three hardware modules, the owner units
#   it declares and when, the owner lookups, and the declarations it
#   refuses;
# - `validate`, a build: `vogix machine validate`, the loader the owner
#   units run, accepts the rendered machine.json and a palette the owner's
#   CLI published, and rejects either one with a field its schema lacks;
# - `console`, a build: for a theme of each scheme, the machine owner's
#   evaluated console.colors, the console/palette their theme package ships
#   and `vogix theme refresh`'s render of the scheme's console template are
#   one palette.
{ pkgs
, nixpkgs
, home-manager
, self
, unfreePackageNames
, schemeSources
}:

let
  inherit (pkgs) lib;
  inherit (pkgs.stdenv.hostPlatform) system;

  # `selection` is a theme name, at its night variant, or { theme; variant; }.
  hmUser = name: selection:
    let
      inherit (if builtins.isString selection then { theme = selection; variant = "night"; } else selection) theme variant;
    in
    {
      imports = [ self.homeManagerModules.default ];
      home = {
        username = name;
        homeDirectory = "/home/${name}";
        stateVersion = "24.11";
      };
      programs.vogix = {
        enable = true;
        appearance = {
          inherit theme variant;
          prebuiltThemes = [ theme ];
        };
        enableDaemon = false;
      };
    };

  # A host with home-manager, whose vogix users are `users` (name → theme).
  host = { users ? { t = "yoga"; }, extra ? { } }:
    nixpkgs.lib.nixosSystem {
      modules = [
        self.nixosModules.default
        home-manager.nixosModules.home-manager
        extra
        {
          nixpkgs.hostPlatform = system;
          nixpkgs.overlays = [ self.overlays.default ];
          nixpkgs.config.allowUnfreePredicate = pkg: builtins.elem (lib.getName pkg) unfreePackageNames;
          vogix.enable = true;
          fileSystems."/" = { device = "/dev/vda"; fsType = "ext4"; };
          boot.loader.grub.enable = false;
          users.users = lib.mapAttrs (_: _: { isNormalUser = true; }) users;
          home-manager.users = lib.mapAttrs hmUser users;
          system.stateVersion = "24.11";
        }
      ];
    };

  # A host without home-manager, for the declarations the module refuses.
  bare = extra: (pkgs.nixos [
    self.nixosModules.default
    extra
    {
      users.users.u.isNormalUser = true;
      system.stateVersion = "24.11";
    }
  ]).config;

  # Only failed assertions have their messages evaluated, as NixOS does.
  refuses = needle: config: builtins.any
    (a: lib.hasInfix needle a.message)
    (builtins.filter (a: !a.assertion) config.assertions);
  noneFail = config: builtins.all (a: a.assertion) config.assertions;
  evaluates = config: (builtins.tryEval (builtins.deepSeq config.vogix.hardware.devices config.system.stateVersion)).success;

  full = host {
    users = { a = "desert"; t = "yoga"; };
    extra = {
      vogix.machine.owner = "t";
      vogix.hardware = {
        dram-rgb.enable = true;
        keychron-k2-he.enable = true;
        kraken-elite.enable = true;
      };
    };
  };
  cfg = full.config;
  # fromJSON refuses store-path context (the command device's argv carries it).
  readJson = config: builtins.fromJSON (builtins.unsafeDiscardStringContext config.environment.etc."vogix/machine.json".text);
  machineJson = readJson cfg;
  machineJsonFile = cfg.environment.etc."vogix/machine.json".source;
  liquidctl = "${full.pkgs.liquidctl}/bin/liquidctl";

  expectedJson = {
    schema = 1;
    owner = "t";
    dropZone = "/var/lib/vogix/machine";
    console.enable = true;
    openrgb = {
      host = "127.0.0.1";
      port = 6742;
      maxProtocol = 6;
      clientName = "vogix";
    };
    devices = {
      dram-rgb = {
        slot = "base01";
        provider.openrgb = { nameContains = "ENE DRAM"; mode = "Static"; };
      };
      keychron-k2-he = {
        slot = "base0D";
        provider.openrgb = { nameContains = "Keychron K2 HE"; mode = "Static"; };
      };
      kraken-ring = {
        slot = "base01";
        provider.command = {
          argv = [ liquidctl "--match" "kraken" "set" "ring" "color" "fixed" "{{color}}" ];
          hotplug.hidraw = { vendorId = "1e71"; productId = "3012"; };
        };
      };
    };
  };

  inherit (cfg.systemd) services;
  openrgbOwner = services.vogix-openrgb;
  localOwner = services.vogix-machine;
  resume = services.vogix-machine-resume;
  sleepTargets = [ "suspend.target" "hibernate.target" "hybrid-sleep.target" "suspend-then-hibernate.target" ];
  hasSuffix = suffix: value: lib.hasSuffix suffix (toString value);

  rendered = noneFail cfg && machineJson == expectedJson;

  openrgbWired =
    openrgbOwner.bindsTo == [ "openrgb.service" ]
    && openrgbOwner.after == [ "openrgb.service" ]
    && openrgbOwner.wantedBy == [ "openrgb.service" ]
    && openrgbOwner.upheldBy == [ ]
    && !openrgbOwner.stopIfChanged
    && openrgbOwner.restartTriggers == [ machineJsonFile ]
    && openrgbOwner.unitConfig.RequiresMountsFor == "/var/lib/vogix/machine"
    && openrgbOwner.unitConfig.StartLimitBurst == 3
    && openrgbOwner.unitConfig.StartLimitIntervalSec == 60
    && openrgbOwner.environment.RUST_LOG == "vogix=info"
    && (with openrgbOwner.serviceConfig;
    Type == "exec"
    && hasSuffix "/bin/vogix machine serve openrgb" ExecStart
    && NotifyAccess == "main"
    && Restart == "on-failure"
    && RestartPreventExitStatus == 78
    && RuntimeDirectory == "vogix/openrgb"
    && DynamicUser
    && CapabilityBoundingSet == ""
    && IPAddressDeny == "any"
    && IPAddressAllow == "localhost"
    && RestrictAddressFamilies == [ "AF_INET" "AF_INET6" "AF_UNIX" ]
    && ProtectSystem == "strict"
    && ProtectHome
    && PrivateDevices
    && NoNewPrivileges
    && SystemCallFilter == "@system-service")
    && cfg.systemd.services.openrgb.serviceConfig.Type == "notify"
    && cfg.systemd.services.openrgb.serviceConfig.NotifyAccess == "main"
    && cfg.services.hardware.openrgb.package.passthru.vogixReadiness;

  localWired =
    localOwner.wantedBy == [ "multi-user.target" ]
    && localOwner.before == [ "systemd-user-sessions.service" ]
    && localOwner.restartTriggers == [ machineJsonFile ]
    && localOwner.unitConfig.RequiresMountsFor == "/var/lib/vogix/machine"
    && localOwner.environment.XDG_RUNTIME_DIR == "/run/vogix/machine"
    && (with localOwner.serviceConfig;
    Type == "notify"
    && hasSuffix "/bin/vogix machine serve local" ExecStart
    && Restart == "on-failure"
    && RestartPreventExitStatus == 78
    && RuntimeDirectory == "vogix/machine"
    && ProtectSystem == "strict"
    && ProtectHome
    && PrivateNetwork
    && NoNewPrivileges
    && CapabilityBoundingSet == "CAP_SYS_TTY_CONFIG"
    && DevicePolicy == "closed"
    && DeviceAllow == [ "/dev/tty0 rw" "char-hidraw rw" "char-usb_device rw" ]
    && RestrictAddressFamilies == [ "AF_UNIX" "AF_NETLINK" ]);

  # Both owner units act on machine::exit::OwnerExit alike: a restart after
  # any failure but a 78 (EX_CONFIG).
  restartAlike = builtins.all
    (unit: unit.serviceConfig.Restart == "on-failure" && unit.serviceConfig.RestartPreventExitStatus == 78)
    [ localOwner openrgbOwner ];

  resumeWired =
    resume.wantedBy == sleepTargets
    && resume.after == sleepTargets
    && resume.serviceConfig.Type == "oneshot"
    && hasSuffix "/bin/systemctl try-reload-or-restart vogix-machine.service vogix-openrgb.service" resume.serviceConfig.ExecStart;

  dropZone =
    builtins.elem "d /var/lib/vogix/machine 0755 t users -" cfg.systemd.tmpfiles.rules
    && builtins.elem "d /var/lib/vogix 0755 root root -" cfg.systemd.tmpfiles.rules;

  # The build-time check runs the Rust loader on this very file.
  validatedAtBuild = builtins.any
    (drv: lib.getName drv == "vogix-machine-json-valid"
      && lib.hasInfix (builtins.unsafeDiscardStringContext "${machineJsonFile}") (builtins.unsafeDiscardStringContext drv.buildCommand))
    cfg.system.checks;

  # A cross-built system runs that check with a vogix the build platform
  # executes, not the target's.
  crossTarget = if system == "x86_64-linux" then "aarch64-linux" else "x86_64-linux";
  crossBuilt = host {
    extra.nixpkgs = {
      hostPlatform = lib.mkForce crossTarget;
      buildPlatform = system;
    };
  };
  crossCheck = lib.findFirst (drv: lib.getName drv == "vogix-machine-json-valid") null crossBuilt.config.system.checks;
  plainText = builtins.unsafeDiscardStringContext;
  validatedOnBuildPlatform =
    crossBuilt.pkgs.stdenv.hostPlatform.system == crossTarget
    && crossCheck != null
    && crossCheck.system == system
    && lib.hasInfix (plainText "${crossBuilt.pkgs.buildPackages.vogix}/bin/vogix machine validate") (plainText crossCheck.buildCommand)
    && !(lib.hasInfix (plainText "${crossBuilt.pkgs.vogix}") (plainText crossCheck.buildCommand));

  # The owner: by default the first vogix user by name; the console
  # colours follow whoever it is.
  ownerFollowed =
    (host { users = { a = "desert"; t = "yoga"; }; }).config.vogix.machine.owner == "a"
    && cfg.console.colors == plain.console.colors
    && cfg.console.colors != (host { users = { a = "desert"; t = "yoga"; }; extra.vogix.machine.owner = "a"; }).config.console.colors;

  # maxProtocol and the server port reach machine.json.
  tuned = readJson (host {
    extra = {
      vogix.hardware.dram-rgb.enable = true;
      vogix.openrgb.client.maxProtocol = 5;
      services.hardware.openrgb.server.port = 6800;
    };
  }).config;
  endpointFollows = tuned.openrgb == { host = "127.0.0.1"; port = 6800; maxProtocol = 5; clientName = "vogix"; };

  # Each owner unit exists exactly when it has something to own.
  plain = (host { }).config;
  commandOnly = (host { extra = { vogix.hardware.kraken-elite.enable = true; vogix.machine.console.enable = false; }; }).config;
  nothingToOwn = (host { extra.vogix.machine.console.enable = false; }).config;
  noOwner = (host { users = { }; }).config;
  unitsFollowDevices =
    plain.systemd.services ? vogix-machine
    && !(plain.systemd.services ? vogix-openrgb)
    && !(plain.systemd.services ? openrgb)
    && (readJson plain).openrgb == null
    && commandOnly.systemd.services ? vogix-machine
    && !(commandOnly.systemd.services ? vogix-openrgb)
    && hasSuffix "try-reload-or-restart vogix-machine.service" commandOnly.systemd.services.vogix-machine-resume.serviceConfig.ExecStart
    && !(nothingToOwn.systemd.services ? vogix-machine)
    && !(nothingToOwn.systemd.services ? vogix-machine-resume)
    && nothingToOwn.environment.etc ? "vogix/machine.json"
    && noOwner.vogix.machine.owner == null
    && !(noOwner.environment.etc ? "vogix/machine.json")
    && !(noOwner.systemd.services ? vogix-machine)
    && noneFail noOwner;

  # What the module refuses.
  commandDevice = argv: { vogix.hardware.devices.probe = { slot = "base01"; provider.command = { inherit argv; }; }; };
  refused = {
    unpatchedServer = refuses "vogix.lib.openrgbPatched" (bare {
      vogix.hardware.dram-rgb.enable = true;
      services.hardware.openrgb.package = pkgs.openrgb;
    });
    relativeProgram = refuses "must be a path in ${builtins.storeDir}" (bare (commandDevice [ "liquidctl" "{{color}}" ]));
    programOutsideStore = refuses "must be a path in ${builtins.storeDir}" (bare (commandDevice [ "/run/current-system/sw/bin/liquidctl" "{{color}}" ]));
    programLeavingStore = refuses "must be a path in ${builtins.storeDir}" (bare (commandDevice [ "${builtins.storeDir}/../tmp/liquidctl" "{{color}}" ]));
    ownerNotVogixUser = refuses "vogix.machine.owner is \"u\", which is not a" (host { extra = { users.users.u.isNormalUser = true; vogix.machine.owner = "u"; }; }).config;
    devicesWithoutVogix = refuses "show the machine owner's theme" (bare { vogix.hardware.dram-rgb.enable = true; });
    openrgbServerOff = refuses "use the openrgb provider" (bare { vogix.hardware.dram-rgb.enable = true; vogix.openrgb.enable = false; });
    badDeviceName = refuses "these are not: probe.1" (bare { vogix.hardware.devices."probe.1" = { slot = "base01"; provider.openrgb = { nameContains = "x"; mode = "Static"; }; }; });
    themeApplyRemoved = !(evaluates (bare { vogix.hardware.themeApply.dram-rgb = "true"; }));
    twoProviders = !(evaluates (bare {
      vogix.hardware.devices.probe = {
        slot = "base01";
        provider = { openrgb = { nameContains = "x"; mode = "Static"; }; command.argv = [ storeProgram ]; };
      };
    }));
    badSlot = !(evaluates (bare { vogix.hardware.devices.probe = { slot = "base 01"; provider.openrgb = { nameContains = "x"; mode = "Static"; }; }; }));
    badUsbId = !(evaluates (bare (lib.recursiveUpdate (commandDevice [ storeProgram ]) { vogix.hardware.devices.probe.provider.command.hotplug.hidraw = { vendorId = "1E71"; productId = "3012"; }; })));
  };
  refusedFailures = builtins.attrNames (lib.filterAttrs (_: ok: !ok) refused);

  # The same declarations without the defect are accepted, so each refusal
  # above is its defect's.
  storeProgram = "${pkgs.coreutils}/bin/true";
  accepted = {
    patchedServer = !(refuses "vogix.lib.openrgbPatched" (bare { vogix.hardware.dram-rgb.enable = true; }));
    programInStore = !(refuses "must be a path in" (bare (commandDevice [ storeProgram "{{color}}" ])));
    vogixOwner = !(refuses "vogix.machine.owner is" (host { extra.vogix.machine.owner = "t"; }).config);
    devicesWithVogix = !(refuses "show the machine owner's theme" (host { extra.vogix.hardware.dram-rgb.enable = true; }).config);
    openrgbServerOn = !(refuses "use the openrgb provider" (bare { vogix.hardware.dram-rgb.enable = true; }));
    goodDeviceName = !(refuses "these are not:" (bare { vogix.hardware.devices.probe_1 = { slot = "base01"; provider.openrgb = { nameContains = "x"; mode = "Static"; }; }; }));
    wellTyped = evaluates (bare (lib.recursiveUpdate (commandDevice [ storeProgram ]) {
      vogix.hardware.devices.probe.provider.command.hotplug.hidraw = { vendorId = "1e71"; productId = "3012"; };
    }));
  };
  acceptedFailures = builtins.attrNames (lib.filterAttrs (_: ok: !ok) accepted);

  contract =
    assert rendered || throw "machine.json for dram-rgb, keychron-k2-he and kraken-elite is not the expected document: ${builtins.toJSON machineJson}";
    assert openrgbWired || throw "vogix-openrgb.service or openrgb.service is not wired as the machine module specifies";
    assert localWired || throw "vogix-machine.service is not wired as the machine module specifies";
    assert restartAlike || throw "vogix-machine.service and vogix-openrgb.service do not both restart after a failure but a 78";
    assert resumeWired || throw "vogix-machine-resume.service does not reload both owners after every sleep";
    assert dropZone || throw "the drop zone is not a tmpfiles directory owned by the machine owner";
    assert validatedAtBuild || throw "the system build does not validate machine.json with the Rust loader";
    assert validatedOnBuildPlatform || throw "a system cross-built for ${crossTarget} on ${system} validates machine.json with a vogix the build platform cannot run";
    assert ownerFollowed || throw "the machine owner default or the owner-following console colours are wrong";
    assert endpointFollows || throw "vogix.openrgb.client.maxProtocol or the OpenRGB server port does not reach machine.json";
    assert unitsFollowDevices || throw "the machine owner units do not exist exactly when they have something to own";
    assert refusedFailures == [ ] || throw "the machine module accepted declarations it must refuse: ${toString refusedFailures}";
    assert acceptedFailures == [ ] || throw "the machine module refused declarations it must accept: ${toString acceptedFailures}";
    pkgs.runCommand "vogix-machine-contract" { } ''
      echo "machine.json, owner units, owner lookups and refusals hold"
      touch $out
    '';

  inherit (self.packages.${system}) vogix;

  # The VT palette the system is built with is the one the runtime applies,
  # for a theme of each scheme: the machine owner's evaluated console.colors
  # equals the console/palette their theme package ships (what the owner's
  # CLI publishes and vogix-machine applies) and the runtime's own render of
  # templates/<scheme>/console.palette.vogix by `vogix theme refresh`.
  consoleThemes = {
    vogix16 = { theme = "nordic"; variant = "night"; };
    base16 = { theme = "dracula"; variant = "default"; };
    base24 = { theme = "argonaut"; variant = "default"; };
    ansi16 = { theme = "aardvark-blue"; variant = "default"; };
  };
  consoleCase = scheme: selection:
    let
      inherit (selection) theme variant;
      hostConfig = (host { users.t = selection; }).config;
      themeVariant = "${theme}-${variant}";
      # config.toml's parts the refresh reads: the default theme, its
      # scheme, the templates and the theme sources.
      manifest = pkgs.writeText "vogix-${themeVariant}-config.toml" ''
        [default]
        theme = "${theme}"
        variant = "${variant}"

        [templates]
        path = "${../../templates}"
        hash = "console-palette"

        [theme_sources]
        ${lib.concatStrings (lib.mapAttrsToList (name: path: "${name} = \"${path}\"\n") schemeSources)}
        [themes."${theme}"]
        scheme = "${scheme}"
        variants = ["${variant}"]
      '';
    in
    ''
      check ${scheme} ${theme} ${variant} \
        '${lib.concatStringsSep " " hostConfig.console.colors}' \
        ${hostConfig.home-manager.users.t.xdg.dataFile."vogix/themes/${themeVariant}".source} \
        ${manifest}
    '';

  console = pkgs.runCommand "vogix-console-palette" { nativeBuildInputs = [ vogix ]; } ''
    # scheme, theme, variant, console.colors, theme package, config.toml
    check() {
      scheme=$1 theme=$2 variant=$3 colors=$4 package=$5 manifest=$6
      name="$scheme $theme-$variant"
      home="$PWD/$theme-$variant"
      mkdir -p "$home/.local/state/vogix" "$home/.local/share/vogix/themes" "$home/.cache"
      cp "$manifest" "$home/.local/state/vogix/config.toml"
      ln -s "$package" "$home/.local/share/vogix/themes/$theme-$variant"
      HOME="$home" XDG_STATE_HOME="$home/.local/state" XDG_DATA_HOME="$home/.local/share" \
        XDG_CACHE_HOME="$home/.cache" vogix theme refresh --quiet

      # shellcheck disable=SC2086 # one word per colour
      printf '#%s\n' $colors | tr A-F a-f > "$home/evaluated"
      grep -v '^$' "$package/console/palette" | tr A-F a-f > "$home/package"
      grep -v '^$' "$home/.cache/vogix/themes/console-palette/$scheme/$theme/$variant/console.palette" \
        | tr A-F a-f > "$home/rendered"
      test "$(wc -l < "$home/package")" -eq 16
      agree=1
      if ! diff -u "$home/package" "$home/evaluated"; then
        echo "$name: console.colors is not the theme package's console/palette"
        agree=0
      fi
      if ! diff -u "$home/package" "$home/rendered"; then
        echo "$name: the runtime render of the console template is not the theme package's console/palette"
        agree=0
      fi
      if [ "$agree" = 1 ]; then
        echo "$name: console.colors, the theme package and the runtime render agree"
      else
        failed="$failed $scheme"
      fi
    }
    failed=""

    ${lib.concatStrings (lib.mapAttrsToList consoleCase consoleThemes)}
    if [ -n "$failed" ]; then
      echo "the console palettes disagree for:$failed"
      exit 1
    fi
    touch $out
  '';

  validate = pkgs.runCommand "vogix-machine-contract-validate"
    {
      nativeBuildInputs = [ vogix pkgs.jq ];
      machineJson = machineJsonFile;
      palette = ../../tests/fixtures/machine/palette.json;
      buildTimeCheck = lib.findFirst (drv: lib.getName drv == "vogix-machine-json-valid") (throw "no machine.json check in system.checks") cfg.system.checks;
    } ''
    rejects() {
      if vogix machine validate "$@" > rejected.out 2>&1; then
        echo "vogix machine validate accepted $*:"; cat rejected.out; exit 1
      fi
      cat rejected.out
    }

    # The rendered machine config, as the owner units load it.
    vogix machine validate "$machineJson" | tee config.out
    grep -qxF '  owner:     t' config.out
    grep -qxF '  openrgb:   127.0.0.1:6742, protocol up to 6, client name "vogix"' config.out
    grep -qxF '  owners:    vogix-openrgb.service, vogix-machine.service' config.out
    grep -qxF '    dram-rgb: slot base01, openrgb, name contains "ENE DRAM", mode "Static"' config.out
    grep -qxF '    keychron-k2-he: slot base0D, openrgb, name contains "Keychron K2 HE", mode "Static"' config.out
    grep -qxF '    kraken-ring: slot base01, command ${liquidctl} with 7 argument(s), re-run on hidraw 1e71:3012' config.out

    # The loader checks the schema: one field it does not know is refused.
    jq '.devices."kraken-ring".provider.command.hotplug.usb = null' "$machineJson" > unknown-field.json
    rejects unknown-field.json

    # A palette the owner's CLI published (tests/fixtures/machine), checked
    # on the drop-zone terms: in a directory owned by the file's owner.
    mkdir zone
    cp "$palette" zone/palette.json
    vogix machine validate --palette zone/palette.json | tee palette.out
    grep -qxF '  console: 16 colours' palette.out
    jq '.console |= .[1:]' "$palette" > zone/short.json
    rejects --palette zone/short.json

    touch $out
  '';
in
{
  inherit contract validate console;
}
