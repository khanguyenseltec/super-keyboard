#!/usr/bin/env bash
# Installs the Super Keyboard Vietnamese engine for IBus from source, so the
# physical keyboard types Telex or VNI in every application.
#
#   apps/super-keyboard-ibus/install.sh              build, install, add the input source
#   apps/super-keyboard-ibus/install.sh --uninstall  remove it again
#
# Installing the .deb does the same (except adding the input source). Needs
# sudo for two files: the engine in /usr/libexec and its component file in
# /usr/share/ibus/component, where IBus looks.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
bin=/usr/libexec/super-keyboard-ibus
component=/usr/share/ibus/component/super-keyboard.xml
source="('ibus', 'super-keyboard')"

sources() { gsettings get org.gnome.desktop.input-sources sources 2>/dev/null || echo "@a(ss) []"; }

if [ "${1:-}" = "--uninstall" ]; then
    current="$(sources)"
    if [[ "$current" == *"$source"* ]]; then
        updated="$(python3 -c 'import ast,sys; s=ast.literal_eval(sys.argv[1].removeprefix("@a(ss) ")); print([x for x in s if x != ("ibus","super-keyboard")])' "$current")"
        gsettings set org.gnome.desktop.input-sources sources "$updated"
    fi
    sudo rm -f "$bin" "$component"
    ibus write-cache --system >/dev/null 2>&1 || true
    ibus restart >/dev/null 2>&1 || true
    echo "Removed the Super Keyboard input method."
    exit 0
fi

command -v ibus >/dev/null || { echo "IBus is not installed: sudo apt install ibus" >&2; exit 1; }

echo "Building the engine..."
cargo build --release -p super-keyboard-ibus --manifest-path "$root/Cargo.toml"

echo "Installing $bin and $component (sudo)..."
sudo install -Dm755 "$root/target/release/super-keyboard-ibus" "$bin"
sudo install -Dm644 "$here/data/super-keyboard.xml" "$component"
sudo ibus write-cache --system >/dev/null 2>&1 || true

echo "Restarting IBus..."
ibus restart >/dev/null 2>&1 || true
for _ in $(seq 30); do ibus list-engine 2>/dev/null | grep -q super-keyboard && break; sleep 0.5; done

current="$(sources)"
if [[ "$current" != *"$source"* ]] && command -v gsettings >/dev/null; then
    updated="$(python3 -c 'import ast,sys; s=ast.literal_eval(sys.argv[1].removeprefix("@a(ss) ")); s.append(("ibus","super-keyboard")); print(s)' "$current")"
    gsettings set org.gnome.desktop.input-sources sources "$updated"
    echo "Added \"Vietnamese (Super Keyboard)\" to your input sources."
fi

cat <<'MSG'

Done. Switch input source with Super+Space, or from the indicator in the top
bar. The indicator shows VI or EN and has a menu for Telex / VNI; the same
switches are in the Super Keyboard app (VI button, Tiếng Việt settings), and
the two stay in sync.
MSG
