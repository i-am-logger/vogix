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

  src = lib.cleanSource ../..;

  cargoLock = {
    lockFile = ../../Cargo.lock;
    # praxis is a git dep (pr4xis-domains is publish=false on crates.io, so it +
    # its path-dep pr4xis come from git). buildRustPackage needs the fetched-source
    # hash; cargo vendors the whole praxis workspace, so `version.workspace = true`
    # resolves against its root. All praxis crates share one git source ⇒ one hash.
    outputHashes = {
      "pr4xis-0.25.0" = "sha256-9/FZp8lYXPjgOi2ek6I0DsujxVdv2spRx7iYIhY8qE8=";
      "pr4xis-derive-0.25.0" = "sha256-9/FZp8lYXPjgOi2ek6I0DsujxVdv2spRx7iYIhY8qE8=";
      "pr4xis-domains-0.25.0" = "sha256-9/FZp8lYXPjgOi2ek6I0DsujxVdv2spRx7iYIhY8qE8=";
      "pr4xis-runtime-0.25.0" = "sha256-9/FZp8lYXPjgOi2ek6I0DsujxVdv2spRx7iYIhY8qE8=";
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
