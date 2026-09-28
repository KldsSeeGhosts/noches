#!/usr/bin/env bash
# Real-engine rig for the iOS companion: a seeded mock-harness engine behind
# noches-connect on loopback, reachable from the iOS Simulator. It prints a
# pairing code to paste into the app. Nothing touches your real Noches data.
#
#   scripts/companion-rig.sh           # build, seed, serve (Ctrl-C stops)
#   scripts/companion-rig.sh --slow    # pace mock streams to watch streaming
#
# State lives under /tmp/noches-companion-rig; delete it to reseed.
set -euo pipefail
cd "$(dirname "$0")/.."

RIG=/tmp/noches-companion-rig
IPC=27931
GATEWAY=127.0.0.1:27932
DELAY=""
[[ "${1:-}" == --slow ]] && DELAY=350
mkdir -p "$RIG"

echo "▸ building engine + gateway"
cargo build -q -p zeron -p zeron-rpc --bin zeron --bin noches-connect --example rpc_probe
BIN=./target/debug

env ZERON_DATA_DIR="$RIG/engine" ZERON_IPC_PORT=$IPC ZERON_HARNESS=mock ZERON_MOCK_AGENTS=1 \
  ${DELAY:+ZERON_MOCK_DELAY_MS=$DELAY} RUST_LOG=warn "$BIN/zeron" headless &
PIDS=($!)
trap 'kill "${PIDS[@]}" 2>/dev/null || true' EXIT
for _ in $(seq 1 60); do
  (exec 3<>/dev/tcp/127.0.0.1/$IPC) 2>/dev/null && { exec 3>&-; break; }
  sleep 0.25
done

probe() { "$BIN/examples/rpc_probe" "ws://127.0.0.1:$IPC" "$@"; }

if [[ ! -f "$RIG/.seeded" ]]; then
  echo "▸ seeding sessions"
  DEV=$(probe LocalDevice '{}' | python3 -c 'import json,sys;print(json.load(sys.stdin)["deviceId"])')
  for project in noches website; do
    mkdir -p "$RIG/projects/$project"
    sid=$(uuidgen | tr 'A-Z' 'a-z')
    probe Mutate "{\"op\":\"createSpace\",\"spaceId\":\"$sid\",\"deviceId\":\"$DEV\",\"path\":\"$RIG/projects/$project\"}" >/dev/null
    eval "SPACE_$project=\$sid"
  done
  seed() { # title project branch prompt-or-skip
    local id sid; id=$(uuidgen | tr 'A-Z' 'a-z'); eval "sid=\$SPACE_$2"
    probe Mutate "{\"op\":\"createChat\",\"chatId\":\"$id\",\"spaceId\":\"$sid\",\"config\":{\"harness\":\"mock\",\"model\":null,\"reasoning\":null,\"sandbox\":\"workspace-write\"}}" >/dev/null
    probe Mutate "{\"op\":\"renameChat\",\"chatId\":\"$id\",\"title\":\"$1\"}" >/dev/null
    probe Mutate "{\"op\":\"setChatBranch\",\"chatId\":\"$id\",\"branch\":\"$3\"}" >/dev/null
    if [[ "$4" != skip ]]; then
      probe QueueCommand "{\"chatId\":\"$id\",\"command\":{\"kind\":\"run\",\"messageId\":\"$(uuidgen | tr 'A-Z' 'a-z')\",\"request\":{\"prompt\":\"$4\",\"model\":null,\"reasoning\":null,\"modelOptions\":{},\"cwd\":\"$RIG/projects/$2\",\"sandbox\":\"workspace-write\",\"autoApprove\":true,\"resume\":null}}}" >/dev/null
    fi
  }
  seed "Port the sidebar sections" noches design/control-plane "Walk me through the streaming pipeline"
  seed "Fix auth token refresh" noches feat/auth-refresh "Find where tokens refresh and fix the race"
  seed "Pricing page copy pass" website main "Tighten the pricing copy"
  seed "Scratch notes" website notes skip
  touch "$RIG/.seeded"
fi

if [[ ! -f "$RIG/access.json" ]]; then
  "$BIN/noches-connect" pair --name "Companion rig" --endpoint "ws://$GATEWAY" \
    --upstream "ws://127.0.0.1:$IPC" --credentials "$RIG/access.json" --out "$RIG/phone.code"
fi
"$BIN/noches-connect" serve --bind "$GATEWAY" --upstream "ws://127.0.0.1:$IPC" \
  --credentials "$RIG/access.json" &
PIDS+=($!)
echo "▸ gateway on ws://$GATEWAY - pairing code:"
cat "$RIG/phone.code"; echo
wait
