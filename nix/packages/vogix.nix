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
    # praxis is a git dependency (publish=false workspace); buildRustPackage
    # needs the fetched-source hash. cargo's vendoring fetches the whole praxis
    # repo, so `version.workspace = true` resolves against its workspace root —
    # the reason this (buildRustPackage) builds where crate2nix's per-crate
    # vendor could not. All praxis crates share one git source ⇒ one hash.
    outputHashes = {
      "pr4xis-0.24.0" = "sha256-GdHAKg+1pvT0KTgoVJn/QuD9vs7yt7HSQZjG6hfeNvw=";
      "pr4xis-derive-0.24.0" = "sha256-GdHAKg+1pvT0KTgoVJn/QuD9vs7yt7HSQZjG6hfeNvw=";
      "pr4xis-domains-0.24.0" = "sha256-GdHAKg+1pvT0KTgoVJn/QuD9vs7yt7HSQZjG6hfeNvw=";
      "pr4xis-runtime-0.24.0" = "sha256-GdHAKg+1pvT0KTgoVJn/QuD9vs7yt7HSQZjG6hfeNvw=";
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
