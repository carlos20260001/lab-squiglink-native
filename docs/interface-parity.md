# Interface parity: the reference and the native

What the web reference puts in front of the user, and where the native
application stands against it.

The reference is the older CrinGraph interface, extended: the ids the native
already knows (`norm-phon`, `norm-fr`, `smooth-level`, `inspector`, `label`,
`download`, `recolor`) are the same ones, and the extra panels are new.

The reference's source is kept whole in [`web/`](../web), on the `pink-eq`
branch of the site repository, and the native is written against it.

## The shell

| Reference | Native |
|---|---|
| Header: wordmark, a review dropdown, the site navigation, a `+Create` link | Header: the mark, the wordmark, the number of curves, the theme picker. A window has no site to navigate. |
| Left column with three tabs: **Brands / Models / Equalizer**, and the column's contents sixteen points in from its edge | The same three tabs ✓, joined side by side as the reference joins them, with the rule under them ✓ — opening on the models, with the brands down a narrow column beside them, and the measurements on the graph as chips at the head of the list ✓. The equalizer's panel scrolls under the tabs, as the reference's does ✓ |
| Graph toolbar: Copy URL, Screenshot, `NORMALIZE: [dB 60] [Hz 500]`, `SMOOTH: 5`, `ZOOM: Bass Mids Treble`, a help button | The same controls, in a row that scrolls sideways when it overflows ✓ — but no help button ✗ |
| Graph: the dB scale and its drag handle, the frequency axis, the band names, the target label, the curve labels, the watermark | The same ✓ — the scale's mark moves the range, the two circles change how far it reaches, and a double-click puts it back ✓ |
| Under the graph: the **DELTA** chips, the rig corrections, `REFERENCE TARGETS` / `LEGACY TARGETS` | The target chips, grouped as the reference groups them, with the rig corrections in a group of their own marked Δ ✓, and the active curves below them ✓ |
| Manager rows: the curve, `L ONLY / R ONLY`, a dB offset, and the row buttons | The same ✓ — the key graphic is the reference's own drawing, and the buttons carry the reference's marks |
| **IEM GRAPH DATABASES** — the other graphs' tools, as links | The same ✓ — the same eleven links, and they open in the browser the user has ✓ |

## The equalizer panel

| Reference | Native |
|---|---|
| `Parametric Equalizer`, a help button, `Reset`, a settings gear | `Parametric Equalizer`, `Reset` and the gear, the arrow and the cog being the reference's own marks ✓ — no help button ✗. The reference's `Reset` sets the bands' values back to flat and leaves the rows and the constraints alone; the native's does the same ✓. The gear opens the constraints, as the reference's does ✓ |
| The model dropdown, the target dropdown, `Auto EQ` | The same ✓ |
| The band table: `Type / Frequency / Gain / Q` | The same ✓ — ten bands, empty ones included, as the reference shows them ✓; the kind of filter written in plain words with a caret ✓, the unit inside the field ✓, the figure in the accent ✓, and the fields short, as the reference's are ✓ |
| `+` `−` `Sort`, `Import` `Export`, `Export Graphic EQ (Wavelet)` | The same ✓ — the import takes a path, as the upload's does ✓. The reference's `−` removes the last band that still says something and never the last row, and `+` stops at twenty bands; the native does both ✓. The Wavelet export sits on a row of its own, in plain words, as the reference writes it ✓ |
| **The constraints panel** — see below | **The same** ✓ |

## The constraints panel

The reference can constrain what AutoEQ is allowed to fit. Read from the
reference's own controls and their help text:

| Control | Meaning | Native |
|---|---|---|
| `eq-constraint-preset-input` | A preset, or none. The reference also ships presets for particular devices; those are its data about those devices and are not carried over. | ✓ |
| `eq-constraint-type-toggle` ×3 | Whether the fit may use `PK`, `LSQ` or `HSQ` filters | ✓ |
| `eq-constraint-max-bands` | How many filters the fit may produce (0–20, 0 = no cap) | ✓ |
| `eq-constraint-freq-min` / `-max` | The frequency band the fit may use (0 = no limit) | ✓ |
| `eq-constraint-freq-graphic-list` | The band frequencies of a graphic equalizer, as a list (`32, 64, 125`) | ✓ |
| `eq-constraint-gain-min` / `-max` | The gain the fit may use (−40–40 dB, 0 = no limit) | ✓ |
| `eq-constraint-q-min` / `-max` | The Q the fit may use (0–10, 0 = no limit) | ✓ |
| `eq-constraint-2ch` | Separate left and right filter banks | ✓ — the shared filters shape every channel, and each side may carry filters of its own |

## The sound tools

| Reference | Native |
|---|---|
| `Compare A / B`, a settings gear | The same ✓ — the switch is the reference's own: a track with `A` and `B` in it, and the chosen one on a filled thumb. With B the source is played through the equalizer, and with A it is played on its own ✓. The gear is the reference's cog, and it opens the card the reference opens: the volume, with the speaker beside it and the share it is at on the right ✓ |
| **Music** — a list of tracks, `+ Add Music`, `Search Apple` | ✓ — the card is the reference's, and the two ways in are plain words, as the reference writes them ✓. The music the reference lists is its own; the native plays a file from this computer and searches the Apple Music catalogue for a preview |
| **Pink Noise** — a play button | ✓ — a pill with the reference's triangle in it, and no lettering, as the reference draws it ✓ |
| **Tone Generator** — a play button, a slider, the frequency, `+ Add Filter` | The same ✓ — the slider is a thin track with a round thumb, on a log scale as the reference's is ✓, and `+ Add Filter` adds a band at the tone's frequency to the equalizer, as the reference's does ✓ |
| **Range** — `Reset`, and the frequency span the slider covers (`20 Hz`–`20000 Hz`) | **The same** ✓ — the word and the arrow that bends back, as the reference writes them ✓ |

## Below the sound tools

| Reference | Native |
|---|---|
| **Upload Data** — the warning that another rig's measurements do not mix, `Upload FR`, `Upload Target` | The same ✓ |
| **Change History** — undo and redo | The same ✓ |
| **The augment card** — the review score, the price, the shop link, on the model row | The same ✓ |
| **A third theme**, a contrast mode beside the light and the dark | The same ✓ |

## What the reference has that the native does not

The targets, in the order they are worth doing.

### 1. The music list

The reference lists the music it plays. The native plays a file from this
computer, or a preview from the Apple Music catalogue, and does not keep a
list of them. **Not worth doing: the tone generator and the pink noise
already cover hearing the equalizer.**

### 2. The reference's guidance

The reference keeps a `?` in its markup beside the normalising control and
hides it in the stylesheet, so that the script can show the text on hover.
The native shows the same text as a tooltip on the control itself ✓, which
is the same guidance without the mark. **Nothing to do.**

## What the native has that the reference does not

- The measurements and the targets in one list, rather than the reference's
  separate list and strip.
- The links to the other graphs' tools open in the browser, where the
  reference's own pages would have taken the reader.
- A music player that reads a file from this computer or a preview from the
  Apple Music catalogue, where the reference lists the music it holds.
- A graph that keeps the reference's proportions, and a tools row that scrolls
  rather than clipping.
- The rig corrections as their own data, with the `data_deltas/` folder
  beside the measurements.
