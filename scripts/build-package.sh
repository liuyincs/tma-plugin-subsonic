#!/usr/bin/env sh
set -eu

: "${TMA_PLUGIN_SIGNING_KEY_B64:?set TMA_PLUGIN_SIGNING_KEY_B64 to a release-only Ed25519 seed}"
: "${TMA_PLUGIN_DEV:?set TMA_PLUGIN_DEV to a compatible tma-plugin-dev executable}"

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
wasm="$root/target/wasm32-unknown-unknown/release/tma_plugin_subsonic.wasm"
[ -f "$wasm" ] || {
  echo "missing $wasm; run cargo build --target wasm32-unknown-unknown --release first" >&2
  exit 1
}
cp "$wasm" "$root/plugin.wasm"
"$TMA_PLUGIN_DEV" pack "$root" "$TMA_PLUGIN_SIGNING_KEY_B64" -o "$root/subsonic.tmap"
sha256sum "$root/subsonic.tmap" > "$root/subsonic.tmap.sha256"
rm -f "$root/plugin.wasm"
