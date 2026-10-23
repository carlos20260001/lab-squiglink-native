# Feature parity: the reference and the native

What a person can do in the web reference, and what they can do in the
native, step by step. The reference is the older CrinGraph interface,
extended, and its source is kept whole in [`web/`](../web) on the `pink-eq`
branch of the site repository. The native is written against it.

The verdicts are made from what the two do, not from what their code looks
like: each step was carried out in both — the reference in a browser, driven
by `npx --yes @playwright/cli@0.1.22`, the native in a window under Xvfb,
driven by `xdotool` — and the two were looked at side by side.

## 1. Arriving

| Step | The reference | The native | |
|---|---|---|---|
| Open | A page with a dark header, the wordmark, a review dropdown and the site's navigation | A window with the mark, the wordmark and the number of curves on the graph. A window has no site to navigate | ✓ |
| The theme | One button that turns the page dark, contrast or default, and keeps the choice | The same button, with the same three themes, and the choice is kept | ✓ |
| The tools row | Copy URL, Screenshot, `NORMALIZE: [dB 60] [Hz 500]`, `SMOOTH: 5`, `ZOOM: Bass Mids Treble`, Inspect, Label, Recolor | The same controls, in the same order, with the same words. The unit stands before its figure, as a button of its own; the zoom has three regions and no `Full`; the toggles are buttons with a mark before the word | ✓ — the native's `Full` and its unit inside the figure are gone, and the toggles carry the reference's own marks |
| Normalizing and smoothing | The curves change as the figures are dragged | The same, and the graph is redrawn as the figures are dragged | ✓ — the native used to leave the graph stale; it does not now |
| Searching | A search box over the brands and the measurements | The same box, over the same things | ✓ |
| The measurement's own page | A review score, a price and a shop link | The same card | ✓ |

## 2. Choosing what to look at

| Step | The reference | The native | |
|---|---|---|---|
| The brands tab | The list of brands, with the list of models sliding in from the right | The same, and the names alone | ✓ |
| Pressing a brand | The models tab opens with that brand's measurements | The same | ✓ |
| The models tab | The list of models slides over the list of brands, which are behind it in a quiet grey. The rows are forty points tall, with six between them | The same, and the brands behind are now veiled with the page's own colour, as the reference veils them | ✓ — the rows were thirty-six tall, and the brands behind were not veiled |
| Pressing a model | The measurement goes on the graph, and its row appears in the manager | The same | ✓ |
| The manager row | The name, the key, the offset field, and five marks. The key is a fan: the whole of it at the wide end, faded at the point, with the channel letters at eight points in the monospaced face | The same, with the reference's own marks, and the key's two halves the right way round | ✓ — the native's key had the faded and the whole parts the wrong way round |
| The target strip | One line of chips, in the reference's groups. The groups are headed in capitals, in the monospaced face, in the faint grey | The same, and the rig corrections in a group of their own marked Δ | ✓ — the native's group headings were in sentence case, in the ordinary face |
| Pressing a target | The target goes on the graph | The same | ✓ |

## 3. Looking

| Step | The reference | The native | |
|---|---|---|---|
| The dB scale | Dragged: the mark moves the range, the two circles change how far it reaches, a double click puts it back. It is a shape of its own — a thin spine with a bracket in the middle and three grips within it — with the figures standing beside it and the unit `dB` written on its side at the top | The same, and the shape is now the reference's own path, with the figures plain numbers (`85`, not `+85`) in the monospaced face | ✓ — the native's scale used to be a plain line with two circles, and drew the figures with a sign; it does not now |
| The graph's proportions | The plot is 800 by 360, and the width is two and a bit units for every unit of the window's height. Its coordinates use D3's logarithmic frequency scale, linear dB scale, and natural/cardinal curves | The same D3 v5.16.0 scale and curve operations are now used by the core through the checked local port | ✓ |
| The band names | Sub bass, Mid bass, … Air, under the axis | The same | ✓ |
| The curve labels | A swatch and a name, stacked so they do not overlap | The same | ✓ |
| The watermark and the rig line | The wordmark in the middle, and the line that says what the rig was | The same | ✓ |
| The inspector | A line at the pointer's frequency, and the reading of each curve | The same | ✓ |

## 4. Shaping

| Step | The reference | The native | |
|---|---|---|---|
| The equalizer's head | The title, `Reset` with its arrow, and the cog | The same | ✓ |
| The band table | Ten bands, empty ones included; the kind of filter in plain words with a caret; the unit inside the field; the figure in the accent | The same | ✓ |
| Editing a band | Type into the table | The same | ✓ |
| `＋` `－` `Sort` | Add a band, take the last one that still says something away, order them by frequency | The same, and the cap of twenty bands is the same | ✓ |
| The constraints | Behind the cog: a preset, the kinds of filter, the number of bands, the frequency span, the gain span, the Q span, the graphic bands, and the separate left and right banks | The same, behind the same cog | ✓ |
| Auto EQ | Fits bands to the target, within the constraints | The same, and the fit is checked against the reference's own | ✓ |
| **The graph as an editor** | A band is drawn out of the curve by dragging on it; a band is moved by dragging its mark; the wheel over a mark changes the band's width; the right button on a mark turns its kind round | **The same** ✓ — the axis locks to the way the pointer moved, as the reference's does, and the width of a band changes by the reference's own step | ✓ |
| **The range brush** | Away from the curve, a drag brushes a range of frequencies for the tone generator | **The same** ✓ | ✓ |
| Undo and redo | The state before a change is kept, so undo undoes it, and `Ctrl+Z` and `Shift+Ctrl+Z` (or `Ctrl+Y`) do it from the keyboard | The same — the native used to keep the state after, so undo did nothing. The shortcuts are wired and left alone while a field is being written in, as the reference leaves them; they cannot be exercised under Xvfb: the window can be given the focus, and the keys arrive, but the server does not carry the modifier | ✓ |
| Import and export | An Equalizer APO file in, an APO file and a Wavelet file out | The same, and the file is chosen with the system's own dialog where there is one | ✓ |

## 5. Hearing, and keeping

| Step | The reference | The native | |
|---|---|---|---|
| Pink noise | A play button | The same, a pill with the reference's triangle in it | ✓ |
| The tone generator | A play button, a slider on a log scale, the frequency, and `+ Add Filter`, which adds a band at that frequency to the equalizer | The same | ✓ |
| Compare A and B | A switch: with B the source is played through the equalizer, with A it is played on its own | The same, and the switch is the reference's own | ✓ |
| The volume | Behind a cog, in a card with a speaker | The same | ✓ |
| Music | A file from the computer, or a search of the Apple Music catalogue | The same, and the catalogue is searched in the background | ✓ |
| Upload | A file chosen with the system's dialog | The same, and the path can still be typed where there is no dialog | ✓ |
| Choosing a file | The browser's own file input | The desktop's own dialog, the browser's file input, the phone's document picker, and on the other phone the system's own file table. Where there is a filesystem the file is named by its path, and where there is not the bytes come back instead | ✓ |
| The row's marks | Hide, pin, the baseline, the download, the cross | The same, and each is the reference's own vector | ✓ |
| The other graphs' tools | A row of links to other people's tools | The same eleven links, opening in the browser | ✓ |
| Closing and opening again | The view is in the link | The view is in the preferences file, and the link carries it too | ✓ |

## The file picker, and where it comes from

| Where | What asks for the file |
|---|---|
| Windows, macOS, Linux | The system's own dialog, through `rfd` — the one every other program on those systems uses |
| The browser | The file input the web has always had, through `rfd`'s browser backend |
| The phone | The system's own document picker, written against the system's own libraries (`objc2`), because there is no third party between the program and the phone |
| The other phone | The system's own file table, read through the JNI, because the system picker hands its answer back through the activity, which the activity the game runs in does not pass on |

## What the native has that the reference does not

- The measurements and the targets in one list, rather than the reference's
  separate list and strip.
- A music player that reads a file from this computer or a preview from the
  Apple Music catalogue, where the reference lists the music it holds.
- The file paths can be typed as well as chosen, which is what a window
  without a dialog needs.
- The graph's editor works with the mouse alone, which the reference's does
  too, but the native's also keeps the table and the graph in step, so a
  band drawn on the graph can be read in the table.

## What the reference has that the native does not

- **The tutorial overlay** — the seven band names with a description beside
  each. The reference's descriptions are placeholders ("Lorem ipsum"), so
  there is nothing to carry.
- **The sound tools' own gear** opens a card with the volume in it; the
  native's does the same ✓.
