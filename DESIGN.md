# CrinGraph Design System

## Product context

CrinGraph is a native desktop tool for comparing frequency-response measurements of
in-ear monitors. Users select measurements and reference targets, adjust processing
(normalization, smoothing, zoom, channels), and read the resulting curves against each
other. It is an **Operate** surface: the user is in a task, comparing data, and the
interface should disappear into that task.

Reference for visual language and information architecture is the web tool at squig.link,
which is the same product built on D3 and rendered in the browser. The native app adopts
its type family, accent, and control vocabulary, but keeps its own denser native layout.

## Principles

1. **Chrome is neutral, curves are not.** The measurement palette is a wide, saturated
   golden-angle HCL set. Any saturated UI color would fight it. The only saturated color
   in the interface is the accent, and it is reserved for primary actions, current
   selection, and focus.
2. **The graph is the product.** Panels and toolbar frame it; they do not compete with
   it. Surfaces step down in brightness as they approach the graph so the eye lands there.
3. **Density is earned, not cramped.** Users compare many measurements at once, so rows
   are tight. Every gap is still a multiple of the spacing unit so it reads as rhythm.
4. **Numbers are measurements.** dB, Hz, and phon values render monospace and tabular so
   columns align and values do not shift width as they change.

## Type

Family **Open Sans** (Regular 400, Bold 700), vendored at `assets/fonts/`. This is the
family the web tool declares and renders. Monospace data uses egui's bundled **Hack**.

Scale — fixed sizes, ratios 1.13–1.2, not fluid:

| Step | Size | Role |
|---|---|---|
| Micro | 10 | counts, units, attribute meta |
| Caption | 11.5 | secondary labels, monospace data |
| Body | 13 | reading text, controls, default labels |
| Subtitle | 15 | section headings (bold) |
| Title | 17.5 | toolbar/panel headings |
| Heading | 20 | app title, large readouts (bold) |

`Body` and `Button` share 13 because `Button` is egui's fallback for plain labels;
keeping them equal stops incidental labels from stepping down. Emphasis comes from bold,
size, or fill — not from bolding everything.

Text styles map to these steps in `theme.rs::text_styles`, plus named roles `display`,
`title`, `label`, `value`, `meta`.

## Color

Two themes. **Dark is the default**: users measure and compare in a dark room or for long
sessions, and the measurement palette reads better against a dark ground.

Surfaces step from sunken → base → panel → raised as they come forward:

| Token | Dark | Light | Use |
|---|---|---|---|
| `surface_sunken` | #101216 | #E4E8EE | inputs, wells |
| `surface_base` | #16181D | #F6F7F9 | graph canvas, content ground |
| `surface_panel` | #1E2127 | #EEF0F4 | side panels, toolbar |
| `surface_raised` | #262A32 | #FFFFFF | controls, popups |

Text clears 4.5:1 on every surface it sits on (asserted in tests):

| Token | Dark | Light |
|---|---|---|
| `text_primary` | #E9EBEE | #12151A |
| `text_secondary` | #A6ACB8 | #4A5260 |
| `text_tertiary` | #848B98 | #5F6775 |

Accent — calm blue, matching the web tool's `hsl(210,70%,40%)`:

| Token | Dark | Light | Use |
|---|---|---|---|
| `accent` | #3870A6 | #2B6CB0 | primary actions, selection, focus |
| `accent_hover` | #427FB9 | #337BC4 | hover |
| `accent_active` | #2E5F8D | #235A99 | pressed |
| `on_accent` | #FFFFFF | #FFFFFF | text on an accent fill (≥4.5:1, asserted) |

The dark accent is deliberately dimmer than it looks right at: white text on the brighter
#3E82C6 falls to 4.03:1, so the token is darkened to pass.

Semantic status — separate from the accent, never the same hue: `warning`, `error`,
`success`.

Structural: `border`, `border_strong`, `divider`, `hover_fill`, `active_fill`.

## Spacing

4pt base unit (`SPACE_UNIT`). Steps: Micro 2, Tight 4, Compact 8, Normal 12, Relaxed 16,
Loose 24, Section 32. egui's `Margin` stores whole points, so margin values round.

Control radius 4px (`RADIUS_CONTROL`). Control height 22px.

## States

Every interactive control resolves through egui's widget state set (inactive, hovered,
active, open) plus disabled, and carries: background fill, border, foreground. Selection
and focus use the accent. Disabled text uses `text_disabled`.

## Application rules

- `theme::ensure_ready` is the single place appearance is configured: it registers fonts
  once and installs a full `Style` per theme via `set_style_of`. Calling
  `Context::set_visuals` elsewhere silently overrides the active theme.
- New UI resolves to a token. Raw hex and ad-hoc sizes are not introduced outside
  `theme.rs`.
- Contrast and type-scale invariants are enforced by tests in `theme.rs`.

## Bundled fonts

Open Sans Regular and Bold at `assets/fonts/`, SIL Open Font License 1.1; the licence and
copyright notice ship alongside at `assets/fonts/OFL.txt`. The fonts are used unmodified.