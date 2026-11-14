# Architecture

Super Keyboard is one Rust engine with one Dioxus UI, running on Ubuntu desktop and
Android. The engine decides *what* to type; small platform "sinks" decide *how* it
reaches the focused application.

## Overview

```text
┌──────────────────────────── apps/super-keyboard (Dioxus 0.7) ─────────────────────────────┐
│  KeyboardView · Preview · (later) Settings · Layout editor                                │
└──────────────┬──────────────────────────────────────────────────────────▲─────────────────┘
               │ tap(row, col)                                             │ layer, modifier latches
┌──────────────▼──────────────────── crates/core (super-keyboard-core) ────┴─────────────────┐
│  Layout (TOML)  ──▶  Keyboard state machine  ──▶  Output = Text("->") | Key(Ctrl+Shift+T)   │
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
| `crates/core` | `super-keyboard-core`: layouts, actions, the `Keyboard` state machine, `Output` and `OutputSink`. No UI and no platform dependencies, so it builds and tests anywhere (CI also checks it for Android targets). |
| `crates/core/layouts/` | Built-in layouts. `dev.toml` is the default. |
| `apps/super-keyboard` | The Dioxus app. Cargo features `desktop` (default), `mobile` (Android) and `web` (browser preview for layout work). |
| `docs/` | This document and the [roadmap](ROADMAP.md). |
| *planned* `crates/sink-linux` | uinput, portal and XTest output for Ubuntu. |
| *planned* `android/` | Kotlin `InputMethodService` host for the system keyboard (see below). |

## Core concepts

**Layouts are code.** A layout is a TOML file: a list of layers, each a list of rows of
keys. Keys use a short notation borrowed from Vim: `"a"` and `"->"` insert text,
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
