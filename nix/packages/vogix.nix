{ lib
, rustPlatform
, pkg-config
, dbus
,
}:

let
  cargoToml = builtins.fromTOML (builtins.readFile ../../Cargo.toml);
in
rustPlatform.buildRustPackage {
  pname = cargoToml.package.name;
  inherit (cargoToml.package) version;

  # Exactly the files the build and its tests read, so an edit anywhere else
  # (the docs, the Nix modules, the VM suites) leaves this derivation, and
  # every one that uses it, unchanged.
  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ../../Cargo.toml
      ../../Cargo.lock
      ../../src
      ../../tests/machine_reactor_signals.rs
      # include_str!/include_bytes! fixtures and the captured OpenRGB payloads
      ../../tests/fixtures
      # The templates the template tests render and include
      ../../templates
      # Every example in it is parsed by a CLI test.
      ../../docs/cli.md
      # The shell sources the desktop and CLI tests pin: the widget
      # registry and the components it names, the section, the panels
      # service, the registry service and shell.qml
      ../../desktop/Bar/widgets
      ../../desktop/Bar/Section.qml
      ../../desktop/Services/Panels.qml
      ../../desktop/Services/WidgetRegistry.qml
      ../../desktop/shell.qml
      # The default desktop.json the home-manager module renders, pinned
      ../../nix/modules/desktop/desktop-json.pin.json
    ];
  };

  cargoLock = {
    lockFile = ../../Cargo.lock;
    # praxis is a git dep (pr4xis-domains is publish=false on crates.io, so it +
    # its path-dep pr4xis come from git). buildRustPackage needs the fetched-source
    # hash; cargo vendors the whole praxis workspace, so `version.workspace = true`
    # resolves against its root. All praxis crates share one git source ⇒ one hash.
    outputHashes = {
      "pr4xis-0.29.1" = "sha256-Inp6/q6AOkTb6KHbTNbQZRTmNToYVWRnRqALyuk5ceE=";
      "pr4xis-derive-0.29.1" = "sha256-Inp6/q6AOkTb6KHbTNbQZRTmNToYVWRnRqALyuk5ceE=";
      "pr4xis-domains-0.29.1" = "sha256-Inp6/q6AOkTb6KHbTNbQZRTmNToYVWRnRqALyuk5ceE=";
      "pr4xis-runtime-0.29.1" = "sha256-Inp6/q6AOkTb6KHbTNbQZRTmNToYVWRnRqALyuk5ceE=";
    };
  };

  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs = [
    dbus
  ];

  meta = with lib; {
    inherit (cargoToml.package) description;
    homepage = "https://github.com/i-am-logger/vogix";
    license = licenses.cc-by-nc-sa-40;
    maintainers = [ ];
    mainProgram = cargoToml.package.name;
  };
}
