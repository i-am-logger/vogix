{ config, lib, pkgs, ... }:

let
  inherit (lib)
    mkIf
    optionals
    ;

  cfg = config.vogix.hardware.kraken-elite;
in
{
  config = mkIf cfg.enable {
    # Kernel module for NZXT Kraken hardware control (built into kernel since Linux 5.13)
    boot.kernelModules = [ "nzxt_kraken3" ];

    # Disable LCD at kernel level if not wanted
    boot.extraModprobeConfig = mkIf (!cfg.lcd.enable) ''
      options nzxt_kraken3 disable_lcd=1
    '';

    # Userspace control tools
    environment.systemPackages =
      (optionals cfg.liquidctl.enable [ pkgs.liquidctl ])
      ++ (optionals cfg.monitoring.enable [ pkgs.lm_sensors ]);

    # udev rules for liquidctl device access
    services.udev.packages = mkIf cfg.liquidctl.enable [ pkgs.liquidctl ];

    # The ring shows the slot's colour: vogix-machine.service runs liquidctl
    # at boot, on every published palette, and whenever the cooler's hidraw
    # node (NZXT 1e71:3012) appears again.
    vogix.hardware.devices = mkIf cfg.rgb.ring.enable {
      kraken-ring = {
        slot = cfg.rgb.ring.colorSlot;
        provider.command = {
          argv = [ "${pkgs.liquidctl}/bin/liquidctl" "--match" "kraken" "set" "ring" "color" "fixed" "{{color}}" ];
          hotplug.hidraw = {
            vendorId = "1e71";
            productId = "3012";
          };
        };
      };
    };

    # Auto-initialize on boot
    systemd.services.liquidctl-kraken-elite =
      mkIf (cfg.liquidctl.enable && cfg.liquidctl.autoInitialize)
        {
          description = "NZXT Kraken Elite 240 RGB liquidctl initialization";
          wantedBy = [ "multi-user.target" ];
          after = [ "systemd-udev-settle.service" ];
          serviceConfig = {
            Type = "oneshot";
            RemainAfterExit = true;
            ExecStart = "${pkgs.liquidctl}/bin/liquidctl initialize";
          };
        };
  };
}
