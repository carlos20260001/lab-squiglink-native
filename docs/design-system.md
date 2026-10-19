# The SquigLab design system

The interface matches the web reference at squig.link. This file is the
extraction of it: the values below were read from the reference's own
stylesheet and rendered DOM, not chosen.

The reference is a page; this is a window. The vocabulary, the metrics and the
themes are the reference's. The arrangement is the reference's too, adapted
only where a window cannot scroll like a page.

## The three themes

The reference keeps its whole theme system in CSS variables — thirty of them,
in three sets. These are the reference's values, with one change: **the dark
theme's accent is the native's blue, not the reference's yellow**, because the
curves are the native's own coloured palette and a yellow accent fights them.

The values here are the ones the live site serves, which are the "Super"
theme. The repository's own stylesheet still carries the older "Primary"
theme, whose page is a neutral grey rather than a warm off-white; the live
site is the later of the two, and the window follows it.

| Token | Light | Dark | High contrast |
|---|---|---|---|
| accent | `hsl(210, 53%, 49%)` | `#3870A6` (native) | `hsl(0, 0%, 0%)` |
| accent, on | `hsl(210, 50%, 40%)` | `hsl(54, 100%, 40%)` | `hsl(0, 0%, 0%)` |
| background | `hsl(54, 5%, 96%)` | `hsl(215, 14%, 10%)` | `hsl(0, 0%, 100%)` |
| background, contrast | `hsl(54, 5%, 90%)` | `hsl(215, 14%, 10%)` | `hsl(0, 0%, 0%)` |
| background, contrast more | `hsl(54, 0%, 65%)` | `hsl(215, 14%, 25%)` | `hsl(0, 0%, 0%)` |
| background, graph | `hsl(54, 0%, 100%)` | `hsl(215, 14%, 7%)` | `hsl(0, 0%, 100%)` |
| background, inputs | `hsl(210, 10%, 98%)` | `hsl(215, 14%, 5%)` | `hsl(0, 0%, 100%)` |
| font, primary | `hsl(210, 20%, 10%)` | `hsl(54, 11%, 87%)` | `hsl(0, 0%, 0%)` |
| font, secondary | `hsl(210, 20%, 100%)` | `#000` | `hsl(0, 0%, 100%)` |
| font, inputs | `hsl(210, 20%, 40%)` | `hsl(215, 14%, 40%)` | `hsl(0, 0%, 0%)` |
| header | `hsl(215, 20%, 10%)` | `hsl(215, 20%, 10%)` | `hsl(215, 20%, 10%)` |
| header, links | `hsl(0, 0%, 100%)` | `hsl(0, 0%, 100%)` | `hsl(0, 0%, 100%)` |
| scrollbar, track | `rgb(255,255,255)` | `rgb(0,0,0)` | `rgb(255,255,255)` |
| scrollbar, thumb | `rgb(0,0,0)` | `rgb(255,255,255)` | `rgb(0,0,0)` |
| the graph's curves | `grayscale(1)` | `invert(1) grayscale(1)` | none |

The curve colours themselves are the native's golden-angle palette
(`squiglab_core::color`), not the reference's grey: the reference greys its
curves because its page carries a single accent, and the native's window
carries many.

## The type

Open Sans throughout, with monospace for figures. The reference's sizes:

| Use | Size | Line |
|---|---|---|
| The logo | 18 px | 24 px |
| Buttons, selects | 12 px | 1em |
| Labels, inputs, chips | 11 px | 1em |
| The graph's axis labels | 11 px | 1em |
| Body | 13–14 px | 1.6em |

Figures are tabular: the reference right-aligns the values in a field so the
digits line up, and reserves room to the right of a right-aligned value for its
unit.

## The metrics

| | |
|---|---|
| The header | 48 px tall; the logo column 350 px |
| The left column | 350 px, of which 94 px is the brand column and 256 px the list |
| A list row | 36 px tall, 11/12 px padding, 6 px radius |
| An input | 36 px tall, 10 px padding, 6 px radius, the right edge square where it meets the next field |
| A select | 12/14 px, 10/22/10/10 px padding, 6 px radius |
| A button | 12 px, 6 px radius |
| A slider | 16–18 px tall, 100 px radius, a 12 px round thumb |
| A toggle | a 4 px gap, 6 px radius |
| The EQ table | four columns, 4 px gaps, 20 % wide fields |
| Radii | 6 px, 4 px, 10 px, 100 px for a slider, 50 % for a round thing |
| Gaps | 4, 6, 8, 10 px |
| Padding | 10/16 px, 11/16 px, 12/12/10 px |

## The layout

The reference's page is a flex row: a fixed 350 px column on the left, and a
flexible column on the right that scrolls. The graph sits at the top of the
right column, under a row of tools that scrolls sideways.

**The graph scales with the viewport's height, not its width:**

```css
div.graph-sizer {
    width: 100%;
    max-width: calc(50vh * 2.2);
    max-width: calc((100vh - 300px) * 2.2);   /* the later one wins */
    margin: 16px auto 0 auto;                  /* centred */
}
```

So the graph is at most 2.2 units across for every unit of the window's
height, less 300 px of chrome, and it is centred in its column. A window that
is very tall gets a wide, short graph; a window that is short gets a narrow
one. The graph never distorts.

The reference's breakpoints:

| Width | What changes |
|---|---|
| 1500 px and an aspect of 2:1 or wider | the columns widen |
| 1200 px | the tools row tightens |
| 1000 px | the columns stack: the graph on top, the list below it, scrolling |
| 500 px | the controls tighten further |
| 500 px tall and an aspect of 3:2 or wider | the graph gives up height |

## The parts

The header, the three tabs (Brands, Models, Equalizer), the graph with its
toolbar, the manager, the target strip, and the equalizer panel with its
constraints, the sound tools with their band, the upload panel, the change
history, and the augment card on a model row.

`docs/interface-parity.md` says which of these the native has and which it
has still to build.
