{ lib
, pkgs
, rustPlatform
, pkg-config
, dbus
, praxis-src ? null
,
}:

let
  cargoToml = builtins.fromTOML (builtins.readFile ../../Cargo.toml);

  rawSrc = lib.cleanSource ../..;

  # Vendor praxis into source tree if available
  src =
    if praxis-src != null then
      runCommand "vogix-with-praxis" { } ''
        cp -r ${rawSrc} $out
        chmod -R u+w $out
        mkdir -p $out/.nix-deps/praxis
        cp -r ${praxis-src}/crates/praxis/. $out/.nix-deps/praxis/
        substituteInPlace $out/Cargo.toml \
          --replace-fail '../praxis/crates/praxis' '.nix-deps/praxis'
      ''
    else
      rawSrc;
in
rustPlatform.buildRustPackage rec {
  pname = cargoToml.package.name;
  inherit (cargoToml.package) version;

  inherit src;

  cargoLock = {
    lockFile = ../../Cargo.lock;
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
