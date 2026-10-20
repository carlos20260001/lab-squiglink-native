# The Stitch mockup: the measurement bench

The web reference (`web/`, the `pink-eq` branch of the site repository) is
the authority. This file records the Stitch mockup built against it, and how
the mockup was brought to the reference.

## Where it is

| | |
|---|---|
| The project | `SquigLab — The Measurement Bench`, `projects/7241344322456677515` |
| The design system | `assets/a503c7f03a0344ad86ceaf6fa11bc7a4`, made from the repository's own `DESIGN.md` |
| The screen | `The Measurement Bench - CrinGraph`, `projects/7241344322456677515/screens/2930500ffaa64031be0aaef0272256a4` |
| The source | The screen's HTML, 1440×900, uploaded as an HTML screen and given the design system |

The design system is the repository's `DESIGN.md` unchanged, so the mockup
and the native draw from the same tokens: Bench Blue `#3B7DBF`, the page
`#F5F5F4`, Open Sans, 6 pt radii, and no shadows.

## The corrections that were made, and what each one was for

Each was measured in the reference with
`npx --yes @playwright/cli@0.1.22 -s=parity` and then made in the mockup.

| The correction | What the reference does |
|---|---|
| **The equalizer is its own tab** | Choosing `Equalizer` hides the list panel and shows the equalizer, the sound tools, the upload and the change history. Choosing `Brands` or `Models` hides all of that. The two are never on the screen together. |
| **The two columns overlap** | The brands column is 289 pt wide at the left; the models column is 289 pt wide at the right, over it. On the `Brands` tab the models slide 229 pt to the right, so a sliver of them still shows. |
| **The brands fade behind** | On the `Models` tab the brands are veiled with the page's own colour at 80 %, so the names behind are quiet, and a press on them brings the `Brands` tab back. |
| **The list rows** | Forty points tall, 12 px, a 6 pt radius, 6 pt between them; the models are 256 pt wide, with the `+` being the reference's own mask. |
| **The band table** | Ten bands, empty ones included, in four columns with 4 pt gaps and no card around them; the heading is 12 pt in Faint Grey; the fields are 26 pt tall with the unit inside on the right. |
| **`Auto EQ`** | An ordinary button — a Faint Grey border, the page's fill, the accent's darker text — not a filled one. |
| **The marks** | `Reset` carries the reference's own arrow; `Export` carries the reference's download mark; the `+` on a model is the reference's own plus. The Wavelet export has no mark, as the reference writes it. |
| **The sound tools** | The source titles are 13 pt in Faint Grey, not 11 pt capitals; the cards have a 10 pt radius; the play button is a 44 pt pill. |
| **Upload and Change History** | Two separate panels, each with a 14 pt bold title in the accent's darker text, and the reference's own wording in each. |
| **The tools row** | The reference's own group of four — inspect, label, screenshot, recolor — at the row's right, reached by scrolling. |
| **The key graphic** | The reference's own path, filled with the curve's colour through a gradient, with the channel letters at 8 pt in Faint Grey and the word `only` beside them. |
| **The plot** | The resting plot carries no tooltip, no crosshair and no marker; the curve labels sit inside the plot, clear of the axes; the axis labels are `Type / Frequency / Gain / Q` and the band names carry no frequency ranges. |

## The method

The reference is brought up beside the mockup, and the two are compared at
the same size, in the same state:

```sh
python3 -m http.server 8765 --bind 127.0.0.1     # the repository root
# the reference:  http://127.0.0.1:8765/web/index.html
# the mockup:     the screen's HTML, served the same way
```

Both are then driven with the same Playwright session, resized to 1440×900,
and captured:

```sh
PW="npx --yes @playwright/cli@0.1.22 -s=parity"
$PW open "http://127.0.0.1:8765/web/index.html"
$PW resize 1440 900
$PW screenshot --filename /tmp/opencode/ref.png
```

The mockup's HTML is edited, uploaded, and given the design system again.

## What is still different

- The mockup's curves are drawn, not computed: there is no data behind them,
  and the plot is a picture of the reference's plot rather than a live one.
- The reference's header links to the rest of the site; the mockup's do
  nothing.
- The `Dark` and `Contrast` themes of the reference are not in the mockup;
  it is the light theme only.
- The reference's own tutorial overlay, and its `Lorem ipsum` descriptions,
  are not carried over — there is nothing to carry.
