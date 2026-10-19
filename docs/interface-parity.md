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
| Left column with three tabs: **Brands / Models / Equalizer** | The same three tabs ✓ |
| Graph toolbar: Copy URL, Screenshot, `NORMALIZE: [dB 60] [Hz 500]`, `SMOOTH: 5`, `ZOOM: Bass Mids Treble`, a help button | The same controls, in a row that scrolls sideways when it overflows ✓ — but no help button ✗ |
| Graph: the dB scale and its drag handle, the frequency axis, the band names, the target label, the curve labels, the watermark | The same, except the dB scale cannot be dragged ✗ |
| Under the graph: the **DELTA** chips, the rig corrections, `REFERENCE TARGETS` / `LEGACY TARGETS` | The target chips and the active curves ✓ — the Δ chips are not offered ✗ |
| Manager rows: the curve, `L ONLY / R ONLY`, a dB offset, and the row buttons | The same ✓ |

## The equalizer panel

| Reference | Native |
|---|---|
| `Parametric Equalizer`, a help button, `Reset`, a settings gear | `Parametric Equalizer`, `Reset` ✓ — no help button ✗, no gear ✗. The reference's `Reset` sets the bands' values back to flat and leaves the rows and the constraints alone; the native's does the same ✓ |
| The model dropdown, the target dropdown, `Auto EQ` | The same ✓ |
| The band table: `Type / Frequency / Gain / Q` | The same ✓ |
| `+` `−` `Sort`, `Import` `Export`, `Export Graphic EQ (Wavelet)` | `+` `−` `Sort`, `Export`, `Export Graphic EQ` ✓ — no import ✗. The reference's `−` removes the last band that still says something and never the last row, and `+` stops at twenty bands; the native does both ✓ |
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
| `Compare A / B`, a settings gear | **Absent** ✗ — the sources are played on their own, not through the equalizer |
| **Music** — a list of tracks, `+ Add Music`, `Search Apple` | Absent (the music is the reference's own) |
| **Pink Noise** — a play button | ✓ (a source) |
| **Tone Generator** — a play button, a slider, the frequency, `+ Add Filter` | ✓ (a frequency and a volume) |
| **Range** — `Reset`, and the frequency span the slider covers (`20 Hz`–`20000 Hz`) | **The same** ✓ |

## Below the sound tools

| Reference | Native |
|---|---|
| **Upload Data** — the warning that another rig's measurements do not mix, `Upload FR`, `Upload Target` | The same ✓ |
| **Change History** — undo and redo | The same ✓ |
| **The augment card** — the review score, the price, the shop link, on the model row | The same ✓ |
| **A third theme**, a contrast mode beside the light and the dark | The same ✓ |

## What the reference has that the native does not

The targets, in the order they are worth doing.

### 1. The sources played through the equalizer, with a Compare switch

The reference's `Compare` toggle is an A/B: with it on (B) the live playback
runs the parametric equalizer, and with it off (A) it runs the reference path.
So the pink noise and the tone are heard *through* the bands, and the switch
says whether they are.

The native plays the sources on their own. Hearing the bands work needs a
live biquad chain: the core has the coefficients and the filter maths, but
the generator produces a mono signal and the banks are per channel, so the
audio path has to become stereo and run the filters sample by sample.
**The core work; the interface work is one toggle and a switch in the sound
tools.**

### 2. The inspector: the line under the pointer, and the readings

The reference draws a vertical line at the pointer's frequency and the value
of each curve there, and it can follow the pointer as it moves. The native
has the `Inspect` toggle and the core's nearest-point search, but the graph
does not draw the line or the readings. **Small: the geometry is in the core
and the drawing is a line and some text.**

### 3. The dB scale that can be dragged

The reference's scale is dragged: the handle moves the range and the circle
scales it, and a double-click resets it. The native's scale is set from the
curves and cannot be dragged. **Small to medium: the transform is the core's,
so this is a drag on the graph and a change of its y range.**

### 4. The Δ chips: the rig corrections

The reference's target strip has a group marked `Δ` — the rig corrections,
the compensation curves for the rig the measurements were taken on. The
native reads them (the core loads `data_deltas/` and offers them as delta
targets) but does not put them in the strip, so they cannot be turned on
alongside the other targets. **Small: the data is already loaded; the strip
needs a group for it.**

### 5. The help buttons and the import

The reference's equalizer has a `?` that opens its guidance, and it can
import an Equalizer APO file. The native has neither in the interface,
though the core can import one. **Small: an import is a file read and the
core's `import_parametric`; the help is a window.**

### 6. The music list

The reference plays music files and searches the Apple Music catalogue. The
catalogue is the reference's own and does not travel; a local file list
would. **Not worth doing: the tone generator and the pink noise already
cover hearing the equalizer.**

### 7. The settings gears

The reference hides the sound tools' settings and the equalizer's behind a
gear. The native shows the volume and the reset in the open. **Not worth
doing: a window has the room, and a gear hides the controls it holds.**

## What the native has that the reference does not

- The measurements and the targets in one list, rather than the reference's
  separate list and strip.
- Pink and white noise, a volume control, and a second sound slot.
- A graph that keeps the reference's proportions, and a tools row that scrolls
  rather than clipping.
- The rig corrections as their own data, with the `data_deltas/` folder
  beside the measurements.
