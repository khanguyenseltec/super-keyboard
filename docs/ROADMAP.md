# Roadmap

Each phase is a GitHub milestone, and each unchecked item is (or will be) an issue.
Items marked **good first issue** are a good way in. Phases are ordered, but anyone can
start on a later item, especially the spikes.

## Phase 0: Foundation (done)

- [x] Cargo workspace with `super-keyboard-core` (layouts, modifiers, layers, output)
- [x] Built-in developer layout `dev`: base, `sym` and `nav` layers
- [x] Dioxus app rendering the layout with an in-app preview of what would be typed
- [x] CI: format, lint, tests, and core builds for Android targets
- [x] README, contributing guide, architecture notes, this roadmap

## Phase 1: MVP keyboard (in-app), milestone `v0.1`

**Done when** a developer can comfortably type a Rust function and Vim commands into the
preview, on Ubuntu desktop and on an Android phone (running as an app).

- [ ] Run on Ubuntu 22.04 and 24.04 with `dx serve --desktop`; fix anything that only shows up in a real window
- [ ] Run on an Android device with `dx serve --android` and document the setup
- [x] Key repeat while holding Backspace, Delete and arrow keys
- [x] Hold actions in layout files (`hold = "..."`), for example long-press `.` for `..`
- [x] Swipe along the space bar to move the cursor (Shift selects)
- [x] Load user layouts from the config directory, reload on change, and show layout errors in the app
- [x] `settings.toml` and a settings screen: theme, long-press and repeat timing, swipe step, key size, style overrides
- [x] Modular layouts: sections switched on or off and reordered per profile; presets; a profile menu that can be reordered and trimmed
- [x] Light theme and theme variables
- [x] Responsive key sizing for phone portrait and landscape, tablet and desktop
- [x] Property tests for the engine with `proptest`

## Phase 2: Android system keyboard, milestone `v0.2`

**Done when** Super Keyboard can be enabled in Android settings and types into any app,
including Ctrl and Alt chords in Termux.

- [ ] Spike and decision: host the Dioxus UI inside an `InputMethodService` ([options](ARCHITECTURE.md#android-running-as-the-system-keyboard))
- [ ] Android sink: `commitText`, `sendKeyEvent` with meta state, `deleteSurroundingText`
- [ ] Respect the editor: password fields, the Enter action (Go, Send, Search), input types
- [ ] Onboarding: enable the keyboard, pick a layout
- [ ] Measure touch-to-commit latency against the 16 ms budget
- [ ] Signed release APK built by GitHub Actions; remove the `INTERNET` permission from release builds

## Phase 3: Ubuntu desktop keyboard, milestone `v0.3`

**Done when**, on Ubuntu 24.04 with GNOME (Wayland and X11), the keyboard types into a
focused terminal or editor without taking focus.

- [ ] Spike and decision: a keyboard window that never takes focus ([options](ARCHITECTURE.md#ubuntu-desktop-typing-into-other-windows))
- [ ] uinput sink with XKB text mapping, plus a udev rule and setup docs
- [ ] xdg-desktop-portal RemoteDesktop sink as the no-setup fallback
- [ ] XTest sink for X11 sessions
- [ ] Always on top, dock to a screen edge, show/hide shortcut
- [ ] `.deb` package from `dx bundle`

## Phase 4: Developer power features, milestone `v0.4`

- [ ] Snippets with placeholders and language profiles (Rust, Python, TypeScript, shell, Git, Vim)
- [ ] Auto-pair brackets and quotes (toggle)
- [ ] Record and replay macros
- [ ] Per-app profiles: switch profile by Android package name or desktop window class
- [ ] Clipboard history: opt-in, in memory, off in password fields
- [ ] Command palette key: fuzzy search over snippets and commands
- [ ] Visual layout editor that writes TOML

## Later

- Phone as a dev keypad and macro pad for the desktop, over a paired, encrypted local link
- Community layout gallery and import (layouts are data, never code)
- Windows, macOS and iOS (Dioxus supports them; out of scope until Ubuntu and Android are solid)
- Upgrade to Dioxus 0.8 when it is stable
