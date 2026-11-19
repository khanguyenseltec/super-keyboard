# Architecture

Super Keyboard is one Rust engine with one Dioxus UI, running on Ubuntu desktop and
Android. The engine decides *what* to type; small platform "sinks" decide *how* it
reaches the focused application.

## Overview

```text
┌──────────────── crates/config (super-keyboard-config) ────────────────┐
│  settings.toml · layouts/*.toml · themes/*.json  ──▶  UserConfig      │
└──────────────┬────────────────────────────────────────────────────────┘
               │ chosen layout, gesture timing, theme and style
┌──────────────▼──────────────── apps/super-keyboard (Dioxus 0.7) ─────────────────────────┐
│  TopBar · Notices · Preview · SettingsPanel · KeyboardView      (later) Layout editor     │
└──────────────┬──────────────────────────────────────────────────────────▲─────────────────┘
               │ press / move / release (pointer, key, x, time)            │ layer, latches, pressed keys
┌──────────────▼──────────────────── crates/core (super-keyboard-core) ────┴─────────────────┐
│  Gestures (tap, hold, repeat, swipe) ──▶ Trigger ──▶ Keyboard (layers, modifier latches)   │
│  Layout (TOML)                                       ──▶ Output = Text("->") | Key(Ctrl+T) │
└──────────────┬────────────────────────────────────────────────────────────────────────────┘
               │ OutputSink::emit(output)
   ┌───────────┼──────────────────────┬──────────────────────────┬─────────────────────────┐
   ▼           ▼                      ▼                          ▼                         ▼
PreviewSink  Android sink          Linux uinput sink          Linux portal sink          Linux XTest sink
(today)      InputConnection       /dev/uinput + XKB          xdg-desktop-portal         X11 sessions
             (Phase 2)             (Phase 3)                  RemoteDesktop (Phase 3)    (Phase 3)
```

## Repository layout

| Path | What it is |
|---|---|
| `crates/core` | `super-keyboard-core`: layouts, actions, gestures, the `Keyboard` state machine, `Output` and `OutputSink`. No UI, platform or file system dependencies, so it builds and tests anywhere (CI also checks it for Android targets). |
| `crates/core/layouts/` | Built-in layouts. `dev.toml` is the default. |
| `crates/config` | `super-keyboard-config`: the config directory, `settings.toml` (parse, validate, save with comments kept), themes and style overrides, the layout and theme libraries, and reload with last-good fallback. No UI dependency. |
| `crates/config/themes/` | Built-in themes as JSON, and `theme.schema.json` for editors. |
| `apps/super-keyboard-ibus` | `super-keyboard-ibus`: the Vietnamese input method for IBus. Physical-keyboard typing in every app, with the core's Telex and VNI rules and the app's `[vietnamese]` settings. Talks to IBus over D-Bus with `zbus`; `tests/e2e.sh` checks it through a private `ibus-daemon`. |
| `apps/super-keyboard` | The Dioxus app. Cargo features `desktop` (default), `mobile` (Android) and `web` (browser preview for layout work). |
| `docs/` | This document, the [roadmap](ROADMAP.md) and the [customizing guide](CUSTOMIZING.md). |
| *planned* `crates/sink-linux` | uinput, portal and XTest output for Ubuntu. |
| *planned* `android/` | Kotlin `InputMethodService` host for the system keyboard (see below). |

## Core concepts

**Layouts are code, built from sections.** A layout is a TOML file of named sections
(blocks of rows: number row, brackets, letters, the space-bar row) and layers that list
the sections they show; simple layouts can give a layer its rows directly. Keys use a short notation borrowed from Vim: `"a"` and `"->"` insert text,
`"<esc>"` presses a named key, `"<ctrl>"` arms a modifier, `"<layer:sym>"` switches
layer. The table form `{ tap = "<space>", width = 3.0 }` sets widths and labels, and
`{ text = "<div>" }` inserts text that would otherwise parse as a key name. Layout errors
name the layer, row and key. See [`crates/core/layouts/dev.toml`](../crates/core/layouts/dev.toml)
and [`action.rs`](../crates/core/src/action.rs).

**Modifiers latch.** Each modifier cycles `Off → OneShot → Locked → Off` on tap. One-shot
modifiers apply to the next key and release; locked ones stay until tapped again. This is
what makes `Ctrl` then `c` send `Ctrl+C` on a touch screen without holding two keys.

**Output is small and explicit.** The engine produces only two kinds of output:
`Text(String)` and `Key { key, mods }`. A single character with Ctrl, Alt or Super becomes
a chord (`Ctrl+C`); with Shift alone it becomes uppercase text; multi-character text such
as `=>` is inserted as written. Every platform sink only has to handle these two cases.

**Profiles arrange sections; the keyboard runs a keymap.** An `Arrangement` switches
sections on or off and reorders them, storing only overrides, so a layout can gain
sections without breaking saved profiles. `Layout::keymap(&arrangement)` produces the
`Keymap` (layers of concrete rows) that `Keyboard` runs; rearranging builds a new keymap
and the app swaps it in, cancelling presses in progress. One order applies to every
layer, which is why a layer must list its sections in declaration order. Required
sections cannot be switched off, so a profile can never hide the way back to a layer.
Profiles live in `super-keyboard-config`: presets ship as data (`presets.toml`), the
user's profiles and the profile menu's order and visibility (another `Arrangement`) live
in `settings.toml`, and every otherwise unused layout gets a profile automatically.

**Gestures are a clock-free state machine.** `Gestures` turns pointer events into
`Trigger`s: a plain key taps on press; Backspace and the arrows repeat after a delay; a key
with a `hold` action or a `swipe` waits for release, the hold delay or sideways movement.
A new press settles any waiting tap first, so overlapping fingers keep their order. Hosts
pass the time with every event and call `tick` at `next_deadline()`, so the rules are
deterministic in tests and identical on every platform. `Keyboard::trigger` turns each
trigger into output; one-shot modifiers apply to every step of a swipe, which is how Shift
plus a space-bar drag selects.

**Configuration is files.** `super-keyboard-config` reads `settings.toml`,
`layouts/*.toml` and `themes/*.json` from the config directory and never fails: broken files become
`Problem`s the app shows, and on reload a broken file keeps its last good version.
Settings changed in the app are written back with `toml_edit`, changing values in place so
the user's comments survive. The app polls a cheap fingerprint of the directory once a
second on desktop rather than using file notifications, which miss editors that save by
renaming and cost more than they save for a handful of small files. See
[CUSTOMIZING.md](CUSTOMIZING.md).

**Themes are variables.** A theme (JSON) sets the same style variables as `[style]`
in `settings.toml`: colours, font families, sizes and weights, key radius and gap.
What a theme leaves out comes from the built-in theme of its appearance, so every
resolved theme sets every variable. The app writes the active theme, merged with the
user's `[style]` changes, as CSS variables on `.app` (both a light and a dark theme,
behind `prefers-color-scheme`, when following the system), and `main.css` only reads
variables. The keyboard's size is a share of the window set the same way: attributes
on `.app` switch the keyboard to a flex share of the window, and labels size
themselves with container query units.

**Vietnamese typing rewrites output.** `vietnamese::Composer` sits between `Keyboard`
and the sink. It keeps the syllable being typed as letters plus one tone, applies Telex or
VNI keys to it, places the tone from the whole syllable, and sends the difference from what
is on screen as Backspaces and text (`hoa` + `f` sends ⌫ ⌫ `òa`). Anything that is not a
letter ends the syllable and passes through. Because it only ever sends `Text` and
Backspace, it needs nothing from a platform sink.

The physical keyboard takes another route to the same rules. `super-keyboard-ibus` is
an IBus engine: IBus hands it every key in every application, and it uses the composer's
key-level API (`push_key`, `backspace`, `take`) to show the syllable as pre-edit text
and commit it when it ends. It shares `[vietnamese]` with the app through
`settings.toml`, reading it every second and writing it from the desktop's input menu,
so the on-screen and physical keyboards always type the same way.

**Sinks are the platform boundary.** `OutputSink::emit(Output)` is the only interface a
platform backend implements. `PreviewSink` (an in-app text buffer and event log) is the
reference implementation used by the app today and by tests.

## Platform integration: the hard parts

These are the open technical questions. Each gets a spike issue and a short decision
record (added to the table at the end) before the phase that depends on it starts.

### Android: running as the system keyboard

A system keyboard on Android is an `InputMethodService`. It supplies the keyboard view and
types through `InputConnection`: `commitText` for text, `sendKeyEvent` with meta state for
chords (terminal apps such as Termux understand `Ctrl` and `Alt` this way) and
`deleteSurroundingText` for Backspace.

`dx` 0.7 generates a Gradle project whose entry point is an Activity (`WryActivity`), so
today the Dioxus app runs on Android as a normal app, not as a keyboard. Options:

1. **Extend the project `dx` generates.** `Dioxus.toml` can replace `AndroidManifest.xml`
   and `MainActivity.kt` (`android_manifest`, `android_main_activity`). An
   `InputMethodService` declared there would host a WebView with the same UI and call into
   Rust over JNI. The IME also needs `res/xml/method.xml`, which `dx` cannot add today: a
   post-build step or an upstream contribution to Dioxus.
2. **A Gradle project of our own in `android/`.** Build the Rust core with `cargo-ndk`,
   host the Dioxus web build in a WebView inside the IME, and bridge JS ↔ Kotlin ↔ JNI.
   Most control; two build systems.
3. **Native rendering** (Dioxus Native / Blitz) drawing into the IME's surface. Most direct
   path with no WebView, least mature today.

Whichever we choose must meet the latency budget below.

### Ubuntu desktop: typing into other windows

Ubuntu uses GNOME on Wayland by default, which deliberately stops ordinary apps from
injecting input or staying unfocused.

- **Focus.** Clicking a normal window focuses it, so keystrokes would land in the keyboard
  itself. Candidates: an X11/XWayland window with `WM_HINTS.input = false` and a
  dock/utility window type; `wlr-layer-shell` on KDE and wlroots compositors (not GNOME);
  a GNOME Shell extension; or the phone-as-keypad mode below, which has no focus problem.
- **Injection.**
  - *uinput*: a virtual input device. Works on Wayland, X11 and the console, but needs
    write access to `/dev/uinput` (a udev rule and group membership).
  - *xdg-desktop-portal RemoteDesktop* (via `ashpd`): no setup, but GNOME asks the user to
    allow it each session.
  - *XTest*: X11 sessions only.
- **Characters vs keycodes.** uinput sends keycodes, not characters, so text must be mapped
  through the active XKB layout (`xkbcommon`). Characters that are not on the layout need a
  fallback such as GTK/IBus Unicode entry (`Ctrl+Shift+U`) or the portal.

### Phone as a dev keypad for the desktop (later)

The Android app sends `Output` over the local network to a small desktop service that uses
the uinput sink. This sidesteps the focus problem entirely. It needs pairing and an
encrypted channel and is planned after both platforms work on their own.

## Privacy and security

A keyboard sees passwords, tokens and private messages. These rules are not optional:

- **No network in the keyboard.** Release builds must not request network access. (The
  Android manifest that `dx` generates adds `INTERNET` for development hot-reload; release
  builds must remove it.)
- **No keystroke logging.** Typed text is never written to logs, files or analytics. The
  preview panel's log exists only inside the running app.
- **History is opt-in and local.** Clipboard and snippet history are off by default, kept
  in memory, and disabled in password fields (Android `TYPE_TEXT_VARIATION_PASSWORD`,
  `IME_FLAG_NO_PERSONALIZED_LEARNING`).
- **Layouts are data.** Layout files can insert text and press keys; they can never run
  code. Imported layouts are shown before they are used.
- **Styles and themes are data too.** Theme files and style overrides in
  `settings.toml` are CSS values, but only colours, lengths, maths functions, font
  weights and the names of installed fonts: anything that could load a resource
  (`url()`, `image-set()`, `@import`, `@font-face`) or break out of a declaration
  (quotes, `;`, braces, backslashes) is rejected, and font names are quoted by the app,
  so a theme can never exfiltrate what the keyboard shows. Fonts are never downloaded.
- **Desktop injection is powerful.** A uinput device can type into any window, including
  `sudo` prompts. The desktop output service accepts input only from the local UI, and
  later from devices the user has explicitly paired.

Report vulnerabilities privately; see [SECURITY.md](../SECURITY.md).

## Performance budget

From touch to `OutputSink::emit` in under 16 ms (one frame at 60 Hz) on a mid-range
Android phone. The WebView path is the main risk; it is measured in the Phase 2 spike
before the IME is built on it.

## Decisions

| # | Decision | Why |
|---|---|---|
| 1 | Rust + Dioxus 0.7 (latest stable) for UI on both platforms | One codebase and one UI; Dioxus targets Linux desktop and Android. Revisit 0.8 when it is stable. |
| 2 | Engine in a UI-free core crate | Testable anywhere; reusable from an IME host, a desktop service or another UI. |
| 3 | TOML layouts with Vim-style `<key>` notation | Readable diffs, easy sharing, familiar to developers. |
| 4 | MIT OR Apache-2.0 | The Rust ecosystem default, same as Dioxus. |
| 5 | Android IME host | *Open, Phase 2 spike.* |
| 6 | Ubuntu focus and injection strategy | *Open, Phase 3 spike.* |
| 7 | Gesture timing in the core, with the clock passed in | Same long-press and repeat feel on every front end, including a native Android host; fully testable without timers. |
| 8 | Configuration in its own crate, polled for changes | Front ends share one meaning of every setting; polling a fingerprint is simpler and more robust than file notifications for a few small files. |
| 9 | `settings.toml` saved with `toml_edit` | Users edit the same file by hand; saving from the app must keep their comments and formatting. |
| 10 | Sections with one order for every layer | One list to arrange instead of one per layer; shared rows such as the space-bar row stay in the same place on every layer. |
| 11 | Arrangements store overrides; listed items trade places | Saved profiles survive layouts gaining sections; a partial order changes only what it names. |
| 12 | Presets as data, changed presets as user copies | Presets can be improved in new releases while "reset to preset" stays one step; no code needed to add one. |
