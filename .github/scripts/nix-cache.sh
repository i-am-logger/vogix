#!/usr/bin/env bash
# The CI jobs' Nix binary cache: a file:// cache, kept in the GitHub Actions
# cache, holding only store paths that carry no signature, i.e. the paths
# no public substituter serves: what this flake builds itself (vogix,
# OpenRGB, quickshell, the VM test results and the rest) and what devenv
# assembles (the Rust toolchain). Their signed references stay out of it;
# a job substitutes them from cache.nixos.org and nix-community.cachix.org,
# only when it needs them.
#
#   nix-cache.sh graph <installable>...
#     Prints every store path of the installables' build graph: each
#     derivation's outputs and every source a derivation reads.
#   nix-cache.sh export <cache-dir> <installable>...
#     Adds the graph's paths that are in the store and unsigned.
#   nix-cache.sh export-closure <cache-dir> <store-path>...
#     Adds the unsigned paths of the store paths' closure.
#   nix-cache.sh prune <cache-dir> <graph-file>
#     Removes every entry whose store path is not listed in <graph-file>,
#     and every NAR no remaining entry names.
#
# With NIX_CACHE_BASE set to another cache directory, export and
# export-closure skip the paths that cache already holds.
set -euo pipefail

graph() {
  local requisites
  requisites=$(mktemp)
  nix path-info --derivation "$@" | xargs nix-store --query --requisites >"$requisites"
  {
    grep -v '\.drv$' "$requisites" || true
    grep '\.drv$' "$requisites" | xargs -r nix-store --query --outputs
  } | sort -u
  rm -f "$requisites"
}

summary() {
  echo "$1: $(find "$1" -maxdepth 1 -name '*.narinfo' | wc -l) paths, $(du -sh "$1" | cut -f1)"
}

# Copies the store paths listed in $2 that carry no signature into the
# binary cache $1. A path's signature list follows a tab; "ultimate" (built
# here) is not a signature, and every signature has the form <key>:<sig>.
copy_unsigned() {
  local dir=$1 paths=$2 unsigned signed stubs ref hash size name
  unsigned=$(mktemp)
  signed=$(mktemp)
  stubs=$(mktemp)
  xargs -r nix path-info --sigs <"$paths" |
    awk -F'\t' '$2 !~ /:/ { sub(/ +$/, "", $1); print $1 }' |
    while read -r path; do
      name=${path#/nix/store/}
      [ -n "${NIX_CACHE_BASE:-}" ] && [ -e "$NIX_CACHE_BASE/${name:0:32}.narinfo" ] && continue
      echo "$path"
    done | sort -u >"$unsigned"
  mkdir -p "$dir"
  [ -e "$dir/nix-cache-info" ] || printf 'StoreDir: /nix/store\n' >"$dir/nix-cache-info"
  # A binary cache accepts a path only when each of its references has an
  # entry there. Each signed reference gets a placeholder entry, removed
  # once the copy is done, so the cache names those references without
  # holding them.
  xargs -r nix-store --query --references <"$unsigned" | sort -u >"$signed.all"
  comm -23 "$signed.all" "$unsigned" >"$signed"
  xargs -r nix-store --query --hash <"$signed" >"$signed.hash"
  xargs -r nix-store --query --size <"$signed" >"$signed.size"
  paste "$signed" "$signed.hash" "$signed.size" | while read -r ref hash size; do
    name=${ref#/nix/store/}
    [ -e "$dir/${name:0:32}.narinfo" ] && continue
    printf 'StorePath: %s\nURL: nar/placeholder.nar\nCompression: none\nNarHash: %s\nNarSize: %s\n' \
      "$ref" "$hash" "$size" >"$dir/${name:0:32}.narinfo"
    echo "$dir/${name:0:32}.narinfo" >>"$stubs"
  done
  nix copy --no-recursive --to "file://$dir?compression=zstd&parallel-compression=true" --stdin <"$unsigned"
  xargs -r rm -f <"$stubs"
  rm -f "$unsigned" "$signed" "$signed".* "$stubs"
  summary "$dir"
}

export_graph() {
  local dir=$1 all invalid valid
  shift
  all=$(mktemp)
  invalid=$(mktemp)
  valid=$(mktemp)
  graph "$@" >"$all"
  xargs -r nix-store --check-validity --print-invalid <"$all" | sort -u >"$invalid"
  comm -23 "$all" "$invalid" >"$valid"
  copy_unsigned "$dir" "$valid"
  rm -f "$all" "$invalid" "$valid"
}

export_closure() {
  local dir=$1 closure
  shift
  closure=$(mktemp)
  nix-store --query --requisites "$@" | sort -u >"$closure"
  copy_unsigned "$dir" "$closure"
  rm -f "$closure"
}

prune() {
  local dir=$1 keep=$2 used
  find "$dir" -maxdepth 1 -name '*.narinfo' -exec grep -H '^StorePath: ' {} + |
    sed 's/:StorePath: /\t/' |
    awk -F'\t' 'NR == FNR { keep[$0] = 1; next } !($2 in keep) { print $1 }' "$keep" - |
    xargs -r rm -f
  used=$(mktemp)
  find "$dir" -maxdepth 1 -name '*.narinfo' -exec sed -n 's/^URL: //p' {} + | sort -u >"$used"
  (cd "$dir" && find nar -type f 2>/dev/null | sort | comm -23 - "$used" | xargs -r rm -f)
  rm -f "$used"
  summary "$dir"
}

cmd=${1:?usage: nix-cache.sh graph|export|export-closure|prune ...}
shift
case $cmd in
graph) graph "$@" ;;
export) export_graph "$@" ;;
export-closure) export_closure "$@" ;;
prune) prune "$@" ;;
*)
  echo "unknown command: $cmd" >&2
  exit 2
  ;;
esac
