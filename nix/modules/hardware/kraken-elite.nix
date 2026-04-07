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

    # Theme apply: set ring colors from vogix palette on theme change
    vogix.hardware.themeApply = lib.optionalAttrs cfg.rgb.ring.enable {
      kraken-ring = "liquidctl --match kraken set ring color fixed {{base01}}";
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
