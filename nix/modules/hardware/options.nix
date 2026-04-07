{ lib, ... }:

let
  inherit (lib)
    mkOption
    mkEnableOption
    types
    ;
in
{
  options.vogix.hardware = {

    kraken-elite = {
      enable = mkEnableOption "NZXT Kraken Elite 240 RGB (240mm AIO with 2.72\" LCD + 8-LED RGB ring)";

      lcd = {
        enable = mkOption {
          type = types.bool;
          default = true;
          description = "Enable LCD screen support";
        };

        brightness = mkOption {
          type = types.ints.between 0 100;
          default = 100;
          description = "LCD screen brightness (0-100)";
        };
      };

      rgb = {
        ring.enable = mkOption {
          type = types.bool;
          default = true;
          description = "Enable RGB ring around LCD screen (8 LEDs via liquidctl)";
        };
      };

      liquidctl = {
        enable = mkOption {
          type = types.bool;
          default = true;
          description = "Install liquidctl CLI tool for manual control (fan curves, pump speed, RGB, LCD)";
        };

        autoInitialize = mkOption {
          type = types.bool;
          default = false;
          description = "Automatically run 'liquidctl initialize' on boot";
        };
      };

      monitoring = {
        enable = mkOption {
          type = types.bool;
          default = true;
          description = "Install lm_sensors for temperature and fan speed monitoring";
        };
      };
    };

    keychron-k2-he = {
      enable = mkEnableOption "Keychron K2 HE Hall Effect keyboard (vendor 3434, product 0e20)";

      udev = {
        enable = mkOption {
          type = types.bool;
          default = true;
          description = "Enable udev rules for hidraw access and DFU flashing";
        };
      };
    };

    themeApply = mkOption {
      type = types.attrsOf types.str;
      default = { };
      internal = true;
      description = "Collected reload commands from enabled hardware modules. Keys are device names, values are shell commands with {{color}} placeholders.";
    };

  };
}
