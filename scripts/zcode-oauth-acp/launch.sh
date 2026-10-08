#!/bin/sh
set -eu
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
if [ -n "${ZCODE_NODE:-}" ]; then
  exec "$ZCODE_NODE" "$script_dir/launch.mjs" "$@"
fi
for node_bin in /opt/homebrew/bin/node /usr/local/bin/node; do
  if [ -x "$node_bin" ]; then
    exec "$node_bin" "$script_dir/launch.mjs" "$@"
  fi
done
exec node "$script_dir/launch.mjs" "$@"
