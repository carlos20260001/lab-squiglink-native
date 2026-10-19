# Interface parity: the reference and the native

What the web reference puts in front of the user, read from the live site,
and where the native application stands against it.

The reference is the older CrinGraph interface, extended: the ids the native
already knows (`norm-phon`, `norm-fr`, `smooth-level`, `inspector`, `label`,
`download`, `recolor`) are the same ones, and the extra panels are new.

## The shell

| Reference | Native |
|---|---|
| Header: wordmark, a review dropdown, the site navigation, a `+Create` link | Header: the mark and the wordmark, the number of curves, the theme picker. A window has no site to navigate. |
| Left column with three tabs: **Brands / Models / Equalizer** | The same three tabs ✓ |
| Graph toolbar: Copy URL, Screenshot, `NORMALIZE: [dB 60] [Hz 500]`, `SMOOTH: 5`, `ZOOM: Bass Mids Treble`, a help button | The same controls, in a row that scrolls sideways when it overflows ✓ |
| Graph: the dB scale and its drag handle, the frequency axis, the band names, the target label, the curve labels, the watermark | The same, except the dB scale cannot yet be dragged ✓ |
| Under the graph: the **DELTA** chips, the rig corrections, `REFERENCE TARGETS` / `LEGACY TARGETS` | The target chips and the active curves ✓ (the rig corrections are read but not yet offered) |
| Manager rows: the curve, `L ONLY / R ONLY`, a dB offset, and the row buttons | The same ✓ |

## The equalizer panel

| Reference | Native |
|---|---|
| `Parametric Equalizer`, a help button, `Reset`, a settings gear | `Parametric Equalizer`, `Reset` ✓ (no gear) |
| The model dropdown, the target dropdown, `Auto EQ` | The same ✓ |
| The band table: `Type / Frequency / Gain / Q` | The same ✓ |
| `+` `−` `Sort`, `Import` `Export`, `Export Graphic EQ (Wavelet)` | `+` `−` `Sort`, `Export`, `Export Graphic EQ` ✓ (no import) |
| **The constraints panel** — see below | **The same** ✓ |
| The band the fit may use, `AutoEQ Range` | The same ✓ |

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
| `Compare A / B`, a settings gear | The same ✓ (no gear) |
| **Music** — a list of tracks, `+ Add Music`, `Search Apple` | Absent (the music is the reference's own) |
| **Pink Noise** — a play button | ✓ (a source) |
| **Tone Generator** — a play button, a slider, the frequency, `+ Add Filter` | ✓ (a frequency and a volume), but not yet played through the equalizer |
| **Range** — `Reset`, and the frequency span the slider covers (`20 Hz`–`20000 Hz`) | **The same** ✓ |

## Below the sound tools

| Reference | Native |
|---|---|
| **Upload Data** — the warning that another rig's measurements do not mix, `Upload FR`, `Upload Target` | The same ✓ |
| **Change History** — undo and redo | The same ✓ |
| **The augment card** — the review score, the price, the shop link, on the model row | The same ✓ |
| **A third theme**, a contrast mode beside the light and the dark | The same ✓ |

## What the reference has that the native does not

1. **The music list** — site-specific, and not portable.
2. **The sources played through the equalizer** — the reference plays pink
   noise, the tone and the music *through* the parametric equalizer, so you
   hear what the bands do, with an A/B switch between the curve and the
   curve as shaped. The native plays the sources on their own.
3. **The rig corrections**, offered on the graph as their own strip.
4. **The dB scale that can be dragged**, and the curve that follows the pointer
   with its reading.

## What the native has that the reference does not

- The measurements and the targets in one list, rather than the reference's
  separate list and strip.
- Pink and white noise, a volume control, and a second sound slot.
- A graph that keeps the reference's proportions, and a tools row that scrolls
  rather than clipping.
