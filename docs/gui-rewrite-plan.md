# Full GUI rewrite plan: 1:1 parity with `web/graphtool.js`

Scope: reproduce the reference's observable UI and behaviour exactly, using
`web/graphtool.js` (and `web/style-alt.css`) as the authority. This is a
multi-phase project; each phase must leave the workspace building and the
tests green.

## Phase 0 — Baseline (done)
- Audio/EQ correctness fixes (shared filter state, stereo playback, resampling,
  boundaries, sample-rate rebuild).
- Pure graph math ported (`avg_curves`, `smooth*`, `init_normalize`,
  `find_offset`, marker constants) with IEEE-754 parity.

## Phase 1 — Inventory and mapping
- Walk `graphtool.js` top to bottom and assign every function to a Rust module
  (`core::graph`, `squiglab::graph`, `selector`, `curves`, `equalizer`, `sound`,
  `music`, `upload`, `tools`).
- Record which are already ported and which are missing.

## Phase 2 — Rendering skeleton (graph)
- Axes, grid, curves, labels, markers, watermarks as tessellated shapes.
- Compare side-by-side at 1440x900 for Full/Bass/Mids/Treble zoom.

## Phase 3 — EQ editor interaction
- Pointer down/move/up, marker hit-testing, axis lock, width wheel, right-click
  kind cycling, drag-select block.

## Phase 4 — Sound tools UI
- Pink noise, tone generator, compare A/B, volume, music player, upload.

## Phase 5 — Selector, target strip, manager rows, inspector
- Brands/models tabs, target strip, manager rows, inspector, links.

## Phase 6 — Cutover
- Switch `graph::show` and panels to the new implementation.
- Delete legacy graph files only after parity is verified.

## Validation at every phase
- `cargo fmt --all -- --check` (for touched crates)
- `cargo test --workspace`
- Side-by-side pixel comparison at 1440x900.

## Rule
No file is deleted until its replacement compiles and the comparison agrees.