<div align="center">

# Super Keyboard

**A virtual keyboard made for programmers, built in Rust with Dioxus, for Ubuntu desktop and Android.**

[![CI](https://github.com/tristanseltec/super-keyboard/actions/workflows/ci.yml/badge.svg)](https://github.com/tristanseltec/super-keyboard/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![Contributors welcome](https://img.shields.io/badge/contributors-welcome-brightgreen.svg)](CONTRIBUTING.md)

</div>

---

Phone keyboards hide brackets three taps deep and have no Esc, Tab, Ctrl or arrow keys.
Desktop on-screen keyboards are built for accessibility, not for code. Super Keyboard puts
the keys developers use (brackets, operators, Esc, Tab, Ctrl, Alt, arrows, F-keys) where
they can reach them, and lets you define your own layouts as code.

Use it to SSH into a server from your phone, edit in Vim or Helix on a tablet, drive a
terminal on a touchscreen laptop, or keep a programmable symbol pad next to your editor.

> **Status: pre-alpha.** The engine and the app shell are in place: the app renders the
> developer layout and types into its own preview panel. Typing into *other*
> applications (Android system keyboard, Ubuntu input) is the next big step. See the
> [roadmap](docs/ROADMAP.md). **Collaborators are very welcome.**

## Features

| Feature | Status |
|---|---|
| Developer layout: brackets, Esc and Tab on the base layer; symbols, operators (`->` `=>` `::` `&&`), navigation and F-keys one tap away | Done |
| One-shot and locked modifiers: tap `ctrl` then `c` to send `Ctrl+C`; tap twice to lock | Done |
| Layers and layouts defined in TOML ("keyboard as code"), with clear errors | Done |
| Dioxus UI for Ubuntu desktop and Android, with an in-app preview | Built and render-tested in CI; first device runs are Phase 1 |
| Key repeat, long-press actions, space-bar cursor swipe, user layouts with live reload, themes | Phase 1 |
| Android system keyboard (IME) that types into any app, including Ctrl/Alt in Termux | Phase 2 |
| Ubuntu desktop keyboard that types into other windows (uinput, portal, XTest) | Phase 3 |
| Snippets per language, auto-pairs, macros, per-app profiles, clipboard history (opt-in) | Phase 4 |
| Phone as a dev keypad for your desktop | Later |

## The `dev` layout

```text
 base                                         sym                               nav
 esc tab  (   )   {   }   [   ]   ;   :       1  2  3  4  5  6  7  8  9  0      F1 … F12
  q   w   e   r   t   y   u   i   o   p       !  @  #  $  %  ^  &  *  -  =      esc ins home  ↑  end pgup
  a   s   d   f   g   h   j   k   l   _       `  ~  |  \  '  "  <  >  /  ?      tab del   ←   ↓   →  pgdn
  ⇧    z   x   c   v   b   n   m     ⌫        ⇧  -> => :: != == && ||   ⌫       ⇧ super ./ ../ ~/ $( ${ ⌫
 ctrl sym alt [   space   ]  ,   .  nav ⏎     (same bottom row on every layer)
```

Layouts are TOML. This is all it takes to define a layer:

```toml
[[layers]]
name = "base"
rows = [
    ["<esc>", "<tab>", "(", ")", "{", "}", "[", "]", ";", ":"],
    ["<ctrl>", "<layer:sym>", { tap = "<space>", width = 3.0 }, "->", "<enter>"],
]
```

`"->"` inserts text, `"<esc>"` presses a key, `"<ctrl>"` arms a modifier and
`"<layer:sym>"` switches layer. The full notation is documented at the top of
[`crates/core/layouts/dev.toml`](crates/core/layouts/dev.toml).

## Getting started

You need Rust (stable, 1.88 or newer).

```sh
git clone https://github.com/tristanseltec/super-keyboard
cd super-keyboard

# Engine tests and a UI render test. Needs no system libraries.
cargo test --workspace --no-default-features
```

### Run on Ubuntu desktop

```sh
# System libraries for the Dioxus desktop webview (Ubuntu 22.04 / 24.04)
sudo apt install build-essential libwebkit2gtk-4.1-dev libgtk-3-dev \
  libayatana-appindicator3-dev libxdo-dev libssl-dev librsvg2-dev

# The Dioxus CLI, pinned to the Dioxus version this project uses
cargo install dioxus-cli --version 0.7.10 --locked

dx serve -p super-keyboard --desktop
```

### Run on Android

Install Android Studio (or the command-line tools) with an SDK, the NDK and a JDK 17, and
connect a device with USB debugging on, or start an emulator.

```sh
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
export ANDROID_HOME="$HOME/Android/Sdk"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/<version>"
export JAVA_HOME="<path to JDK 17>"   # Android Studio ships one in android-studio/jbr

dx serve -p super-keyboard --android
```

For now this runs Super Keyboard as an app. Running it as the system keyboard is
[Phase 2](docs/ROADMAP.md#phase-2-android-system-keyboard-milestone-v02).

## How it works

```text
apps/super-keyboard  (Dioxus UI)  ──tap──▶  crates/core  (layouts, layers, modifiers)
                                                 │
                                         Output: Text("->") | Key(Ctrl+Shift+T)
                                                 ▼
                       OutputSink: preview today; Android InputConnection, Linux uinput / portal / XTest next
```

The engine has no UI or platform code, so it is tested on any machine and shared by every
front end. Each platform only implements one small trait. The hard parts, Android IME
hosting and typing into other windows on GNOME Wayland, are described with their options
in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Privacy

A keyboard sees everything you type, so Super Keyboard is built to be trusted: no network
access in the keyboard, no telemetry, no logging of typed text, and any history is opt-in,
local and off in password fields. See [Privacy and security](docs/ARCHITECTURE.md#privacy-and-security)
and [SECURITY.md](SECURITY.md).

## Contributing

Super Keyboard is looking for collaborators. Especially useful right now:

- **Android developers** (Kotlin, `InputMethodService`, JNI) for the system keyboard
- **Linux desktop developers** (Wayland, GNOME, uinput, XKB) for typing into other windows
- **Rust and Dioxus developers** for the engine and UI
- **Keyboard and layout nerds** for the developer layouts
- **Testers** with Android devices and Ubuntu machines

Start with the [contributing guide](CONTRIBUTING.md), pick an issue labelled
[`good first issue`](https://github.com/tristanseltec/super-keyboard/labels/good%20first%20issue)
or [`help wanted`](https://github.com/tristanseltec/super-keyboard/labels/help%20wanted),
or open a [discussion](https://github.com/tristanseltec/super-keyboard/discussions) to
propose an idea or a layout.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for
inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual
licensed as above, without any additional terms or conditions.
