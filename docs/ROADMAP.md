# Roadmap

**Where Super Keyboard is going:** the keyboard you reach for when a touchscreen is all
you have and you still need to write code, on your phone, your tablet and your Linux
desktop. It types into every app, in English and in Vietnamese. Layouts, themes and
settings are files you own, and it never phones home.

Each milestone is a [GitHub milestone](https://github.com/tristanseltec/super-keyboard/milestones),
and each unchecked item is (or will be) an issue. Items marked **good first issue** are a
good way in. Milestones are ordered, but anyone can start on a later item, especially the
spikes. Ideas and priorities are discussed in
[Discussions](https://github.com/tristanseltec/super-keyboard/discussions).

## Principles

Every item below has to keep these true:

1. **Private by design.** No network access, no telemetry, no logging of typed text.
   History of any kind is opt-in, in memory, and off in password fields.
2. **One engine.** Typing rules live once, in `crates/core`, tested on any machine, and
   every front end (the app, the Android keyboard, the IBus engine) uses them.
3. **Files you own.** Layouts, themes and settings are plain files that are data, never
   code, and a mistake in one never breaks the keyboard.
4. **Fast.** A key reaches the app in under 16 ms; the keyboard never makes you wait.
5. **For everyone.** Accessible (contrast, sizes, screen readers) and multilingual, with
   Vietnamese as a first-class language, not an afterthought.

## At a glance

| Milestone | Goal | Status |
|---|---|---|
| **v0.1** MVP keyboard | Type a Rust function and Vim commands in the app, on Ubuntu and Android | 🟢 Features done; first device runs open |
| **v0.1** Tiếng Việt | Telex and VNI on the on-screen keys, and on the physical keyboard in every Ubuntu app | ✅ Done |
| **v0.2** Android system keyboard | Enable it in Android settings; type into any app, Termux chords and Vietnamese included | 🔜 Next |
| **v0.3** Ubuntu desktop keyboard | Type into the focused app on GNOME (Wayland and X11) without taking focus | 🗺️ Planned |
| **v0.4** Power features | Snippets, auto-pairs, macros, per-app profiles, a visual layout editor | 🗺️ Planned |
| **v0.5** Better Vietnamese | Shorthand, English auto-restore, per-app VI / EN, more methods, a Vietnamese UI | 🗺️ Planned |
| **v1.0** Ready for everyone | Signed packages in the usual stores, stable file formats, accessibility and security reviews | 🗺️ Planned |

## v0.1: MVP keyboard (in the app)

**Done when** a developer can comfortably type a Rust function and Vim commands into the
preview, on Ubuntu desktop and on an Android phone (running as an app).

- [ ] Run on Ubuntu 22.04, 24.04 and 26.10 with `dx serve --desktop`; fix anything that only shows up in a real window ([#1](https://github.com/tristanseltec/super-keyboard/issues/1))
- [ ] Run on an Android device with `dx serve --android` and document the setup ([#2](https://github.com/tristanseltec/super-keyboard/issues/2))
- [x] Developer layout `dev`: base, `sym` and `nav` layers; one-shot and locked modifiers
- [x] Key repeat, hold actions (`hold = "..."`), space-bar cursor swipe (Shift selects)
- [x] Layouts, settings and themes as files, reloaded on change, with located errors
- [x] Modular layouts: sections switched on or off and reordered per profile; presets; a profile menu
- [x] 24 themes as JSON with their own colours and fonts; separate light and dark themes when following the system
- [x] Keyboard width, height and position as a share of the window, up to a full-window keyboard
- [x] Responsive key sizing for phone portrait and landscape, tablet and desktop
- [x] Property tests for the engine with `proptest`

## v0.1: Tiếng Việt (done)

**Done when** Vietnamese can be typed with Telex or VNI on the on-screen keys, and with the
physical keyboard in any Ubuntu app, from one set of settings.

- [x] Telex and VNI composer in the engine: marks in any order, the tone moving with the syllable, undo by repeating a key
- [x] Classic (`hòa`) or modern (`hoà`) tone placement, and a spelling check that leaves English and code alone
- [x] VI / EN switch in the top bar, a Tiếng Việt settings tab with live cheat sheets and a box to try keys in
- [x] IBus engine for the physical keyboard in every app, with a VI / EN menu in GNOME's top bar
- [x] One `[vietnamese]` section in `settings.toml`, followed both ways by the app and the engine
- [x] Shipped in the `.deb`; end-to-end tests through a real `ibus-daemon` in CI

## v0.2: Android system keyboard

**Done when** Super Keyboard can be enabled in Android settings and types into any app,
including Ctrl and Alt chords in Termux, and Vietnamese.

- [ ] Spike and decision: host the Dioxus UI inside an `InputMethodService` ([#10](https://github.com/tristanseltec/super-keyboard/issues/10), [options](ARCHITECTURE.md#android-running-as-the-system-keyboard))
- [ ] Android sink: `commitText`, `sendKeyEvent` with meta state, `deleteSurroundingText` ([#11](https://github.com/tristanseltec/super-keyboard/issues/11))
- [ ] Vietnamese through `setComposingText`, so the syllable is underlined in the app as on the desktop ([#21](https://github.com/tristanseltec/super-keyboard/issues/21))
- [ ] Respect the editor: password fields, the Enter action (Go, Send, Search), input types ([#12](https://github.com/tristanseltec/super-keyboard/issues/12))
- [ ] Onboarding: enable the keyboard, pick a profile, try Vietnamese ([#22](https://github.com/tristanseltec/super-keyboard/issues/22))
- [ ] Measure touch-to-commit latency against the 16 ms budget ([#23](https://github.com/tristanseltec/super-keyboard/issues/23))
- [ ] Haptic and sound feedback on key press (each a setting, off by default for sound) ([#24](https://github.com/tristanseltec/super-keyboard/issues/24))

## v0.3: Ubuntu desktop keyboard

**Done when**, on Ubuntu 24.04 and later with GNOME (Wayland and X11), the on-screen
keyboard types into a focused terminal or editor without taking focus.

- [ ] Spike and decision: a keyboard window that never takes focus ([#13](https://github.com/tristanseltec/super-keyboard/issues/13), [options](ARCHITECTURE.md#ubuntu-desktop-typing-into-other-windows))
- [ ] **IBus sink**: send the on-screen keyboard's text and keys to `super-keyboard-ibus`, which already types into the focused app on Wayland and X11, with no root and no udev rule ([#25](https://github.com/tristanseltec/super-keyboard/issues/25))
- [ ] uinput sink with XKB text mapping, for keys IBus cannot send (chords to the compositor) ([#14](https://github.com/tristanseltec/super-keyboard/issues/14))
- [ ] xdg-desktop-portal RemoteDesktop sink as a no-setup fallback ([#15](https://github.com/tristanseltec/super-keyboard/issues/15))
- [ ] XTest sink for X11 sessions ([#26](https://github.com/tristanseltec/super-keyboard/issues/26))
- [ ] Always on top, dock to a screen edge, show/hide shortcut and a panel icon ([#27](https://github.com/tristanseltec/super-keyboard/issues/27))
- [ ] Light up on-screen keys as the physical keyboard is pressed, so both keyboards feel like one ([#28](https://github.com/tristanseltec/super-keyboard/issues/28))
- [x] `.deb` package from `dx bundle`, with the IBus engine, app icons and maintainer scripts

## v0.4: Developer power features

- [ ] Snippets with placeholders and language profiles: Rust, Python, TypeScript, shell, Git, Vim ([#16](https://github.com/tristanseltec/super-keyboard/issues/16))
- [ ] Auto-pair brackets and quotes, as a toggle (**good first issue**, [#17](https://github.com/tristanseltec/super-keyboard/issues/17))
- [ ] Record and replay macros ([#29](https://github.com/tristanseltec/super-keyboard/issues/29))
- [ ] Per-app profiles: switch profile by Android package name or desktop window class ([#30](https://github.com/tristanseltec/super-keyboard/issues/30))
- [ ] Clipboard history: opt-in, in memory, off in password fields ([#31](https://github.com/tristanseltec/super-keyboard/issues/31))
- [ ] Command palette key: fuzzy search over snippets and commands ([#32](https://github.com/tristanseltec/super-keyboard/issues/32))
- [ ] Visual layout editor that writes TOML ([#33](https://github.com/tristanseltec/super-keyboard/issues/33))

## v0.5: Better Vietnamese

For everyone who types Vietnamese all day, and for the Vietnamese developer community.

- [ ] **Gõ tắt** (shorthand): your own abbreviations that expand as you type, such as `vn` → `Việt Nam`, in `settings.toml` ([#34](https://github.com/tristanseltec/super-keyboard/issues/34))
- [ ] Auto-restore English: a word that cannot be Vietnamese goes back to the keys typed when it ends (`tét` → `test`) ([#35](https://github.com/tristanseltec/super-keyboard/issues/35))
- [ ] Remember VI or EN per app on the desktop (a terminal in EN, a chat app in VI) ([#36](https://github.com/tristanseltec/super-keyboard/issues/36))
- [x] More methods: VIQR, and Telex options (no lone `w` → `ư`, brackets for `ơ` and `ư`) ([#37](https://github.com/tristanseltec/super-keyboard/issues/37), [#50](https://github.com/tristanseltec/super-keyboard/pull/50))
- [ ] Output in NFC (default) or NFD Unicode, for the rare app that needs it (**good first issue**, [#38](https://github.com/tristanseltec/super-keyboard/issues/38))
- [ ] A Vietnamese phone layout section: `ă â đ ê ô ơ ư` one tap away, for typing without Telex or VNI (**good first issue**, [#39](https://github.com/tristanseltec/super-keyboard/issues/39))
- [ ] The app's interface in Vietnamese, the first of its translations ([#40](https://github.com/tristanseltec/super-keyboard/issues/40))
- [ ] An Fcitx5 add-on beside the IBus engine, for KDE and other desktops ([#41](https://github.com/tristanseltec/super-keyboard/issues/41))

## v1.0: Ready for everyone

**Done when** someone who has never heard of Rust can install it from the place they install
everything else, and it keeps working after updates.

- [ ] Releases from CI: signed `.deb`, AppImage and APK on GitHub Releases, with checksums and notes ([#42](https://github.com/tristanseltec/super-keyboard/issues/42))
- [ ] Ubuntu PPA and Flathub; F-Droid and Google Play, without the `INTERNET` permission ([#43](https://github.com/tristanseltec/super-keyboard/issues/43))
- [ ] Stable layout, theme and settings formats, with versioning and automatic migration ([#44](https://github.com/tristanseltec/super-keyboard/issues/44))
- [ ] Accessibility review: screen readers, large keys, high-contrast themes, reduced motion ([#45](https://github.com/tristanseltec/super-keyboard/issues/45))
- [ ] Security review of the IBus engine, the sinks and the file loaders; fuzzing of the parsers ([#46](https://github.com/tristanseltec/super-keyboard/issues/46))
- [ ] A documentation site with guides, the key notation and a theme gallery ([#47](https://github.com/tristanseltec/super-keyboard/issues/47))
- [ ] Performance budget in CI: start-up time, memory and key latency ([#48](https://github.com/tristanseltec/super-keyboard/issues/48))

## Later

- Your phone as a dev keypad and macro pad for the desktop, over a paired, encrypted local link
- A community gallery of layouts and themes, imported in one tap (they are data, never code)
- Windows, macOS and iOS: Dioxus supports them; out of scope until Ubuntu and Android are solid
- Upgrade to Dioxus 0.8 when it is stable

## Not planned

To keep the keyboard fast, private and small, these are deliberately out of scope:
cloud sync, accounts, ads, word prediction that needs a server, and anything that sends
what you type off your device.
