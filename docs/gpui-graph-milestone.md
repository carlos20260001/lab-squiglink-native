# Spec: GPUI graph milestone

Status: draft. Applies to `crates/squiglab-gpui`. The egui frontend
(`crates/squiglab`) and `crates/squiglab-core` are the authority for
behaviour; this spec describes what the GPUI frontend must gain so that
the graph act of the journey (see `docs/feature-parity.md`) can be
walked in it.

## Problem statement

The GPUI frontend draws the shell (header, tabs, search, brand/model
lists, three themes) but renders an empty `flex_1` pane where the graph
belongs. A user cannot look at a measurement, read its curve, inspect a
frequency, or edit the equalizer, so the journey stops after "Choosing".

## Solution

Port the graph from the egui frontend into GPUI, reusing the core's
mathematics (`Transform`, `y_range`, `scaled_y_range`, `place_labels`,
`nearest_point`, `format_frequency`, `format_axis_frequency`,
`tick_db`, `FREQUENCY_BANDS`, `PlotCurve`) and the egui frontend's
`graph_port` as the behaviour reference. The GPUI painter is the only
new code; layout, input rules and equalizer commands come from the core
and the (to be extracted) `EqEditorState`.

## User stories

1. As a user, I want to see the loaded measurements drawn as curves on
   a logarithmic frequency axis, so that I can compare them.
2. As a user, I want the dB axis to be draggable with two grips, so
   that I can change the visible range.
3. As a user, I want a double-click on the dB scale to reset the range.
4. As a user, I want frequency and dB grid lines and labels, so that I
   can read values.
5. As a user, I want band names under the axis, so that I can orient
   myself.
6. As a user, I want curve labels that do not overlap, so that I can
   read every name.
7. As a user, I want the wordmark watermark and the rig line, so that
   the graph is identified.
8. As a user, I want an inspector line at the pointer's frequency with
   each curve's reading, so that I can probe a point.
9. As a user, I want to draw a band out of the curve by dragging on it,
   so that I can shape the response.
10. As a user, I want to move a band by dragging its mark, so that I
    can tune it.
11. As a user, I want the wheel over a mark to change the band's width.
12. As a user, I want the right button on a mark to cycle its filter
    kind.
13. As a user, I want a drag away from the curve to brush a frequency
    range for the tone generator.
14. As a user, I want to export the graph as an SVG, so that I can keep
    it.
15. As a user, I want the GPUI graph to agree with the egui graph and
    the reference pixel-for-pixel at 1440 by 900.

## Implementation decisions

- **Seam.** One `GraphFrame { rect, plot, transform, curves, min, max }`
  computed per frame in the core (or in a shared helper), consumed by
  both painters. This is Candidate 1 of the architecture review; it is
  the prerequisite for everything below.
- **Painting.** GPUI has no D3; the egui frontend already ports D3 v5's
  scale and curve operations through `crates/squiglab-core`'s `d3`
  module and `graph.rs`. GPUI will call the same core functions and paint
  with `window.paint_path` / `paint_stroke` / `paint_quad` / `paint_glyph`
  inside a `canvas` element, or via `paint_svg` for the export.
- **Input.** GPUI's `canvas` (or an interactive `div` with hitboxes)
  provides pointer events. The axis lock, hit radius (`MARKER_HIT_PX`),
  drag threshold (`DRAG_THRESHOLD_PX`) and Q-step logic must be extracted
  into an `EqEditorState` reducer (Candidate 3) so they are unit-testable
  and shared with egui.
- **Equalizer commands.** The editor calls `equalizer::move_band`,
  `cycle_band_type`, `set_band_q`, `add_band_at` (in `crates/squiglab`).
  These must move into `squiglab-core` or a shared crate so GPUI can
  call them without depending on egui. (Currently they live in the egui
  crate.)
- **Renderer-agnostic scene (Candidate 5).** Optional but highest
  leverage: a `GraphScene` of paths, labels, rects built by a pure
  function from `GraphFrame` + session state. If done, both painters
  consume it and parity is by construction.
- **`db_at` interpolation** must move into core (Candidate 4) so both
  frontends share it.
- **Dependencies.** GPUI graph needs no new crates: `gpui::canvas`,
  `gpui::Path`, `gpui::paint_path`, and the core's existing math.
- **Shared state.** `app.graph_drag` and `app.session` are the same
  types in both frontends; no new state.

## Testing decisions

- Unit tests on `GraphFrame` construction (range, transform round-trip).
- Unit tests on `EqEditorState` (pointer down/move/up, wheel, right
  click, axis lock) against synthetic inputs, not Xvfb.
- Parity checks: render the same session in egui and GPUI at 1440 by
  900, crop the graph pane, compare pixel histograms (as the shell
  milestone did).
- `cargo test --offline --workspace` must stay green; the GPUI crate
  gains its first tests here.

## Out of scope

- The equalizer panel (band table, constraints, history, import/export):
  separate milestone.
- The sound tools (pink noise, tone generator, A/B, volume, music):
  separate milestone.
- The file picker and music player: already native-only, not GPUI yet.
- Mobile pickers: unchanged.
- Rewriting the egui graph: it stays as the reference implementation.

## Further notes

- The `data_delta` → `data_deltas` typo was fixed in both frontends as
  part of this work's run-up.
- The `bank_rows` index mismatch (multi-bank editing) was fixed.
- The debug `eprintln!` in `curves_into` was removed.
- `db_at` (curve interpolation) moved into `squiglab-core::graph` so both
  frontends share it.
- Candidate 1 done (on-screen): `show` computes the curve list, range and
  transform once; `transform_for` is gone and `scale` no longer recomputes
  the range. Tick computation is now shared via `db_tick_values` between
  the on-screen grid and `export_svg`, so the two can no longer drift on
  tick placement. `export_svg` still draws its own lines/curves (the SVG
  string builder is separate from the egui painter).
- Candidate 3 further done: the wheel Q-step (`equalizer::q_step`), the
  drag-start classification (`classify_drag_start`) and the band-drag
  update (`update_band_drag`) are now pure, unit-tested functions, and
  `eq_editor` calls them instead of inlining the logic. The drawing and
  the `Range` brush remain in `eq_editor`.
- `improve-codebase-architecture` surfaced the above deepening
  opportunities; Candidates 1 and 5 are the prerequisites for this
  milestone.