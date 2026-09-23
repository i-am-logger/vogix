# A PipeWire daemon with no hardware, for the desktop checks that run the
# shell against a real one: the core modules, the dummy driver that clocks
# its nodes, and the objects a check declares (null sinks and sources, the
# default-device metadata a session manager would publish).
{ pkgs }:
name: objects:
pkgs.writeText name ''
  context.properties = {
    core.daemon = true
    core.name = pipewire-0
    link.max-buffers = 16
    support.dbus = false
  }
  context.spa-libs = {
    audio.convert.* = audioconvert/libspa-audioconvert
    support.* = support/libspa-support
  }
  context.modules = [
    { name = libpipewire-module-protocol-native }
    { name = libpipewire-module-metadata }
    { name = libpipewire-module-spa-node-factory }
    { name = libpipewire-module-client-node }
    { name = libpipewire-module-adapter }
    { name = libpipewire-module-link-factory }
    { name = libpipewire-module-access }
  ]
  context.objects = [
    { factory = spa-node-factory
      args = {
        factory.name = support.node.driver
        node.name = Dummy-Driver
        priority.driver = 20000
      }
    }
  ${objects}
  ]
''
