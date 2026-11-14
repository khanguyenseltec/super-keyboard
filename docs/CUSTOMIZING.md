# Customizing Super Keyboard

Super Keyboard is configured with plain files: one `settings.toml` and any number of
layouts. The app's **settings** screen edits `settings.toml` for you; everything else
is a text editor away. On the desktop, the app reloads when you save a file.

Three ideas make the keyboard modular:

- **Sections** are named blocks of keys in a layout: the number row, the bracket row,
  terminal keys, letters, the space-bar row. Layers are built from sections.
- **Profiles** pick a layout and arrange its sections: switch any section on or off and
  move it up or down. One order applies to every layer.
- **Presets** are profiles that ship with the app. Change one and you get your own copy;
  "reset to preset" brings the original back. The profile menu lists presets, your
  profiles and your layouts, in an order you choose, and can hide any of them.

## Where the files live

| Platform | Directory |
|---|---|
| Linux (Ubuntu) | `$XDG_CONFIG_HOME/super-keyboard`, usually `~/.config/super-keyboard` |
| Android | The app's private storage (`/data/user/0/<package>/files`); the settings screen is the way to change it |
| Anywhere | `$SUPER_KEYBOARD_CONFIG_DIR`, if set, wins |

```text
~/.config/super-keyboard/
├── settings.toml        profile, theme, key timing and size, colours, profiles, menu
└── layouts/
    ├── dev.toml         optional: replaces the built-in `dev` layout
    └── vim.toml         another layout, chosen as "vim"
```

Nothing has to exist. The directory and `settings.toml` are created the first time you
change a setting in the app, with every option written out and explained.

## When something is wrong

A broken file never stops the keyboard. The app shows what is wrong and where, under
the top bar:

```text
layouts/vim.toml  layer `base`, row 2, key 4: unknown key `<esc2>`
settings.toml     `keys.repeat-rate` must be between 5 and 60, not 100
settings.toml     profile `mine`: layout `dev` has no section `numbers`
```

While you edit, the last good version of a broken file stays in use, so saving a
half-finished layout does not reset the keyboard. Fix the file and save again.

## settings.toml

Every option is optional. Unknown names and out-of-range values are reported rather
than ignored, so typos do not go unnoticed. The app keeps your comments and formatting
when it saves.

```toml
# Profile to use: a preset, one of your [[profiles]], or a layout file's name.
profile = "coding"

# Colour theme: "system" (follow the desktop or phone), "light" or "dark".
theme = "system"

[keys]
hold-delay = 300      # milliseconds before a long-press fires a key's hold action (150-1000)
repeat-delay = 400    # milliseconds before Backspace, Delete and the arrows repeat (150-1000)
repeat-rate = 30      # repeats per second (5-60)
swipe-step = 12       # pixels of drag per cursor step on the space bar (4-40)
height = "auto"       # "auto" fits keys to the screen, or a number of pixels (32-96)

[preview]
show-log = true       # show recent output beside the preview text

[style]               # colours and sizes for both themes
radius = "8px"

[style.dark]          # only in the dark theme
accent = "#ff9e64"

[style.light]         # only in the light theme
accent = "#c2410c"

[menu]                # the profile menu (see Profiles below)
order = ["terminal", "coding"]
shown = { compact = false }

[[profiles]]          # your profiles, and your changes to presets
name = "mine"
title = "Mine"
layout = "dev"
sections = { number-row = true, brackets = false }
```

### Style variables

`[style]` applies to both themes; `[style.dark]` and `[style.light]` override it for one
theme. Values are CSS colours (`#7aa2f7`, `rgb(…)`, `hsl(…)`, `oklch(…)`, `color-mix(…)`,
colour names) or lengths (`6px`, `0.5rem`, `calc(…)`, `clamp(…)`).

| Variable | Kind | What it changes |
|---|---|---|
| `bg` | colour | app background |
| `panel` | colour | top bar and keyboard background |
| `key` | colour | key background |
| `key-special` | colour | background of Esc, modifier, layer and other named keys |
| `key-pressed` | colour | key background while pressed |
| `text` | colour | main text and key labels |
| `muted` | colour | secondary text and named-key labels |
| `accent` | colour | caret, one-shot modifiers and the active layer |
| `on-accent` | colour | label of a locked modifier, drawn on the accent colour |
| `selection` | colour | selected text in the preview |
| `warning` | colour | notices about layout and settings errors |
| `radius` | length | key corner radius |
| `gap` | length | space between keys |
| `key-font` | length | key label size |

Key height is `keys.height`, not a style variable, because the settings screen controls it.

**Styles can never load anything.** A keyboard must not be able to send what it shows
anywhere, so style values may only use the characters and functions above:
`url(…)`, `image-set(…)`, `@import`, quotes, `;`, braces and backslashes are rejected
with an error that names the value.

Check contrast when you change colours: the built-in themes keep labels at 4.5:1 or
better against their keys in both themes, including one-shot and locked modifiers.

## Profiles and presets

A profile is a layout plus an arrangement of its sections. Choose one from the menu in
the top bar; build and arrange them on the settings screen:

- **Sections tab**: tick a section to show it, untick to hide it, and move it with ▲ ▼.
  Sections marked *always on* (letters, the space-bar row) can move but not be hidden, so
  you can always type and switch layers. The layer names next to each section show where
  it appears.
- **Profiles tab**: rename the active profile, copy it into a new one, choose another,
  delete your own, or reset a changed preset. Move entries with ▲ ▼ to reorder the menu,
  and untick one to hide it from the menu.

Tapping the layer name in the top bar always goes back to the base layer.

### Presets

| Preset | What it shows |
|---|---|
| `coding` (default) | Esc/Tab and brackets row, letters, space-bar row; symbols and navigation layers |
| `coding-numbers` | The same with a number row on top |
| `terminal` | A row with Esc, Tab, `~ \| / -` and arrows instead of the bracket row |
| `compact` | No bracket row, operator row or F-keys: more room on small phones |

Every layout file in `layouts/` that no profile uses also appears in the menu, under its
file name, so a new layout is always one tap away.

### Profiles in settings.toml

```toml
profile = "mine"

[[profiles]]
name = "mine"                       # letters, digits, - and _
title = "Mine"                      # what the menu shows (default: the name)
layout = "dev"                      # a layout's file name, without .toml
order = ["letters", "brackets"]     # these sections trade places, in this order
sections = { number-row = true, brackets = false }   # switch sections on or off
```

A profile named like a preset (`name = "coding"`) replaces that preset; delete the
`[[profiles]]` entry, or use "reset to preset", to get it back.

`order` lists sections in the order you want them. Listed sections take the places those
sections have in the layout, in your order; sections you leave out stay where they are.
The settings screen always writes the whole order. `sections` only needs the sections you
change: anything not listed keeps the layout's default, so a layout can gain sections
without breaking your profiles.

### The profile menu

```toml
[menu]
order = ["terminal", "mine", "coding"]   # same rule as a profile's `order`
shown = { compact = false, coding-numbers = false }   # hidden from the menu
```

The menu starts with the presets, then your profiles, then layouts that have no profile.
A hidden profile still works if `profile` names it; the menu shows it while it is in use.

## Layouts

A layout is a TOML file in `layouts/`. Its file name (without `.toml`) is the name you
pick it by, in the app's layout menu or in `settings.toml`. A file called `dev.toml`
replaces the built-in layout, so the easiest start is to copy
[`crates/core/layouts/dev.toml`](../crates/core/layouts/dev.toml) there and edit it.

```toml
name = "mine"
description = "Shown in the layout list."

[[sections]]
name = "numbers"              # how profiles refer to it: letters, digits, - and _
title = "Number row"          # what the settings screen shows (default: the name)
enabled = false               # off unless a profile turns it on (default: true)
rows = [["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"]]

[[sections]]
name = "letters"
required = true               # always on; profiles can move it but not hide it
rows = [
    [{ tap = "q", hold = "1" }, "w", "e", "r", "t", "y"],
    ["<shift>", "a", "s", "d", "f", "<bksp>"],
]

[[sections]]
name = "bottom"
title = "Space bar and layer keys"
required = true
rows = [["<layer:nav>", { tap = "<space>", width = 3.0, swipe = "cursor" }, "<enter>"]]

[[layers]]                    # the first layer is the one shown at start
name = "base"
sections = ["numbers", "letters", "bottom"]

[[layers]]                    # a layer can list rows directly, as one fixed section
name = "nav"
rows = [["<home>", "<left>", "<up>", "<down>", "<right>", "<end>", "<layer:nav>"]]
```

Sections show in the order they are declared, and a layer lists the sections it uses
in that same order, so that one order can apply to every layer. A section can appear on
several layers: `bottom` above could be listed by `nav` too, and is then defined once.

Mark the rows you can never do without, such as the one with the layer keys, as
`required`: a profile can then never hide the way back to another layer.

### Key notation

| Written as | Meaning |
|---|---|
| `"a"`, `"->"`, `"::"` | Insert that text. One character responds to Shift and forms chords (`Ctrl+C`). |
| `"<esc>"`, `"<f5>"`, `"<up>"` | Press a named key. |
| `"<ctrl>"`, `"<alt>"`, `"<shift>"`, `"<super>"` | Modifier: tap once for the next key, twice to lock, three times to release. |
| `"<layer:sym>"` | Switch to a layer; tap again to go back to the first layer. |

Named keys: `esc`, `tab`, `enter` (`cr`), `bksp` (`bs`), `del`, `ins`, `space`, `up`,
`down`, `left`, `right`, `home`, `end`, `pgup`, `pgdn`, `f1`–`f24`.

Anything that is not `<name>` is text, so `"<="` and `"</"` need no escaping. TOML
literal strings (`'\'`, `'"'`) avoid escaping backslashes and quotes.

### Key options

Write a key as a table to set options:

| Option | Example | Meaning |
|---|---|---|
| `tap` | `{ tap = "<space>" }` | What a tap does, in key notation. |
| `text` | `{ text = "<div>" }` | Literal text, for text that looks like a key name. Use instead of `tap`. |
| `label` | `{ text = "<div>", label = "div" }` | What the key shows. |
| `width` | `{ tap = "<space>", width = 3.0 }` | Relative width; a normal key is `1.0`. |
| `hold` | `{ tap = ".", hold = ".." }` | What a long-press does, in key notation. Shown as a small hint on the key. |
| `swipe` | `{ tap = "<space>", swipe = "cursor" }` | Drag sideways to move the cursor; with Shift on, it selects. |
| `repeat` | `{ tap = "<bksp>", repeat = false }` | Repeat while held. On by default for Backspace, Delete, arrows, Page Up and Page Down. |

A key does one thing while held: it repeats, or it has a `hold`, or it swipes. An
explicit `repeat = true` together with `hold` or `swipe` is an error; the default repeat
of Backspace and the arrows quietly gives way.

Layouts are data: they can insert text and press keys, never run code.
