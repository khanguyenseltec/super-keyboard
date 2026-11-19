"""End-to-end test of the IBus engine, through a real ibus-daemon.

Run it with ./e2e.sh, which starts a private session bus, ibus-daemon and the
engine, so the desktop's own IBus and settings are never touched. This script
is the application side: it types keys into an IBus input context with
libibus, the library GTK and Qt applications use, and checks what arrives.
"""

import os
import sys
import time
import pathlib

import warnings

import gi

# PyGObject warns about IBusText's floating references in signal handlers;
# the test client's own quirk, not the engine's.
warnings.filterwarnings("ignore", message=".*g_object_is_floating.*")

gi.require_version("IBus", "1.0")
from gi.repository import GLib, IBus  # noqa: E402

SETTINGS = pathlib.Path(os.environ["XDG_CONFIG_HOME"]) / "super-keyboard" / "settings.toml"
RETURN, BACKSPACE, CONTROL = 0xFF0D, 0xFF08, 1 << 2

IBus.init()
bus = IBus.Bus()
assert bus.is_connected(), "cannot reach the private ibus-daemon"
ctx = bus.create_input_context("super-keyboard-e2e")
ctx.set_capabilities(IBus.Capabilite.PREEDIT_TEXT | IBus.Capabilite.FOCUS | IBus.Capabilite.PROPERTY)

committed = []
preedit = [""]
props = {}
ctx.connect("commit-text", lambda _c, text: committed.append(text.get_text()))
ctx.connect("update-preedit-text", lambda _c, text, _pos, visible: preedit.__setitem__(0, text.get_text() if visible else ""))
ctx.connect("hide-preedit-text", lambda _c: preedit.__setitem__(0, ""))


def on_props(_c, prop_list):
    props.clear()
    i = 0
    while (prop := prop_list.get(i)) is not None:
        props[prop.get_key()] = prop
        i += 1


ctx.connect("register-properties", on_props)


def pump(seconds=0.15):
    end = time.time() + seconds
    while time.time() < end:
        while GLib.MainContext.default().iteration(False):
            pass
        time.sleep(0.01)


failures = []


def check(name, got, want):
    ok = got == want
    print(("PASS " if ok else "FAIL ") + f"{name}: {got!r}" + ("" if ok else f" (want {want!r})"))
    if not ok:
        failures.append(name)


def type_keys(keys, state=0):
    """Types keys; returns (text the application ends up with, pre-edit)."""
    committed.clear()
    text = ""
    for ch in keys:
        keyval = {"\n": RETURN, "\b": BACKSPACE}.get(ch, ord(ch))
        handled = ctx.process_key_event(keyval, 0, state)
        pump(0.03)
        text += "".join(committed)
        committed.clear()
        if not handled:
            text += {"\n": "⏎", "\b": "⌫"}.get(ch, ch)
        ctx.process_key_event(keyval, 0, state | IBus.ModifierType.RELEASE_MASK)
    pump()
    return text + "".join(committed), preedit[0]


if os.environ.get("E2E_MODE") == "component":
    listed = [e.get_name() for e in bus.list_engines()]
    check("component file lists the engine", "super-keyboard" in listed, True)
    desc = bus.get_engines_by_names(["super-keyboard"])
    check("language and name", [(d.get_language(), d.get_longname()) for d in desc], [("vi", "Vietnamese (Super Keyboard)")])

ctx.focus_in()
# As GNOME does when you pick the input source. (An engine registered at run
# time is not in ListEngines, so a per-context SetEngine cannot find it.)
check("global engine set", bus.set_global_engine("super-keyboard"), True)
pump(1.0)
engine = ctx.get_engine()
check("engine selected", engine.get_name() if engine else None, "super-keyboard")
check("indicator shows VI", props.get("InputMode") and props["InputMode"].get_symbol().get_text(), "VI")

# Telex.
check("telex sentence", type_keys("Xin chaof cacs banj. Tieesng Vieejt\n"), ("Xin chào các bạn. Tiếng Việt⏎", ""))
check("pre-edit while typing", type_keys("nguwowif"), ("", "người"))
check("space commits the syllable", type_keys(" "), ("người ", ""))
check("backspace edits the syllable", type_keys("hoanf\b"), ("", "hòa"))
check("backspace then belongs to the app", type_keys("\b\b\b\b"), ("⌫", ""))
check("shortcut commits first", type_keys("as") + type_keys("s", CONTROL), ("", "á", "ás", ""))
check("code passes through", type_keys("for class print\n"), ("for class print⏎", ""))

# The app switches to VNI in settings.toml: the engine follows.
SETTINGS.write_text(SETTINGS.read_text().replace('method = "telex"', 'method = "vni"'))
pump(1.6)
check("follows the app to VNI", type_keys("Vie65t Nam\n"), ("Việt Nam⏎", ""))
check("menu follows the app", props.get("Method.vni") and props["Method.vni"].get_state(), IBus.PropState.CHECKED)

# The desktop's input menu switches to English: the app's settings follow.
ctx.property_activate("InputMode", IBus.PropState.UNCHECKED)
pump(0.5)
saved = SETTINGS.read_text()
check("menu writes enabled = false", "enabled = false" in saved, True)
check("settings keep their comments", saved.startswith("# kept"), True)
check("indicator shows EN", props["InputMode"].get_symbol().get_text(), "EN")
check("English passes keys", type_keys("vie65t"), ("vie65t", ""))

ctx.focus_out()
pump()
print("ALL PASSED" if not failures else f"{len(failures)} FAILED: {failures}")
sys.exit(1 if failures else 0)
