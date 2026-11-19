#!/usr/bin/env bash
# End-to-end test of super-keyboard-ibus through a real, private ibus-daemon.
#
# A private ibus-daemon on its own socket, cut off from the desktop's session
# bus and dconf, with a temporary config directory: the desktop's IBus, input
# sources and settings are left alone.
#
#   apps/super-keyboard-ibus/tests/e2e.sh [component|register] [path/to/super-keyboard-ibus]
#
# component (the default) installs the engine as IBus sees an installed one: a
# component file IBus reads, and IBus starts the engine itself with --ibus.
# register runs the engine first and registers it with the running IBus.
#
# Needs ibus-daemon and python3-gi with IBus (both on a standard Ubuntu desktop).
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
mode="${1:-component}"
engine="${2:-$here/../../../target/debug/super-keyboard-ibus}"
engine="$(realpath "$engine")"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# The packaged component file, pointed at this build of the engine.
mkdir -p "$tmp/component"
sed "s|/usr/libexec/super-keyboard-ibus|$engine|" "$here/../data/super-keyboard.xml" \
  > "$tmp/component/super-keyboard.xml"

mkdir -p "$tmp/config/super-keyboard"
cat > "$tmp/config/super-keyboard/settings.toml" <<'TOML'
# kept: the engine must not lose this comment
[vietnamese]
enabled = true
method = "telex"
TOML

export XDG_CONFIG_HOME="$tmp/config" XDG_CACHE_HOME="$tmp/cache"
export IBUS_ADDRESS="unix:path=$tmp/ibus.sock"
# No session bus and no dconf: nothing can reach the desktop.
export DBUS_SESSION_BUS_ADDRESS="unix:path=$tmp/no-session-bus" GSETTINGS_BACKEND=memory
export E2E_MODE="$mode"
if [ "$mode" = component ]; then
  # Only our component: IBus reads this directory instead of /usr/share/ibus/component.
  export IBUS_COMPONENT_PATH="$tmp/component"
fi

bash -c '
  ibus-daemon --address="$IBUS_ADDRESS" --panel=disable --emoji-extension=disable \
    --config=disable --cache=none --single >"$0/daemon.log" 2>&1 &
  daemon=$!
  for _ in $(seq 50); do [ -S "${IBUS_ADDRESS#unix:path=}" ] && break; sleep 0.1; done
  engine=
  if [ "$E2E_MODE" = register ]; then
    "$1" --register >"$0/engine.log" 2>&1 &
    engine=$!
    sleep 1
  fi
  status=0
  python3 "$2" || status=$?
  kill $engine "$daemon" 2>/dev/null || true
  if [ "$status" != 0 ]; then
    echo "--- engine log"; cat "$0/engine.log" 2>/dev/null || true
    echo "--- daemon log"; tail -20 "$0/daemon.log"
  fi
  exit "$status"
' "$tmp" "$engine" "$here/e2e.py"
