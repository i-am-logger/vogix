{ config, lib, pkgs, ... }:

let
  inherit (lib)
    mkIf
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

    # OpenRGB detects the keyboard through its QMK OpenRGB protocol support.
    vogix.openrgb.qmkDevices = [
      { name = "Keychron K2 HE"; vid = "0x3434"; pid = "0x0E20"; }
    ];

    # The backlight shows the slot's colour in the keyboard's Static mode,
    # set by vogix-openrgb.service.
    vogix.hardware.devices.keychron-k2-he = {
      slot = cfg.colorSlot;
      provider.openrgb = {
        nameContains = "Keychron K2 HE";
        mode = "Static";
      };
    };
  };
}
