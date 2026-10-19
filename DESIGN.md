---
name: SquigLab
description: The measurement bench — a window for reading frequency responses against reference targets.
colors:
  accent: "#3B7DBF"
  accent-contrast: "#336699"
  accent-dark: "#3870A6"
  page: "#F5F5F4"
  page-contrast: "#E7E7E4"
  page-contrast-more: "#A6A6A6"
  graph: "#FFFFFF"
  inputs: "#F9FAFA"
  font-primary: "#14191F"
  font-secondary: "#FFFFFF"
  font-inputs: "#52667A"
typography:
  row-name:
    fontFamily: "Open Sans, sans-serif"
    fontSize: "14px"
    fontWeight: 700
  row-detail:
    fontFamily: "Open Sans, sans-serif"
    fontSize: "14px"
    fontWeight: 400
  chip:
    fontFamily: "Open Sans, sans-serif"
    fontSize: "12px"
    fontWeight: 400
  channel-label:
    fontFamily: "Open Sans, sans-serif"
    fontSize: "10.5px"
    fontWeight: 400
  field:
    fontFamily: "Open Sans, sans-serif"
    fontSize: "12px"
    fontWeight: 400
rounded:
  chip: "6px"
  field: "6px"
  control: "6px"
spacing:
  row-gutter: "16px"
  chip-gap: "6px"
  category-gutter: "16px"
  cell-gutter: "6px"
  field-inset: "11px 6px 11px 0"
components:
  chip-outline:
    backgroundColor: "{colors.page}"
    textColor: "{colors.accent-contrast}"
    rounded: "{rounded.chip}"
    padding: "11px 16px"
  chip-selected:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.font-secondary}"
    rounded: "{rounded.chip}"
    padding: "11px 16px"
  row-button:
    backgroundColor: "{colors.page}"
    textColor: "{colors.font-inputs}"
    rounded: "{rounded.control}"
    width: "36px"
    height: "36px"
  offset-field:
    backgroundColor: "{colors.inputs}"
    textColor: "{colors.font-inputs}"
    rounded: "{rounded.field}"
    width: "70px"
    height: "36px"
    padding: "{spacing.field-inset}"
---

# Design System: SquigLab — The Measurement Bench

**Creative North Star: "The Measurement Bench"**

A measurement bench is a surface you read numbers off, not a page you browse.
Every row on it is an instrument panel: the curve's colour is a status light,
the name is a label plate, the small graphic in the middle is the reading
itself, and the icons to the right are the switches that act on it. The
reference at squig.link is the authority for this surface — its row metrics,
its chip treatment and its key graphic are the source, not an interpretation of
them — and this document describes what that surface is, so the native can be
checked against it rather than guessed at.

The rows are dense on purpose. A reviewer comparing a measurement against a
target wants the curves, the names and the controls in one horizontal scan,
so the row is a single 50-pixel line with everything on it and nothing
wrapping. The chrome is flat and quiet: the only saturated colour in the
bench is the curve's own, and the accent is reserved for what is currently
selected. The result should read like a rack of instruments — legible at a
glance, unfussy, and unmistakably about measurement.

**Key Characteristics:**
- Every row is one 50 px line; nothing wraps and nothing scrolls sideways.
- The curve's colour is the only saturated colour in the row, and it appears
  in exactly two places: the dot, and the key graphic.
- The accent marks selection and nothing else.
- Text is for names, numbers are for measurements, icons are for actions.

## Colors

The palette is the reference's own — a warm off-white bench, a blue accent,
and a small set of greys that step from the page down to the disabled.

### Primary
- **Bench Blue** (`#3B7DBF`, `hsl(210, 53%, 49%)`): the accent. A selected
  target chip and nothing else wears it. In the dark theme the native keeps its
  own `#3870A6` instead, because the curves there are the native's own
  coloured palette and the reference's yellow would fight them.
- **Deep Bench Blue** (`#336699`, `hsl(210, 50%, 40%)`): the text of an
  unselected chip, which is the accent darkened until it clears 4.5:1 on the
  page.

### Neutral
- **Bench White** (`#F5F5F4`, `hsl(54, 5%, 96%)`): the page the rows sit on.
- **Rule Grey** (`#E7E7E4`, `hsl(54, 5%, 90%)`): the hairline that separates one
  row from the next.
- **Faint Grey** (`#A6A6A6`, `hsl(54, 0%, 65%)`): the border of a chip, of a
  field and of a button, and the colour of a target's name.
- **Graph White** (`#FFFFFF`, `hsl(54, 0%, 100%)`): the plot's own ground.
- **Input White** (`#F9FAFA`, `hsl(210, 10%, 98%)`): the offset field.
- **Label Ink** (`#14191F`, `hsl(210, 20%, 10%)`): the name on a measurement row.
- **Chalk** (`#FFFFFF`, `hsl(210, 20%, 100%)`): the text on a selected chip.
- **Dial Grey** (`#52667A`, `hsl(210, 20%, 40%)`): the numbers in a field and
  the icons of an action.

### Named Rules

**The Two-Colour Rule.** A row carries the curve's colour in exactly two
places — the dot at its left and the key graphic in the middle. Nowhere else.
The eye must be able to find the curve in a column of rows by colour alone.

**The Accident Marks Selection.** The accent appears on a selected chip and on
nothing else in these rows. If an icon is the accent, it means the thing is
on.

**The Grey Target.** A target row is not a measurement row. Its name is
Faint Grey rather than Label Ink, because a target is a reference and the
measurement is the subject.

## Typography

**Display Font:** Open Sans (with sans-serif)
**Body Font:** Open Sans (with sans-serif)
**Label/Mono Font:** Open Sans (with monospace)

**Character:** One family, used honestly. The reference sets the row's names
in the same face as everything else, and the numbers are the same size, so
the row reads as one object rather than a label beside a field.

### Hierarchy
- **Row name** (700, 14px): the brand and the model, on a measurement row.
  On a target row the same text is 400 and Faint Grey.
- **Row detail** (400, 14px): the variant name — the "EQ" of an equalizer
  child — and the "L ONLY" / "R ONLY" beside the key graphic.
- **Chip** (400, 12px): a target's name.
- **Channel label** (400, 10.5px): the "L" and "R" on the key graphic, and
  the word "only" beneath them at 7.5px.
- **Field** (400, 12px): the offset, and every other figure in the rows.

### Named Rules

**The Half-Point Rule.** The small labels — "L", "R", "only", "Target" — sit at
half-point sizes beside the 14 px row, never on their own line. A label that
needs a second line is a label that does not belong.

## Layout

One row is 50 points tall and never taller. Its cells run left to right in a
fixed order: the curve's colour dot (50 pt), the name (flexible), the key
graphic (the reading, 65 pt), the offset field (70 pt), and then the action
buttons (36 pt each, 6 pt apart). A hairline separates one row from the next.

The target strip above the rows is a single line that scrolls sideways, with
the categories separated by a 16 pt gutter and a 1 pt divider. Its chips are
11 × 16 pt inside a 6 pt radius.

**No Row Wraps.** If a name is too long it is truncated, never wrapped. The
row's height is fixed, and a row that changes height makes the column harder
to read than a truncated name does.

## Elevation & Depth

The bench is flat. Depth comes from the hairline between rows and from the
one-pixel border around a field, a chip and a button — never from a shadow.
There is one exception: a button under the pointer takes the page's own
hover tone, which is a change of fill rather than a lift.

### Shadow Vocabulary
- **None.** The reference's rows have no shadows, and a shadow on a 50-point
  row reads as a card.

### Named Rules

**The Flat Row Rule.** A row is a line, not a card. No fills, no shadows, no
rounded corners around the row itself. Only the hairline.

## Shapes

Corners are 6 points wherever there is a corner at all: a chip, a field, a
button. The key graphic and the colour dot are the only non-rectangular
shapes on the row, and they are there because they carry meaning — a reading
and a colour. Nothing else is rounded, because nothing else is a thing you
press.

## Components

### The row

- **Height:** 50 points, fixed.
- **Order:** dot, name, key graphic, offset field, actions.
- **Separation:** a 1 px hairline in Rule Grey above each row.

### The key graphic

The reading of the curve, and the control that chooses the channel. It is the
reference's own drawing: a pair of tapered strokes meeting in a point on the
left and opening to the right, one per channel, filled with the curve's
colour. A single channel is a straight line. Beside it, "L" and "R" in 10.5 px;
a word "only" in 7.5 px under the channel that has been isolated. A target's
graphic is a straight line with the word "Target" in 8 px, in the curve's
colour.

- **Interaction:** clicking the left half shows the left channel only,
  clicking the right half the right only, clicking the point in the middle
  shows both.

### The target chip

- **Style:** 11 × 16 pt padding, 6 pt radius, 1 pt border in Faint Grey, the
  page's fill, the accent's darker text at 12 px.
- **Selected:** the accent's fill, the accent's border, and Chalk text.
- **Grouping:** chips are 6 pt apart; a new category is 16 pt away, with a
  1 pt divider between the categories.

### The offset field

- **Style:** 70 × 36 points, 6 pt radius, the input white, a 1 pt border in
  Faint Grey, the number right-aligned in Dial Grey at 12 px.
- **Focus:** the border takes the accent.

### The action buttons

- **Shape:** 36 points square, a 6 pt radius, a 1 pt border in Faint Grey.
- **Marks:** the reference's own — the eye and the crossed eye for hiding, the
  pin, the arrow into a tray for the export, the circled cross for removing,
  and the words BASE and -LINE for the baseline. Each is drawn from the
  reference's own paths, at 16 points inside the 36-point cell.
- **States:** a hover fills the cell with the page's hover tone. A pinned or
  baseline button takes the accent.

## Do's and Don'ts

### Do:
- **Do** draw the key graphic in the curve's own colour, from the reference's
  path, so the row can be read without the graph.
- **Do** keep the row at 50 points and truncate a name that does not fit.
- **Do** use the reference's marks and its 6-point radius, and 1-point
  borders in Faint Grey.
- **Do** show a target's name in Faint Grey and a measurement's in Label Ink.
- **Do** make the whole target strip one line that scrolls sideways.

### Don't:
- **Don't** put a second colour on a row. The curve's colour is the dot and
  the graphic, and the accent is selection.
- **Don't** wrap a row, or let a row's height change with its content.
- **Don't** give a row a fill, a shadow or rounded corners of its own.
- **Don't** draw a channel selector as text buttons where the reference has
  the key graphic.
- **Don't** add a gap, a radius or a border size that the reference does not
  have: 6 pt gaps, 6 pt radii, 1 pt borders, 50 pt rows.
