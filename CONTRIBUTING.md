# Contributing to Super Keyboard

Thanks for helping. Super Keyboard is early, so there is room to shape it: code, layouts,
testing on real devices and design discussion all count.

## Where to start

- Read the [README](README.md), the [architecture notes](docs/ARCHITECTURE.md) and the
  [roadmap](docs/ROADMAP.md).
- Pick an issue labelled `good first issue` or `help wanted`, and comment that you are
  taking it so nobody duplicates the work.
- For something bigger or not on the roadmap, open a
  [discussion](https://github.com/tristanseltec/super-keyboard/discussions) or an issue
  first so we can agree on the approach.
- The `spike` issues (Android IME hosting, Ubuntu focus and injection) are research tasks.
  A short write-up of what you tried and measured is a valuable contribution even without
  production code.

## Development setup

You need stable Rust (1.88 or newer). Everything below runs from the repository root.

```sh
cargo test --workspace --no-default-features   # engine + UI render test, no system libraries
cargo fmt --all
cargo clippy --workspace --all-targets --no-default-features
```

To run the app on Ubuntu desktop or Android, follow
[Getting started](README.md#getting-started) in the README (system libraries, `dx`
0.7.10, Android SDK and NDK). CI builds with the `desktop` feature on Ubuntu 24.04, so
you can rely on it if you cannot install the webview libraries locally.

### Project layout

| Path | What lives there |
|---|---|
| `crates/core/src/action.rs` | Key actions and the `<name>` notation |
| `crates/core/src/layout.rs` | TOML layout loading and validation |
| `crates/core/src/engine.rs` | The `Keyboard` state machine (layers, modifier latches) |
| `crates/core/src/output.rs` | `Output`, `OutputSink` and the in-app `PreviewSink` |
| `crates/core/layouts/dev.toml` | The built-in developer layout |
| `apps/super-keyboard/src/main.rs` | The Dioxus UI |
| `apps/super-keyboard/assets/main.css` | Styles and theme variables |

## Making a change

1. Fork the repository and create a branch from `main`.
2. Keep changes focused: one issue per pull request.
3. Add or update tests. Engine behaviour belongs in `crates/core` with unit tests there;
   keep platform and UI code thin.
4. Run `cargo fmt`, `cargo clippy` and `cargo test` as above.
5. Update the docs if behaviour changes: README, `docs/`, or the comment at the top of
   `dev.toml` for layout notation.
6. Open a pull request that says what changed, why, and how you tested it (which devices
   or desktops, for platform work). Link the issue with `Closes #123`.

Commit messages: a short imperative summary line (`engine: repeat keys while held`),
then a body explaining why if it is not obvious.

## Guidelines

- **Privacy first.** Never log, store or send typed text, clipboard contents or key
  events outside the running app. No analytics, no network access in the keyboard.
  Reviews will block on this.
- **The engine stays portable.** `crates/core` must not depend on Dioxus or any platform
  API. It must keep building for Android targets (CI checks this).
- **Layouts are data.** Layout features may insert text and press keys; they must never
  execute code.
- **No `unsafe`** in the workspace crates (it is forbidden by lint). If a platform
  integration truly needs it, discuss it in the issue first.
- **Latency matters.** For changes on the key-press path, mention the effect on
  touch-to-output time.

## Adding to the layout notation

To add a named key such as `<printscreen>`: add the variant and its names to `NamedKey` in
`action.rs` (`from_name`, `label`, `Display`), add a test, and document it at the top of
`dev.toml`. Sinks that cannot send the key yet should ignore it rather than panic.

## Reporting bugs and security issues

Use the issue templates for bugs and ideas. Please do not paste anything private you typed
with the keyboard into an issue. Security problems go through private reporting; see
[SECURITY.md](SECURITY.md).

## Code of conduct

This project follows the [Code of Conduct](CODE_OF_CONDUCT.md). By taking part you agree
to uphold it.

## License

By contributing, you agree that your contributions are dual licensed under MIT OR
Apache-2.0, as described in the [README](README.md#license).
