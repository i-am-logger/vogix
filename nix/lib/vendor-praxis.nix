# Vendor praxis into the vogix source tree for sandboxed Nix builds.
# Copies praxis/crates/praxis into .nix-deps/praxis and patches Cargo.toml.
{ pkgs
, praxis-src
, vogixSrc
}:

pkgs.runCommand "vogix-with-praxis" { } ''
  cp -r ${vogixSrc} $out
  chmod -R u+w $out
  mkdir -p $out/.nix-deps/praxis
  cp -r ${praxis-src}/crates/praxis/. $out/.nix-deps/praxis/
  substituteInPlace $out/Cargo.toml \
    --replace-fail '../praxis/crates/praxis' '.nix-deps/praxis'
''
