# Working in this repository

A guide for an AI agent (or a person) picking up SquigLab: what the project
is, the rules it works under, the method used to bring the native to parity
with the web reference, the exact commands, and the traps that cost hours.

Read this before changing anything.

---

## 1. What this repository is

Two things live here:

| | Where | What it is |
|---|---|---|
| **The native program** | `crates/squiglab-core/` + `crates/squiglab/` | The product. A Rust/egui desktop application for comparing frequency-response measurements against targets. |
| **The web reference** | `web/` | The JavaScript/HTML implementation this native was built to match, kept whole. It is the **authority**: for the maths, for the behaviour, and for the appearance. |

The native was written by porting the reference's code. The UI was
written against the reference's *observable* behaviour and geometry, and the
result was compared against the reference, side by side, until it matched.

### The two rules that govern everything

1. **`crates/squiglab-core` has no egui in it.** The rules, the state and
   the data live there and know nothing about how anything is drawn. The
   interface in `crates/squiglab/` is written against the core, never the
   other way about. Check with:
   ```sh
   grep -rn "egui\|eframe" crates/squiglab-core/src/   # must print nothing
   ```
2. **The reference decides.** When the native and the reference disagree,
   the reference is right until you have measured it and can say why not.
   Never "improve" the reference's arrangement in the native without
   measuring the reference first.

### Where the reference comes from

`web/` is the `pink-eq` branch of the site repository
(`github.com/squiglink/lab`), kept whole so that it can be read, diffed and
served. Two things to know about it:

- **The live site is ahead of the branch.** squig.link now serves a newer
  theme ("Super") than the branch's own ("Primary", the `theme-default`
  class). The native follows the **live site's** colours for the theme and
  the **branch** for the behaviour and the layout. When the two disagree,
  that is why.
- **To take a newer reference**, fetch that branch into `web/` and read the
  diff before changing anything in the native. The reference is the
  authority, so a change in the reference is a change in the native.

---

## 2. The coding practices the owner holds to

These are not stylistic preferences; the owner has pushed back on each of
them.

- **Use established crates, or the platform's own libraries.** A crate with
  no community, no review history and open stability issues does not go in,
  however convenient it is. `robius-file-picker` (139 downloads, an open
  `ndk_context`/`android-activity` conflict) was removed for exactly this
  reason. `rfd` is the one for dialogs.
- **Where a third-party crate is thin over the platform, write the platform
  code instead.** The phone's document picker is written against `objc2`
  (the crates the window already uses) rather than taken from a crate that
  wraps them, and the other phone's file list is read through the JNI
  against the system's own API.
- **Do not add a dependency for a thing the platform already does.**
- **A file is chosen with the system's own dialog.** There are no typed
  paths anywhere: the upload, the equalizer's import and the music all take
  their file from the picker, and show the name of the file that was chosen
  beside the button.
- **One concern per module.** The core is egui-free; the interface owns the
  drawing.
- **Comments explain why, and are written as prose.** The code in this
  repository is read by people who did not write it.
- **Device-specific data does not live in the source.** No preset named
  after a particular rig, no calibration curve in the repository.

---

## 3. The parity method

This is the method that has been used throughout, and the one to use.

### 3.1 Bring both up side by side

**The reference** — serve the repository root and drive it in a browser:

```sh
python3 -m http.server 8765 --bind 127.0.0.1
# → http://127.0.0.1:8765/web/index.html
```

Drive it with the Playwright CLI (**not** the deprecated `playwright-cli`
package):

```sh
PW="npx --yes @playwright/cli@0.1.22 -s=parity"
$PW open  "http://127.0.0.1:8765/web/index.html"
$PW resize 1280 900            # or 1440 900; keep it equal to the native's window
$PW eval   "() => { ... }"     # run JavaScript in the page
$PW click  "e146"              # a snapshot ref, or a CSS selector
$PW screenshot --filename /tmp/opencode/ref.png
```

**The native** — run it under Xvfb and drive it with `xdotool`:

```sh
pgrep -x Xvfb >/dev/null || Xvfb :99 -screen 0 1440x900x24 &
(env -u WAYLAND_DISPLAY DISPLAY=:99 SQUIGLAB_CONFIG_DIR=/tmp/opencode/sq-light \
     setsid nohup ./target/release/squiglab > /tmp/opencode/nat.log 2>&1 &)
DISPLAY=:99 xdotool mousemove 270 66 click 1     # the Equalizer tab
DISPLAY=:99 ffmpeg -y -f x11grab -video_size 1440x900 -i :99 \
     -frames:v 1 /tmp/opencode/nat.png
```

`SQUIGLAB_CONFIG_DIR` points at a scratch preferences file, so the native
opens in a known state (and the user's own settings are never touched).

### 3.2 Measure the reference; never guess

Before changing the native, **read what the reference actually is**:

```sh
# the geometry of a thing
$PW eval "() => { const r = document.querySelector('input[name=freq]')
   .getBoundingClientRect(); return JSON.stringify({x:r.x,w:r.width,h:r.height}); }"

# its colours and type
$PW eval "() => { const cs = getComputedStyle(document.querySelector('h5'));
   return JSON.stringify({size:cs.fontSize, weight:cs.fontWeight, color:cs.color}); }"

# the CSS variables
$PW eval "() => { const b = getComputedStyle(document.body);
   return b.getPropertyValue('--accent-color'); }"
```

and read the reference's own sources, which are in the repository:

- `web/style-alt.css` — every colour, size, border and layout rule, and
  every `--icon-*` mask (the reference's own vectors)
- `web/equalizer.js` — the filter maths
- `web/graphtool.js` — the graph, the equalizer's editor, the sound tools
- `web/config.js` — the data, the tutorial, the links

To find how the reference *behaves*, grep its JavaScript for the class
or the function that draws the thing, and read the numbers there: the
equalizer's editor is in `graphtool.js` under `eqGraphPointerDown`,
`eqGraphDragMove` and `eqGraphPlotWheel` (the axis lock, the hit radius,
the width's step), and the sound tools are under `liveSound*`. The
reference's own numbers are the ones to copy.

### 3.3 Compare, and write down what differs

Put the two captures side by side and read them:

```python
from PIL import Image
a = Image.open("/tmp/opencode/ref.png").convert("RGB").crop((0, 48, 350, 900))
b = Image.open("/tmp/opencode/nat.png").convert("RGB").crop((0, 48, 350, 900))
out = Image.new("RGB", (720, 852), (255, 255, 255))
out.paste(a, (0, 0)); out.paste(b, (370, 0))
out.save("/tmp/opencode/compare.png")
```

Measure the native by scanning its pixels (there is no inspector for egui):

```python
im = Image.open("/tmp/opencode/nat.png").convert("RGB")
print(im.getpixel((x, y)))          # one point
# a row, to find the edges of a field:
prev = None
for x in range(0, 350):
    p = im.getpixel((x, y))
    if p != prev: print(x, p); prev = p
```

### 3.4 The journey

Parity is not one screen; it is what a person can do. The journey is walked
as **one persona**, whose business is: *pick a measurement, hold it against
a target, shape the curve by hand, hear the difference, keep the result.*

Each step is carried out **in both** — the same action, in the same state —
and judged from what the two do, never from how their code looks.

| Act | The steps |
|---|---|
| **Arriving** | open, the three themes, the tools row, the search |
| **Choosing** | the brands tab, pressing a brand, the models tab, pressing a model, the manager row, the target strip, pressing a target |
| **Looking** | the dB scale and its drag, the band names, the curve labels, the watermark, the rig line, the inspector |
| **Shaping** | the band table, `＋ － Sort`, the constraints, the two-channel banks, Auto EQ, undo and redo, import and export, the graph as an editor |
| **Hearing and keeping** | pink noise, the tone generator, A and B, the volume, music, upload, the row's marks, the links, closing and opening again, Copy URL |

### 3.5 The marks

Every mark the program draws is the reference's own vector, taken from its
stylesheet's `--icon-*` masks or from its markup. `scripts/marks_diff.mjs`
compares the two and says which have drifted:

```sh
node scripts/marks_diff.mjs
```

```
Download       matches --icon-download
Eye            matches --icon-hide
…
Remove         drawn by hand
               the reference: M95.36,24.64h0a5,5,0,0,0-7.08,0L60,52.93,…
```

A mark that reads *drawn by hand* is one that cannot be drawn from a path
here — the two crosses are filled shapes that egui cannot fill, so they are
drawn as the bars they are made of, and the reference's own path is printed
so the geometry can be followed.

### 3.6 Diagnosing the native

There is no inspector for egui, so the native is diagnosed by printing what
it is doing, and by measuring its pixels. A temporary print is the quickest
way to find out why a row is the height it is:

```rust
eprintln!("avail {} rect {:?}", ui.available_width(), ui.max_rect());
```

Run it, read the numbers, and **take the print out again** before
committing — a program that talks to the console every frame is not a
program to ship.

### 3.7 Record it

Every journey step has a verdict in **[`docs/feature-parity.md`](docs/feature-parity.md)**,
which is laid out by the five acts above, one row per step, and says what
the reference does, what the native does, and whether they agree. The
detail of the interface's own arrangement is in
[`docs/interface-parity.md`](docs/interface-parity.md), and the design
language is recorded in [`docs/design-system.md`](docs/design-system.md)
and [`DESIGN.md`](DESIGN.md).

Update them when a step changes, and say plainly what is still different.

### 3.8 The equalizer's fixture

The filter maths is pinned against the reference by
`crates/squiglab-core/src/equalizer_golden.json`, which is **captured from
the reference** by `node scripts/equalizer_golden.mjs`. It is never
authored by hand: if the numbers look wrong, the reference is wrong first.

---

## 4. The commands

The window is 1440 by 900, and the reference's viewport is to be set to the
same size before the two are compared.

```sh
cargo build --offline --release        # the program
cargo test --offline --workspace       # 74 core + 9 interface tests
cargo clippy --offline --workspace --all-targets
cargo fmt --all
./build.sh                             # build and package into dist/
node scripts/equalizer_golden.mjs      # regenerate the equalizer fixture
node scripts/marks_diff.mjs            # compare the marks with the reference's
```

Commit one thing at a time, in the style of the existing history (a prose
subject line, the body saying what changed and why), and push when the
work is sound. The branch tracks `origin/main` and is usually ahead of it.

The measurements live in `data/` (in-ear monitors), `data_hp/` (headphones)
and `data_deltas/` (the rig corrections), and the program's own settings
live in a `config.toml` — see `config-examples/<platform>/` for the shape
of one.

**The data is the user's.** `data/real/` and `data_deltas/` are ignored by
git, because a measurement taken on a person's rig, and a calibration
measured on a clone rig, belong to whoever measured them. A stray
`git add -A` must never publish them; check `git status` before
committing.

---

## 5. The shape of the program

```
crates/squiglab-core/            the rules, the state, the data (no egui)
  session.rs       Session: what is loaded, what is selected, what changed
  equalizer.rs     Filter, Biquad, the filter maths (ported from web/equalizer.js)
  constraints.rs   what a fit may produce
  processing.rs    averaging, normalising, smoothing
  graph.rs         Transform, the band names, label placement
  audio/           the generator, the pink noise, the decoders, the devices
  data/            the phone book and the measurement files
  config.rs        the config file, the preferences, the share links
  theme.rs         Theme: Light, Dark, Contrast (the core's own)

crates/squiglab/                 the interface (egui)
  app.rs           App: the state, the frame, the column, the file picker
  header.rs        the mark, the wordmark, the curve count
  selector.rs      the tabs, the brands, the models
  equalizer.rs     the equalizer's panel: the table, the constraints, history
  graph.rs         the graph, the scale, the equalizer's editor
  curves.rs        the target strip, the manager rows, the database links
  sound.rs         the sound tools: compare, volume, pink noise, the tone
  music.rs         the music player and the Apple Music search
  upload.rs        the upload panel
  tools.rs         the tools row
  icons.rs         every mark, from the reference's own vectors
  filepick.rs      the file picker, per platform
  theme.rs         the palette and the type scale
```

### What the native has that the reference does not

These are the native's own, and they are not to be "corrected" towards the
reference — the reference has no counterpart for them:

- **The file picker** (the reference types paths; the native asks the
  system).
- **The music player** (the reference lists the music it holds; the native
  plays a file from this computer or a preview from the Apple Music
  catalogue).
- **The two-channel filter banks** (the reference's constraints carry the
  setting; the native has the banks themselves).
- **The rig corrections as their own data** (`data_deltas/`, loaded as
  first-class targets and marked Δ).
- **The row of links to the other graphs' tools**, which open in the user's
  browser.

---

## 6. The design language

The interface's appearance is the reference's, and the reasoning behind the
parts of it that are the native's own is written down:

- [`docs/design-system.md`](docs/design-system.md) — the reference's
  extracted tokens: colours, type, spacing, the themes.
- [`DESIGN.md`](DESIGN.md) and `.impeccable/design.json` — the description
  of the native's own arrangement, kept as a pair so the sidecar matches
  the prose.

When the design of a panel is in question, work from the reference's own
measurements (§3.2) rather than from taste, and record what was decided
and why.

---

## 7. Traps, learned the hard way

**egui's `interact_size` decides the height of a row, and it is read from
the *style*, not from the Ui's spacing.** A row that should be twenty-six
points tall comes out thirty-six, and no amount of `ui.spacing_mut()` inside
the row will change it: a child takes its measures from the style, and the
style is captured when the parent window is made. Change it with
`ctx.all_styles_mut(...)` **before** the window is made, and restore it
after. `crates/squiglab/src/app.rs` does this for the left column.

**A row that is wider than its column makes everything after it wider.**
The scroll area's content grows to its widest item, so a table that is a few
points too wide pushes the cards, the buttons and the file fields off the
side. Work the column widths out from `ui.available_width()` rather than
hard-coding them (see `field_width()` in `crates/squiglab/src/equalizer.rs`).

**The font has no fullwidth signs.** `＋` and `－` render as `?`. Use `+`
and `−`.

**Under Xvfb the keyboard needs the focus given to it by hand.** There is
no window manager, so nothing has the focus and no key reaches the
program — until the window is given it:

```sh
W=$(DISPLAY=:99 xdotool search --class squiglab | head -1)
DISPLAY=:99 xdotool windowfocus $W     # now the keys arrive
DISPLAY=:99 xdotool type --delay 150 "1234"    # and they land in a field
```

**But the modifier keys do not survive.** `Ctrl+Z` reaches the program as
a plain `Z`: the server's modifier state is not carried by the synthetic
key, so the equalizer's undo shortcut cannot be exercised here. Test the
mouse paths, and say so when a path cannot be checked.

**The `⊗` in the reference is a character, not an icon.** The reference
writes `⊗` as text in the list rows and uses the filled `--icon-remove` mask
in the manager row. Match whichever the reference uses in that place.

**A mark in the reference may be a CSS mask, not markup.** Every
`--icon-*` in `web/style-alt.css` is an SVG path; the ones in the reference's
markup are `<svg><use>` references. `crates/squiglab/src/icons.rs` holds
the paths, flattened by a small SVG path reader in the same file.

**egui cannot fill a concave shape.** The reference's two crosses (the ⊗ and
the +) are filled outlines; they are drawn here as the two round-capped bars
they are made of, with the reference's own geometry.

**The reference's panel headings are 13pt semibold in `#a6a6a6`**, not
near-black bold. They are `TypeStep::Heading` in the native.

---

## 8. Where the work stands

- The shell, the tabs, the lists, the graph, the manager rows, the equalizer's
  panel, the sound tools, the file picker and the marks are at parity.
- The graph is an equalizer editor: a band is drawn out of the curve by
  dragging on it, a band is moved by its mark, the wheel over a mark changes
  its width, the right button turns a band's kind round, and away from the
  curve a drag brushes a range for the tone generator.
- The rows of the band table sit a little further apart than the
  reference's. This is the one known difference in the equalizer's panel.
- The phone's pickers (iOS and Android) are written but cannot be built or
  run on a machine without Xcode or the NDK. The desktop and the browser
  are verified here.

`docs/feature-parity.md` is the ledger; keep it honest.

---

## 9. Before you commit

- `cargo fmt --all`, `cargo clippy --offline --workspace --all-targets` and
  `cargo test --offline --workspace` all pass.
- No temporary print is left in the code.
- The ledger and the notes say what changed and what is still different.
- `git status` shows no data of the user's.

