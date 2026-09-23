# OpenRGB built from the integration/vogix branch of
# github:i-am-logger/OpenRGB: upstream OpenRGB plus
# - systemd readiness: READY=1 goes to $NOTIFY_SOCKET once every SDK
#   server socket listens, so a Type=notify openrgb.service is active
#   exactly when clients can connect;
# - the NetworkServer controller-queue, profile-manager and client-send
#   threads wait on their queues with a predicate, so a write queued while
#   a thread is about to sleep is not stranded;
# - a NetworkServer client is reference-counted: its queued requests and
#   every server signal being sent to it hold it, and its listen thread
#   deletes it only once none do, so a reply, ACK or signal never reaches a
#   deleted client or a reused socket descriptor;
# - signals go to a referenced copy of the client list, sent with no list
#   lock held;
# - StopServer stops the profile-manager thread and shuts every client's
#   socket down, and the listen threads delete the clients.
#
# `pin` is the only place the source is named; moving to a newer branch
# head is one edit of rev and hash. The build is the caller's openrgb
# recipe (its inputs, flags and nixpkgs-local patches) with this source;
# the source-specific hooks below replace the recipe's, which target a
# release tree. passthru.vogixReadiness marks the build as one that sends
# READY=1; the vogix NixOS module asserts it before it runs
# openrgb.service as Type=notify.
{ lib
, openrgb
, fetchFromGitHub
, coreutils
}:

let
  pin = {
    rev = "d1192da8bcab710dba38ff88fd98cf7841b16125";
    hash = "sha256-goRGIYy5JjmPdyhM6jVHqGQ+eemvefc4UhxZdPHWYKE=";
  };

  # A plugin wrapper execs this same binary, so it keeps the readiness.
  keepReadiness = pkg: pkg.overrideAttrs (old: {
    passthru = (old.passthru or { }) // { vogixReadiness = true; };
  });
in
openrgb.overrideAttrs (old: {
  version = "vogix-${builtins.substring 0 8 pin.rev}";

  src = fetchFromGitHub {
    owner = "i-am-logger";
    repo = "OpenRGB";
    inherit (pin) rev hash;
  };

  # This tree has no scripts/build-udev-rules.sh; the binary generates the
  # rules itself (postInstall).
  postPatch = "";

  postInstall = ''
    substituteInPlace "$out/lib/systemd/system/openrgb.service" \
      --replace-fail /usr/bin/openrgb "$out/bin/openrgb"

    # --generate-udev-rules writes the file and exits before any detection
    # or Qt startup.
    mkdir -p "$out/lib/udev/rules.d"
    HOME=$TMPDIR "$out/bin/openrgb" --generate-udev-rules "$out/lib/udev/rules.d/60-openrgb.rules"
    substituteInPlace "$out/lib/udev/rules.d/60-openrgb.rules" \
      --replace-fail '/usr/bin/env chmod' ${lib.getExe' coreutils "chmod"}
  '';

  doInstallCheck = true;
  installCheckPhase = ''
    runHook preInstallCheck

    HOME=$TMPDIR "$out/bin/openrgb" --help > /dev/null

    rules="$out/lib/udev/rules.d/60-openrgb.rules"
    grep -q 'SUBSYSTEM' "$rules"
    if grep -F /usr/bin/env "$rules"; then
      echo "Error: udev rules must not reference /usr/bin/env"
      exit 1
    fi

    unit="$out/lib/systemd/system/openrgb.service"
    grep -qx 'Type=notify' "$unit"
    grep -q "^ExecStart=$out/bin/openrgb " "$unit"

    # The readiness notification is compiled in.
    grep -rqF 'Notified service manager that the server is ready' "$out/bin"

    runHook postInstallCheck
  '';

  passthru = (old.passthru or { }) // {
    vogixReadiness = true;
  } // lib.optionalAttrs (old.passthru ? withPlugins) {
    withPlugins = plugins: keepReadiness (old.passthru.withPlugins plugins);
  };

  # The recipe's changelog names a release tag; this source is a commit.
  meta = (old.meta or { }) // {
    changelog = "https://github.com/i-am-logger/OpenRGB/commits/${pin.rev}";
  };
})
