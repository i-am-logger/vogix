{ config, lib, pkgs, ... }:

let
  inherit (lib)
    mkIf
    mkOption
    types
    ;

  cfg = config.vogix.openrgb;
  inherit (config.vogix.hardware) motherboard;

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
    # OpenRGB service with auto-detected motherboard type
    services.hardware.openrgb = {
      enable = true;
    } // lib.optionalAttrs (motherboard != null) {
      inherit motherboard;
    };

    # SMBus access for DDR5 RGB, motherboard RGB, GPU RGB
    boot.kernelModules = [ "i2c-dev" "i2c-piix4" ];
    boot.kernelParams = [ "acpi_enforce_resources=lax" ];

    # Theme apply: set all DRAM devices to monochromatic base
    vogix.hardware.themeApply.dram-rgb = "openrgb -d 'ENE DRAM' -m static -c {{base01}}";

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
