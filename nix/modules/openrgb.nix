{ config, lib, pkgs, ... }:

let
  inherit (lib)
    mkIf
    mkOption
    types
    ;

  cfg = config.vogix.openrgb;

  qmkDevicesJson =
    if cfg.qmkDevices != [ ] then {
      devices = map
        (dev: {
          inherit (dev) name;
          usb_vid = dev.vid;
          usb_pid = dev.pid;
        })
        cfg.qmkDevices;
    } else null;

  serverConfig = pkgs.writeText "openrgb-server-config.json" (builtins.toJSON {
    QMKOpenRGBDevices = qmkDevicesJson;
  });
in
{
  options.vogix.openrgb = {
    enable = mkOption {
      type = types.bool;
      default = false;
      description = "Enable OpenRGB service for hardware RGB control. Auto-enabled by hardware modules that need it.";
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

  config = mkIf cfg.enable {
    # Base OpenRGB SDK server. The SMBus/i2c specifics (chipset type, i2c
    # drivers, acpi_enforce_resources) live in the dram-rgb module, so USB-only
    # consumers like the keychron keyboard don't drag in DRAM/SMBus dependencies
    # they never use.
    services.hardware.openrgb.enable = true;

    # Merge QMK device list into OpenRGB server config before starting
    systemd.services.openrgb = mkIf (cfg.qmkDevices != [ ]) {
      serviceConfig.ExecStartPre = "${pkgs.writeShellScript "openrgb-qmk-setup" ''
        CONFIG="/var/lib/OpenRGB/OpenRGB.json"
        if [ -f "$CONFIG" ]; then
          ${pkgs.jq}/bin/jq --argjson qmk '${builtins.toJSON qmkDevicesJson}' '.QMKOpenRGBDevices = $qmk' "$CONFIG" > "$CONFIG.tmp" && mv "$CONFIG.tmp" "$CONFIG"
        else
          cp ${serverConfig} "$CONFIG"
        fi
      ''}";
    };
  };
}
