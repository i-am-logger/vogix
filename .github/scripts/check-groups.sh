#!/usr/bin/env bash
# Prints the Integration matrix: the flake's x86_64-linux checks split into
# the groups of .github/check-groups.json, as a JSON array of
# {"group", "checks"}. A check the file does not name runs as a group of its
# own, so every check runs whether or not the file lists it; a name the file
# lists that is no longer a check is reported as a warning and skipped.
#
# The groups follow what a check builds: the desktop suite on Hyprland
# alone (quickshell, the longest VM run); the OpenRGB suites together (the
# OpenRGB build); the other quickshell and QML checks together; the plain VM
# suites four to a group; the evaluation-time and runCommand checks together.
set -euo pipefail

root=$(git rev-parse --show-toplevel)
checks=$(nix eval --json .#checks.x86_64-linux --apply builtins.attrNames)
groups=$(cat "$root/.github/check-groups.json")

jq -r --argjson checks "$checks" '
	[to_entries[] | .value[] | select(. as $c | $checks | index($c) | not)]
	| .[] | "::warning::.github/check-groups.json names \(.), which is not a check"
' <<<"$groups" >&2

jq -c --argjson checks "$checks" '
	(to_entries
		| map({group: .key, checks: [.value[] | select(. as $c | $checks | index($c))]})
		| map(select(.checks != []))) as $listed
	| ($listed | map(.checks[]) ) as $grouped
	| $listed + [$checks[] | select(. as $c | $grouped | index($c) | not) | {group: ., checks: [.]}]
	| map(.checks |= join(" "))
' <<<"$groups"
