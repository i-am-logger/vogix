{ config, lib, pkgs, ... }:

let
  inherit (lib)
    mkIf
    mkDefault
    ;

  cfg = config.vogix.hardware.keychron-k2-he;

  udevRules = pkgs.writeTextFile {
    name = "keychron-k2-he-udev-rules";
    destination = "/lib/udev/rules.d/60-keychron.rules";
    text = ''
      # Keychron K2 HE - USB HID access for OpenRGB
      SUBSYSTEMS=="usb|hidraw", ATTRS{idVendor}=="3434", ATTRS{idProduct}=="0e20", TAG+="uaccess"
      # STM32 DFU bootloader (firmware flashing)
      SUBSYSTEMS=="usb", ATTRS{idVendor}=="0483", ATTRS{idProduct}=="df11", TAG+="uaccess"
    '';
  };
in
{
  config = mkIf cfg.enable {
    # udev rules for hidraw + DFU access
    services.udev.packages = mkIf cfg.udev.enable [ udevRules ];

    # Auto-enable OpenRGB (Keychron uses QMK OpenRGB protocol)
    vogix.openrgb.enable = mkDefault true;

    # Register as QMK OpenRGB device
    vogix.openrgb.qmkDevices = [
      { name = "Keychron K2 HE"; vid = "0x3434"; pid = "0x0E20"; }
    ];

    # Theme apply: set keyboard color from vogix palette on theme change
    vogix.hardware.themeApply.keychron-k2-he = "openrgb -d 'Keychron K2 HE' -m static -c {{base01}}";
  };
}
