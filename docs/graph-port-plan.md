# Graph port plan: `web/graphtool.js` -> Rust

`web/graphtool.js` is ~15,000 lines. A literal 1:1 port is a multi-phase
project; removing the existing Rust graph modules before a replacement exists
would break the build. The plan below ports in vertical slices, each of which
compiles and is testable before the next starts.

## Current state

- `crates/squiglab-core/src/graph.rs` (592 lines): zoom regions, plot curves,
  label placement, frequency bands, axis formatting, y-range scaling. Already
  a partial port of the graph's vocabulary.
- `crates/squiglab/src/graph.rs` (1329 lines): egui/d3 rendering, plot rect,
  curves, labels, EQ editor, interaction. In the middle of a d3 rewrite.
- `web/graphtool.js`: the authority for behaviour and geometry.

## Phases

1. **Inventory.** Map every top-level function/variable in `graphtool.js` to a
   Rust module and mark which are already ported (core graph vocabulary, EQ
   maths). *Output: this table.*
2. **Core math.** Port the pure helpers that have no UI: smoothing
   (`smooth_prep`, `smooth_eval`, `smooth`), ISO normalisation
   (`init_normalize`, `find_offset`), averaging (`avgCurves`, `getAvg`),
   channel helpers, and the marker geometry constants.
3. **Renderer skeleton.** Introduce `graph_js` modules behind a feature flag
   that reproduce the reference's SVG/D3 output as tessellated shapes, without
   interaction, so the picture can be compared side by side.
4. **Interaction.** Port pointer handlers (`eqGraphPointerDown`, drag, wheel)
   and the tone-generator brush.
5. **Cutover.** Switch `graph::show` to the new module, then delete the old
   graph files once nothing references them.
6. **Validation.** Side-by-side pixel comparison at 1440x900 for every zoom
   region and EQ state.

## Inventory (abridged)

| JS symbol | Rust home | Status |
|---|---|---|
| `smooth`, `smooth_prep`, `smooth_eval` | `squiglab_core::graph` | ported |
| `init_normalize`, `find_offset` | `squiglab_core::graph` | ported |
| `avgCurves` | `squiglab_core::graph::avg_curves` | ported (getAvg stays caller-side) |
| `x`, `y`, `xAxis`, `yAxis` | `squiglab::graph` | partial (d3 port) |
| `drawLabels`, `clearLabels` | `squiglab::graph` | partial |
| `eqGraphPointerDown/Move/Wheel` | `squiglab::graph` | partial |
| marker constants (`EQ_GRAPH_MARKER_*`) | `squiglab::graph` | not ported |

## Rule

No graph Rust file is deleted until its replacement compiles and the
side-by-side comparison agrees. `cargo test --workspace` must stay green at
every phase.
EOF
echo created