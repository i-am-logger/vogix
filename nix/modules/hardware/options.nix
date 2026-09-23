{ lib, ... }:

let
  inherit (lib)
    literalExpression
    mkOption
    mkEnableOption
    types
    ;

  # The value types of /etc/vogix/machine.json, as its Rust loader
  # (src/machine/types.rs) accepts them.
  slotType = types.strMatching "[A-Za-z0-9_-]{1,64}";
  usbIdType = types.strMatching "[0-9a-f]{4}";

  slotOption = default: description: mkOption {
    type = slotType;
    inherit default description;
  };

  openrgbProvider = types.submodule {
    options = {
      nameContains = mkOption {
        type = types.nonEmptyStr;
        example = "ENE DRAM";
        description = ''
          Selects every OpenRGB controller whose display name contains this
          text, compared ASCII case-insensitively.
        '';
      };
      mode = mkOption {
        type = types.nonEmptyStr;
        example = "Static";
        description = ''
          The controller mode to set, by its exact name (case-insensitive).
          A mode the controller does not offer is an error naming the modes
          it does offer.
        '';
      };
    };
  };

  commandProvider = types.submodule {
    options = {
      argv = mkOption {
        type = types.nonEmptyListOf types.str;
        example = literalExpression ''[ "''${pkgs.liquidctl}/bin/liquidctl" "set" "ring" "color" "fixed" "{{color}}" ]'';
        description = ''
          The command vogix-machine.service runs, as root, with the slot's
          colour: every element that is exactly `{{color}}` becomes the
          colour as `rrggbb`. argv[0] is a path in the Nix store. One run
          per device at a time; colour changes during a run are coalesced
          into one more run with the latest colour. Its output goes to the
          unit's journal.
        '';
      };
      hotplug.hidraw = mkOption {
        type = types.nullOr (types.submodule {
          options = {
            vendorId = mkOption {
              type = usbIdType;
              example = "1e71";
              description = "USB vendor id, 4 lowercase hex digits.";
            };
            productId = mkOption {
              type = usbIdType;
              example = "3012";
              description = "USB product id, 4 lowercase hex digits.";
            };
          };
        });
        default = null;
        description = ''
          Run the command again whenever a hidraw node of this USB device
          appears (plugged in, or enumerated again after a resume).
        '';
      };
    };
  };

  deviceType = types.submodule {
    options = {
      slot = mkOption {
        type = slotType;
        example = "base01";
        description = ''
          The palette slot whose colour the device shows, as the owner's
          published palette names it (for vogix16 themes, base00..base0F).
        '';
      };
      provider = mkOption {
        type = types.attrTag {
          openrgb = mkOption {
            type = openrgbProvider;
            description = ''
              Controllers served by the OpenRGB SDK server (vogix.openrgb),
              driven by vogix-openrgb.service.
            '';
          };
          command = mkOption {
            type = commandProvider;
            description = "A command run by vogix-machine.service.";
          };
        };
        description = "How the device is driven: exactly one of `openrgb` or `command`.";
      };
    };
  };
in
{
  options.vogix.hardware = {

    devices = mkOption {
      type = types.attrsOf deviceType;
      default = { };
      example = literalExpression ''
        {
          dram-rgb = {
            slot = "base01";
            provider.openrgb = { nameContains = "ENE DRAM"; mode = "Static"; };
          };
          kraken-ring = {
            slot = "base01";
            provider.command = {
              argv = [ "''${pkgs.liquidctl}/bin/liquidctl" "--match" "kraken" "set" "ring" "color" "fixed" "{{color}}" ];
              hotplug.hidraw = { vendorId = "1e71"; productId = "3012"; };
            };
          };
        }
      '';
      description = ''
        Machine-global devices that show a colour of the machine owner's
        (vogix.machine.owner) current theme. Rendered into
        /etc/vogix/machine.json and applied by the machine owner units
        whenever the owner publishes a palette: OpenRGB devices by
        vogix-openrgb.service, command devices by vogix-machine.service.
        Names are 1-64 characters of [A-Za-z0-9_-]. The hardware modules
        declare their devices here.
      '';
    };

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

      rgb.ring = {
        enable = mkOption {
          type = types.bool;
          default = true;
          description = ''
            Colour the RGB ring around the LCD screen (8 LEDs, set with
            liquidctl) from the machine owner's theme: the command device
            vogix.hardware.devices.kraken-ring.
          '';
        };

        colorSlot = slotOption "base01" ''
          Vogix palette slot used to colour the ring.
          See vogix16.nix for the semantic slot map.
        '';
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

      colorSlot = slotOption "base0D" ''
        Vogix palette slot used to colour the keyboard backlight.
        Defaults to base0D (link/accent), bright and legible across themes;
        base01 (surface) is nearly invisible on dark variants because the
        keycaps absorb most of the LED.
        See vogix16.nix for the semantic slot map.
      '';
    };

    dram-rgb = {
      enable = mkEnableOption "addressable RGB on the DDR5 memory modules (ENE DRAM controllers) via OpenRGB's SMBus (i2c) interface";

      colorSlot = slotOption "base01" ''
        Vogix palette slot used to colour the DRAM RGB.
        Defaults to base01 (surface) for a subtle, monochromatic base.
        See vogix16.nix for the semantic slot map.
      '';
    };

  };
}
