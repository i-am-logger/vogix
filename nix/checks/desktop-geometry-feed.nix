# The data checks.desktop-smoke's geometry probe runs on. Every widget
# that shows only with data the build sandbox lacks is fed the widest
# realistic content it can show, from a source shaped like the real one,
# so the probe measures it showing that:
#
# - UPower: python-dbusmock's upower template on a private system bus,
#   each device carrying upowerd's full property set (its
#   dbus/org.freedesktop.UPower.Device.xml): a laptop's two batteries,
#   full; a wireless mouse; the AC adapter; and the display device
#   upowerd composes from the batteries (up-daemon.c), at 100 %;
# - BlueZ: the bluez5 template, an adapter with a headset connected;
# - the kernel, bound over the probe's view by bwrap: a /sys with an
#   amdgpu card busy at 100 % and a k10temp sensor (shaped as
#   desktop-logic's probe fixtures are), the sandbox's own /proc/meminfo
#   with 16 GiB of swap half in use, and a /run whose booted-system and
#   current-system are different generations (an update applied, a reboot
#   pending);
# - PipeWire (pipewire-daemon.nix): an analog sink and source named as a
#   desktop's HD Audio codec is, and a program recording: the microphone
#   in use;
# - Hyprland: its event socket, announcing a screencast as Hyprland 0.56
#   does; nothing in the probe asks the request socket for anything it
#   needs, so there is none;
# - wttrbar: the real one, reading a wttr.in answer (format=j1) from its
#   own cache: heavy snow showers at -23 °C;
# - do-not-disturb on, in the file the shell keeps it in.
#
# feed_start sets it all up, feed_run runs a command inside it, and
# feed_stop takes it down again.
{ pkgs }:

let
  python = pkgs.python3.withPackages (p: [ p.python-dbusmock ]);

  # upowerd's property set for a device (org.freedesktop.UPower.Device.xml),
  # as GVariant text.
  device = p: ''
    {'NativePath': <'${p.native}'>, 'Vendor': <'${p.vendor}'>, 'Model': <'${p.model}'>,
     'Serial': <'${p.serial}'>, 'UpdateTime': <uint64 1790100000>, 'Type': <uint32 ${toString p.type}>,
     'PowerSupply': <${p.powerSupply}>, 'HasHistory': <${p.history}>, 'HasStatistics': <${p.history}>,
     'Online': <${p.online}>, 'Energy': <${p.energy}>, 'EnergyEmpty': <0.0>,
     'EnergyFull': <${p.energyFull}>, 'EnergyFullDesign': <${p.energyFullDesign}>, 'EnergyRate': <0.0>,
     'Voltage': <${p.voltage}>, 'ChargeCycles': <${toString p.cycles}>, 'Luminosity': <0.0>,
     'TimeToEmpty': <int64 0>, 'TimeToFull': <int64 0>, 'Percentage': <${p.percentage}>,
     'Temperature': <0.0>, 'IsPresent': <${p.present}>, 'State': <uint32 ${toString p.state}>,
     'IsRechargeable': <${p.rechargeable}>, 'Capacity': <${p.capacity}>,
     'Technology': <uint32 ${toString p.technology}>, 'WarningLevel': <uint32 1>,
     'BatteryLevel': <uint32 ${toString p.level}>, 'IconName': <'${p.icon}'>,
     'ChargeStartThreshold': <uint32 0>, 'ChargeEndThreshold': <uint32 100>,
     'ChargeThresholdEnabled': <false>, 'ChargeThresholdSupported': <${p.thresholds}>,
     'ChargeThresholdSettingsSupported': <uint32 0>, 'VoltageMinDesign': <${p.voltageMin}>,
     'VoltageMaxDesign': <${p.voltageMax}>, 'CapacityLevel': <'''>}
  '';
  battery = { native, model, serial, energy, energyFullDesign, cycles, capacity }: device {
    inherit native model serial energy energyFullDesign cycles capacity;
    vendor = "SMP";
    # UP_DEVICE_KIND_BATTERY, UP_DEVICE_STATE_FULLY_CHARGED, lithium ion.
    type = 2;
    state = 4;
    technology = 1;
    powerSupply = "true";
    history = "true";
    online = "false";
    present = "true";
    rechargeable = "true";
    thresholds = "true";
    energyFull = energy;
    voltage = "17.38";
    voltageMin = "15.36";
    voltageMax = "17.6";
    percentage = "100.0";
    # UP_DEVICE_LEVEL_NONE: the battery reports a percentage.
    level = 1;
    icon = "battery-full-charged-symbolic";
  };
  devices = {
    battery_BAT0 = battery {
      native = "BAT0";
      model = "5B10W51867";
      serial = "1442";
      energy = "57.0";
      energyFullDesign = "57.0";
      cycles = 112;
      capacity = "100.0";
    };
    battery_BAT1 = battery {
      native = "BAT1";
      model = "01AV431";
      serial = "2718";
      energy = "22.8";
      energyFullDesign = "23.2";
      cycles = 387;
      capacity = "98.3";
    };
    # A peripheral: UP_DEVICE_KIND_MOUSE, discharging, reported by level.
    mouse_hidpp_battery_0 = device {
      native = "hidpp_battery_0";
      vendor = "Logitech";
      model = "MX Master 3S";
      serial = "4a-7c-11-e0";
      type = 5;
      state = 2;
      technology = 0;
      powerSupply = "false";
      history = "false";
      online = "false";
      present = "true";
      rechargeable = "true";
      thresholds = "false";
      energy = "0.0";
      energyFull = "0.0";
      energyFullDesign = "0.0";
      voltage = "0.0";
      voltageMin = "0.0";
      voltageMax = "0.0";
      cycles = -1;
      capacity = "0.0";
      percentage = "55.0";
      # UP_DEVICE_LEVEL_NORMAL.
      level = 6;
      icon = "battery-good-symbolic";
    };
    # UP_DEVICE_KIND_LINE_POWER, online.
    line_power_AC = device {
      native = "AC";
      vendor = "";
      model = "";
      serial = "";
      type = 1;
      state = 0;
      technology = 0;
      powerSupply = "true";
      history = "false";
      online = "true";
      present = "false";
      rechargeable = "false";
      thresholds = "false";
      energy = "0.0";
      energyFull = "0.0";
      energyFullDesign = "0.0";
      voltage = "0.0";
      voltageMin = "0.0";
      voltageMax = "0.0";
      cycles = -1;
      capacity = "0.0";
      percentage = "0.0";
      level = 0;
      icon = "ac-adapter-symbolic";
    };
  };
  # The display device, as upowerd composes it from the two batteries
  # (up_daemon_update_display_battery): a device of its own, whose
  # identity fields are empty.
  displayDevice = device {
    native = "";
    vendor = "";
    model = "";
    serial = "";
    type = 2;
    state = 4;
    technology = 0;
    powerSupply = "true";
    history = "false";
    online = "false";
    present = "true";
    rechargeable = "false";
    thresholds = "false";
    energy = "79.8";
    energyFull = "79.8";
    energyFullDesign = "0.0";
    voltage = "0.0";
    voltageMin = "0.0";
    voltageMax = "0.0";
    cycles = 0;
    capacity = "0.0";
    percentage = "100.0";
    level = 1;
    icon = "battery-full-charged-symbolic";
  };

  # PipeWire's names for a desktop's HD Audio codec: the longest a picker
  # shows on the machines this shell runs on.
  pipewireConf = import ./pipewire-daemon.nix { inherit pkgs; } "geometry-pipewire.conf" ''
    { factory = adapter
      args = {
        factory.name = support.null-audio-sink
        node.name = alsa_output.pci-0000_7a_00.6.analog-stereo
        node.description = "Family 17h/19h/1ah HD Audio Controller Analog Stereo"
        node.nick = "ALC1220 Analog"
        media.class = Audio/Sink
        audio.position = [ FL FR ]
        monitor.channel-volumes = true
      }
    }
    { factory = adapter
      args = {
        factory.name = support.null-audio-sink
        node.name = alsa_input.pci-0000_7a_00.6.analog-stereo
        node.description = "Family 17h/19h/1ah HD Audio Controller Analog Stereo"
        node.nick = "ALC1220 Analog"
        media.class = Audio/Source
        audio.position = [ FL FR ]
      }
    }
    { factory = metadata
      args = {
        metadata.name = default
        metadata.values = [
          { key = default.audio.sink type = "Spa:String:JSON"
            value = { name = alsa_output.pci-0000_7a_00.6.analog-stereo } }
          { key = default.audio.source type = "Spa:String:JSON"
            value = { name = alsa_input.pci-0000_7a_00.6.analog-stereo } }
        ]
      }
    }
  '';

  # Hyprland's event socket, as a client sees it once a screen capture
  # session has started delivering frames.
  hyprEvents = pkgs.writeShellScript "hyprland-events" ''
    printf 'screencast>>1,monitor\n'
    exec cat > /dev/null
  '';

  # A wttr.in answer (format=j1) with every field wttrbar reads. The
  # forecast days are dated when the feed starts: wttrbar drops a day
  # before today.
  hour = time: code: desc: tempC: feelsC: {
    inherit time;
    tempC = toString tempC;
    tempF = toString (tempC * 9 / 5 + 32);
    FeelsLikeC = toString feelsC;
    FeelsLikeF = toString (feelsC * 9 / 5 + 32);
    weatherCode = toString code;
    weatherDesc = [{ value = desc; }];
    chanceoffog = "0";
    chanceoffrost = "91";
    chanceofovercast = "88";
    chanceofrain = "0";
    chanceofsnow = "84";
    chanceofsunshine = "4";
    chanceofthunder = "0";
    chanceofwindy = "12";
    humidity = "86";
    windspeedKmph = "31";
    windspeedMiles = "19";
  };
  day = date: max: min: {
    inherit date;
    maxtempC = toString max;
    maxtempF = toString (max * 9 / 5 + 32);
    mintempC = toString min;
    mintempF = toString (min * 9 / 5 + 32);
    astronomy = [{
      sunrise = "10:18 AM";
      sunset = "03:14 PM";
      moonrise = "01:02 PM";
      moonset = "08:41 AM";
      moon_phase = "Waxing Gibbous";
      moon_illumination = "78";
    }];
    hourly = [
      (hour "0" 338 "Heavy snow" (-24) (-32))
      (hour "300" 338 "Heavy snow" (-25) (-33))
      (hour "600" 335 "Heavy snow showers" (-25) (-34))
      (hour "900" 335 "Heavy snow showers" (-23) (-31))
      (hour "1200" 332 "Moderate snow" (-21) (-29))
      (hour "1500" 326 "Light snow showers" (-20) (-27))
      (hour "1800" 338 "Heavy snow" (-22) (-30))
      (hour "2100" 338 "Heavy snow" (-24) (-32))
    ];
  };
  wttrJson = pkgs.writeText "wttr-j1.json" (builtins.toJSON {
    current_condition = [{
      FeelsLikeC = "-31";
      FeelsLikeF = "-24";
      cloudcover = "100";
      humidity = "86";
      localObsDateTime = "2026-01-14 09:40 AM";
      observation_time = "04:40 PM";
      precipInches = "0.1";
      precipMM = "2.4";
      pressure = "1021";
      temp_C = "-23";
      temp_F = "-9";
      uvIndex = "0";
      visibility = "2";
      weatherCode = "335";
      weatherDesc = [{ value = "Heavy snow showers"; }];
      winddir16Point = "NNW";
      winddirDegree = "338";
      windspeedKmph = "31";
      windspeedMiles = "19";
    }];
    nearest_area = [{
      areaName = [{ value = "Fairbanks"; }];
      region = [{ value = "Alaska"; }];
      country = [{ value = "United States of America"; }];
      latitude = "64.838";
      longitude = "-147.716";
      population = "30917";
    }];
    weather = [ (day "@D0@" (-20) (-25)) (day "@D1@" (-18) (-27)) (day "@D2@" (-15) (-22)) ];
  });
in
{
  packages = [
    python
    pkgs.bubblewrap
    pkgs.glib.bin
    pkgs.pipewire
    pkgs.socat
  ];

  script = pkgs.writeText "desktop-geometry-feed.sh" ''
    feed=$TMPDIR/feed
    feed_bus=unix:path=$feed/system_bus_socket
    feed_hypr=vogix-geometry

    # Polls every 0.1 s, for up to 60 s, until its command succeeds; says
    # which part of the feed never came up otherwise.
    feed_until() {
      local what=$1 _
      shift
      for _ in $(seq 600); do
        "$@" > /dev/null 2>&1 && return 0
        sleep 0.1
      done
      echo "feed: $what never came up"
      return 1
    }

    # Stops at the first part that fails, with a non-zero status.
    feed_start() (
      set -e
      mkdir -p $feed
      # The system bus: UPower and BlueZ.
      dbus-daemon --config-file=${pkgs.dbus}/share/dbus-1/session.conf \
        --address=$feed_bus --fork --print-pid > $feed/bus.pid
      DBUS_SYSTEM_BUS_ADDRESS=$feed_bus python3 -m dbusmock --system --template upower \
        > $feed/upower.log 2>&1 &
      echo $! > $feed/upower.pid
      DBUS_SYSTEM_BUS_ADDRESS=$feed_bus python3 -m dbusmock --system --template bluez5 \
        > $feed/bluez.log 2>&1 &
      echo $! > $feed/bluez.pid
      gdbus wait --address $feed_bus --timeout 60 org.freedesktop.UPower
      gdbus wait --address $feed_bus --timeout 60 org.bluez
      local path
      ${pkgs.lib.concatStrings (pkgs.lib.mapAttrsToList (name: props: ''
        path=/org/freedesktop/UPower/devices/${name}
        gdbus call --address $feed_bus --dest org.freedesktop.UPower \
          --object-path /org/freedesktop/UPower --method org.freedesktop.DBus.Mock.AddObject \
          "'$path'" "'org.freedesktop.UPower.Device'" ${pkgs.lib.escapeShellArg props} '@a(ssss) []' > /dev/null
      '') devices)}
      # The template's display device carries only part of the set:
      # replaced by the whole one.
      gdbus call --address $feed_bus --dest org.freedesktop.UPower \
        --object-path /org/freedesktop/UPower --method org.freedesktop.DBus.Mock.RemoveObject \
        "'/org/freedesktop/UPower/devices/DisplayDevice'" > /dev/null
      gdbus call --address $feed_bus --dest org.freedesktop.UPower \
        --object-path /org/freedesktop/UPower --method org.freedesktop.DBus.Mock.AddObject \
        "'/org/freedesktop/UPower/devices/DisplayDevice'" "'org.freedesktop.UPower.Device'" \
        ${pkgs.lib.escapeShellArg displayDevice} "[('Refresh', \"\", \"\", \"\")]" > /dev/null
      gdbus call --address $feed_bus --dest org.bluez --object-path / \
        --method org.bluez.Mock.AddAdapter "'hci0'" "'yoga'" > /dev/null
      gdbus call --address $feed_bus --dest org.bluez --object-path / \
        --method org.bluez.Mock.AddDevice "'hci0'" "'AC:80:0A:3F:21:9C'" "'WH-1000XM5'" > /dev/null
      # A headset, as bluetoothd lists one once paired: its class of device
      # (audio/video, wearable headset) and its profiles (A2DP sink, AVRCP,
      # HFP, HSP).
      gdbus call --address $feed_bus --dest org.bluez \
        --object-path /org/bluez/hci0/dev_AC_80_0A_3F_21_9C \
        --method org.freedesktop.DBus.Mock.UpdateProperties "'org.bluez.Device1'" \
        "{'Icon': <'audio-headset'>, 'Class': <uint32 2360324>, 'Paired': <true>,
          'Trusted': <true>, 'ServicesResolved': <true>,
          'UUIDs': <['0000110b-0000-1000-8000-00805f9b34fb', '0000110c-0000-1000-8000-00805f9b34fb',
                    '0000110e-0000-1000-8000-00805f9b34fb', '0000111e-0000-1000-8000-00805f9b34fb',
                    '00001108-0000-1000-8000-00805f9b34fb']>}" > /dev/null
      gdbus call --address $feed_bus --dest org.bluez --object-path / \
        --method org.bluez.Mock.ConnectDevice "'hci0'" "'AC:80:0A:3F:21:9C'" > /dev/null

      # /sys: an amdgpu card (the PCI function its DRM card links to) and
      # the CPU's temperature sensor.
      local gpu=$feed/sys/devices/pci0000:00/0000:78:00.0
      mkdir -p $gpu/power $feed/sys/bus/pci/drivers/amdgpu $feed/sys/class/drm/card1 \
        $feed/sys/class/drm/card1-DP-1 $feed/sys/class/hwmon/hwmon0
      ln -s ../../../bus/pci/drivers/amdgpu $gpu/driver
      echo on > $gpu/power/control
      echo 1 > $gpu/boot_vga
      echo 100 > $gpu/gpu_busy_percent
      ln -s ../../../devices/pci0000:00/0000:78:00.0 $feed/sys/class/drm/card1/device
      ln -s ../card1 $feed/sys/class/drm/card1-DP-1/device
      echo k10temp > $feed/sys/class/hwmon/hwmon0/name
      echo Tctl > $feed/sys/class/hwmon/hwmon0/temp1_label
      echo 95250 > $feed/sys/class/hwmon/hwmon0/temp1_input
      # /proc/meminfo with 16 GiB of swap, half of it in use.
      awk '/^SwapTotal:/ { printf "SwapTotal:      %8d kB\n", 16777212; next }
           /^SwapFree:/ { printf "SwapFree:       %8d kB\n", 8388604; next }
           { print }' /proc/meminfo > $feed/meminfo
      # /run: booted into one generation, switched to the next.
      mkdir -p $feed/run
      ln -s /nix/store/0q2fchz0ymc5ic9mgzgq7ydqyyxmwbn4-nixos-system-yoga-26.05.20260901.5d8f2a1 \
        $feed/run/booted-system
      ln -s /nix/store/k3mb1r8yz2xjbdnf6gx3w7c5y0v4p9sa-nixos-system-yoga-26.05.20260915.9b04c7e \
        $feed/run/current-system

      # PipeWire, and a program recording from its default source.
      pipewire -c ${pipewireConf} > $feed/pipewire.log 2>&1 &
      echo $! > $feed/pipewire.pid
      feed_until "the PipeWire daemon" pw-cli info 0
      pw-cat --record --raw --format=s16 --rate=48000 --channels=2 /dev/null >> $feed/pipewire.log 2>&1 &
      echo $! > $feed/record.pid
      feed_until "the recording stream" sh -c "pw-cli ls Node | grep -q 'Stream/Input/Audio'"

      # Hyprland's event socket.
      mkdir -p $XDG_RUNTIME_DIR/hypr/$feed_hypr
      socat UNIX-LISTEN:$XDG_RUNTIME_DIR/hypr/$feed_hypr/.socket2.sock,fork EXEC:${hyprEvents} &
      echo $! > $feed/hypr.pid
      feed_until "Hyprland's event socket" test -S $XDG_RUNTIME_DIR/hypr/$feed_hypr/.socket2.sock

      # wttrbar's cache of a wttr.in answer, fresh (under ten minutes old).
      sed -e "s/@D0@/$(date +%F)/" -e "s/@D1@/$(date -d tomorrow +%F)/" \
        -e "s/@D2@/$(date -d '2 days' +%F)/" ${wttrJson} > /tmp/wttrbar--wttr.in.json

      mkdir -p $XDG_STATE_HOME/vogix/desktop
      printf '{"dnd":true}' > $XDG_STATE_HOME/vogix/desktop/dnd.json
    )

    # Runs its arguments on the fed data: the system bus and Hyprland's
    # socket in the environment, the kernel's files bound in place.
    feed_run() {
      local binds=() d
      for d in /*; do
        case $d in /proc | /dev | /sys | /run) ;; *) binds+=(--bind "$d" "$d") ;; esac
      done
      DBUS_SYSTEM_BUS_ADDRESS=$feed_bus HYPRLAND_INSTANCE_SIGNATURE=$feed_hypr \
        bwrap "''${binds[@]}" --dev-bind /dev /dev --proc /proc \
        --ro-bind $feed/meminfo /proc/meminfo --ro-bind $feed/sys /sys --ro-bind $feed/run /run \
        -- "$@"
    }

    feed_stop() {
      local f
      for f in record pipewire hypr upower bluez bus; do
        kill "$(cat $feed/$f.pid)" 2>/dev/null
      done
      rm -f /tmp/wttrbar--wttr.in.json $XDG_STATE_HOME/vogix/desktop/dnd.json
    }
  '';
}
