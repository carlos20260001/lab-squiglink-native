# Interface parity: the reference and the native

What the web reference puts in front of the user, read from the live site with
REA (`analyze-web-bundle` over `https://squig.link/`, plus the rendered DOM),
and where the native application stands against it.

The reference is the older CrinGraph interface, extended: the ids the native
already knows (`norm-phon`, `norm-fr`, `smooth-level`, `inspector`, `label`,
`download`, `recolor`) are the same ones, and the extra panels are new.

## The shell

| Reference | Native |
|---|---|
| Header: wordmark, a review dropdown, the site navigation, a `+Create` link | Header: wordmark, the `IEMS / HEADPHONES / EARBUDS` categories, a hamburger (mobile), a library button (mobile) |
| Left column with three tabs: **Brands / Models / Equalizer** | The same three tabs ✓ |
| Graph toolbar: Copy URL, Screenshot, `NORMALIZE: [dB 60] [Hz 500]`, `SMOOTH: 5`, `ZOOM: Bass Mids Treble`, a help button | The same controls, in a cluster of its own, and the toolbar scrolls sideways when it overflows |
| Graph: the dB scale and its drag handle, the frequency axis, the band names, the target label, the curve labels, the watermark | The same ✓ (the watermark is the name only) |
| Under the graph: the **DELTA** chips, the rig corrections, `REFERENCE TARGETS` / `LEGACY TARGETS` | The same ✓ |
| Manager rows: the curve, `L ONLY / R ONLY`, a dB offset, and the row buttons | The same ✓ |

## The equalizer panel

| Reference | Native |
|---|---|
| `Parametric Equalizer`, a help button, `Reset`, a settings gear | `Parametric Equalizer`, a help button, `Reset` ✓ (no gear) |
| The model dropdown, the target dropdown, `Auto EQ` | The same ✓ |
| The band table: `Type / Frequency / Gain / Q` | The same ✓ |
| `+` `−` `Sort`, `Import` `Export`, `Export Graphic EQ (Wavelet)` | The same ✓ |
| **The constraints panel** — see below | **Absent** |

## The constraints panel (the reference's, not in the native)

The reference can constrain what AutoEQ is allowed to fit. Read from the
reference's own controls and their help text:

| Control | Meaning |
|---|---|
| `eq-constraint-preset-input` | A preset: `default`, `auto-eq`, `generic-10-band`, `fiio-eh11`, `sennheiser-hdb-630`, `sony-wh-1000xm6`, or none |
| `eq-constraint-type-toggle` ×3 | Whether the fit may use `PK`, `LSQ` or `HSQ` filters |
| `eq-constraint-max-bands` | How many filters the fit may produce (0–20, 0 = no cap) |
| `eq-constraint-freq-min` / `-max` | The frequency band the fit may use (0 = no limit) |
| `eq-constraint-freq-graphic-list` | The band frequencies of a graphic equalizer, as a list (`32, 64, 125`) |
| `eq-constraint-gain-min` / `-max` | The gain the fit may use (−40–40 dB, 0 = no limit) |
| `eq-constraint-q-min` / `-max` | The Q the fit may use (0–10, 0 = no limit) |
| `eq-constraint-2ch` | Separate left and right filter banks |

The native has none of this: its AutoEQ fits `PK` filters only, inside a fixed
20–15000 Hz band, with the Q and gain bounds hard-wired to
`OPTIMIZE_Q_RANGE` and `OPTIMIZE_GAIN_RANGE`.

## The sound tools

| Reference | Native |
|---|---|
| `Compare A / B`, a settings gear | The same ✓ (no gear) |
| **Music** — a list of tracks, `+ Add Music`, `Search Apple` | Absent (the native has no music) |
| **Pink Noise** — a play button | ✓ (a source) |
| **Tone Generator** — a play button, a slider, the frequency, `+ Add Filter` | ✓ (a frequency and a volume) |
| `Range` — `Reset`, and the frequency span the slider covers (`20 Hz`–`20000 Hz`) | **Absent**: the native's slider always covers 20 Hz–20 kHz |

## Below the sound tools

| Reference | Native |
|---|---|
| **Upload Data** — the warning that another rig's measurements do not mix, `Upload FR`, `Upload Target` | The same ✓ |
| **Change History** — undo and redo | Absent |
| **The augment card** — the review score, the price, the shop link, on the model row | Absent (the native ignores the fields in the phone book) |
| **A third theme**, a contrast mode beside the light and the dark | The light and the dark only |

## What the reference has that the native does not

1. **The constraints panel** — the largest gap, and the one that changes what
   AutoEQ can produce rather than how it looks.
2. **The tone generator's band** — the range the slider sweeps.
3. **The augment card** — the review score, the price, the shop link.
4. **The change history** — undo and redo.
5. **A contrast theme.**
6. **The music list** — site-specific, and not portable.

## What the native has that the reference does not

- The measurements and the targets in one list, rather than the reference's
  separate list and strip.
- Pink and white noise, a volume control, and a second sound slot.
- A graph that keeps the reference's proportions, and a toolbar that scrolls
  rather than clipping.
