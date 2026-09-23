{ config, lib, pkgs, ... }:

let
  inherit (lib)
    literalExpression
    mkDefault
    mkIf
    mkMerge
    mkOption
    types
    ;

  cfg = config.vogix.openrgb;
  server = config.services.hardware.openrgb;

  jsonFormat = pkgs.formats.json { };
  jq = lib.getExe pkgs.jq;

  settingsFile = jsonFormat.generate "openrgb-vogix-settings.json" cfg.settings;

  # openrgb.service runs with no HOME, so OpenRGB reads OpenRGB.json from
  # its working directory; ExecStartPre runs in that same directory. The
  # file is merged recursively (jq `*`: objects merge, every other value,
  # arrays included, is replaced). OpenRGB discards a file that is not one
  # JSON object, so such a file is replaced by the declared settings.
  mergeSettings = pkgs.writeShellScript "openrgb-vogix-settings" ''
    set -eu
    umask 022
    config="$PWD/OpenRGB.json"
    tmp="$config.vogix.tmp"
    if [ -e "$config" ] && ${jq} -se 'length == 1 and (.[0] | type) == "object"' "$config" > /dev/null; then
      ${jq} -s '.[0] * .[1]' "$config" ${settingsFile} > "$tmp"
    else
      if [ -e "$config" ]; then
        echo "$config is not one JSON object; OpenRGB discards such a file, so the declared settings replace it" >&2
      fi
      ${jq} . ${settingsFile} > "$tmp"
    fi
    mv -f "$tmp" "$config"
  '';
in
{
  options.vogix.openrgb = {
    enable = mkOption {
      type = types.bool;
      default = false;
      description = ''
        Run the OpenRGB SDK server (services.hardware.openrgb) for the RGB
        devices vogix drives. openrgb.service runs as Type=notify, so it is
        active exactly when its SDK port accepts connections, and its
        package defaults to vogix's OpenRGB build, which sends that
        notification. Hardware modules that drive OpenRGB devices enable it.
      '';
    };

    client.maxProtocol = mkOption {
      type = types.enum [ 5 6 ];
      default = 6;
      description = ''
        Highest OpenRGB SDK protocol version the vogix client offers the
        server; the session runs at the lower of this and the server's
        version. 5 is the protocol of the OpenRGB 1.0 release candidates;
        6 is that of vogix's OpenRGB build.
      '';
    };

    settings = mkOption {
      type = types.attrsOf jsonFormat.type;
      default = { };
      example = literalExpression ''
        {
          DebugDevices.devices = [ { type = "dram"; name = "ENE DRAM"; } ];
        }
      '';
      description = ''
        OpenRGB settings, keyed by top-level OpenRGB.json section, merged
        into the OpenRGB.json that openrgb.service reads before every
        start. Objects merge recursively; any other value set here, arrays
        included, replaces the file's. Keys not set here keep what the file
        holds: OpenRGB's own settings, and keys an earlier configuration
        wrote. A file that is not one JSON object, which OpenRGB would
        discard, is replaced by these settings. qmkDevices is rendered here
        as QMKOpenRGBDevices.
      '';
    };

    qmkDevices = mkOption {
      type = types.listOf (types.submodule {
        options = {
          name = mkOption {
            type = types.str;
            description = "Device name for OpenRGB display";
          };
          vid = mkOption {
            type = types.str;
            description = "USB Vendor ID (e.g. \"0x3434\")";
          };
          pid = mkOption {
            type = types.str;
            description = "USB Product ID (e.g. \"0x0E20\")";
          };
        };
      });
      default = [ ];
      description = "QMK keyboards with OpenRGB firmware support. Hardware modules append to this list.";
    };
  };

  config = mkIf cfg.enable (mkMerge [
    {
      # Base OpenRGB SDK server. The SMBus/i2c specifics (chipset type, i2c
      # drivers, acpi_enforce_resources) live in the dram-rgb module, so
      # USB-only consumers like the keychron keyboard don't drag in
      # DRAM/SMBus dependencies they never use.
      services.hardware.openrgb = {
        enable = true;
        package = mkDefault (pkgs.callPackage ../packages/openrgb.nix { });
      };

      # The server's main process sends READY=1 once every SDK socket
      # listens, so units ordered after openrgb.service can connect.
      systemd.services.openrgb.serviceConfig = {
        Type = "notify";
        NotifyAccess = "main";
      };

      assertions = [{
        assertion = server.package.passthru.vogixReadiness or false;
        message = ''
          vogix.openrgb runs openrgb.service as Type=notify, which only an
          OpenRGB build that notifies systemd once its SDK server listens
          can satisfy; any other server never becomes active. Leave
          services.hardware.openrgb.package at its default, or set it to
          vogix.lib.openrgbPatched pkgs (passthru.vogixReadiness).
        '';
      }];

      vogix.openrgb.settings = mkIf (cfg.qmkDevices != [ ]) {
        QMKOpenRGBDevices.devices = map
          (dev: {
            inherit (dev) name;
            usb_vid = dev.vid;
            usb_pid = dev.pid;
          })
          cfg.qmkDevices;
      };
    }

    (mkIf (cfg.settings != { }) {
      systemd.services.openrgb.serviceConfig.ExecStartPre = [ "${mergeSettings}" ];
    })
  ]);
}
