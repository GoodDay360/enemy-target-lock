#!/usr/bin/env bash
# Run the unit tests of the modules that don't use the Horizon API.
#
# `cargo test` needs the mod API, which only exists once the SDK is fetched, so these
# pure-logic modules are compiled together on their own with plain `rustc`.
# Usage: scripts/unit-tests.sh   (from anywhere; needs rustc with edition 2024)
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

modules=(ballistics camera cinematic config config_file input server_config shapes smooth targeting view_lag)

{
    echo '#![allow(dead_code)]'
    for module in "${modules[@]}"; do
        echo "#[path = \"$root/src/$module.rs\"] mod $module;"
    done
} > "$work/tests.rs"

rustc --edition 2024 --test "$work/tests.rs" -o "$work/tests"
"$work/tests"
