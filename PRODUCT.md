# Product

<!-- impeccable:product-schema 1 -->

## Platform

egui

## Users

Audio engineers and in-ear monitor reviewers who compare frequency-response measurements against reference targets. They work in measurement-focused sessions (often dark rooms, long durations) and rely on dense, accurate curve visualization rather than decorative chrome.

## Product Purpose

SquigLab is a native desktop tool for comparing frequency-response measurements against reference targets. Users select measurements and targets, adjust processing (normalization, smoothing, zoom, channels), fit an equalizer to a target, and read the resulting curves against each other. Success means accurate, scannable comparison at a glance with no visual interference from the interface.

## Positioning

The same measurement-comparison logic as the web reference at squig.link, rebuilt in the reference's own arrangement: the header, the fixed column of brands and measurements, the graph and what belongs to it. The measurement palette—not the chrome—owns the visual attention.

## Operating Context

- Long comparison sessions in controlled lighting (dark default preferred).
- Multiple measurements loaded at once; rows are tight by design.
- Native desktop environment (Windows/macOS/Linux) via eframe.
- Reference for visual language: squig.link web tool (same product, different surface).

## Capabilities and Constraints

- Curve comparison with selectable measurements and reference targets.
- Processing controls: normalization, smoothing, zoom, channels.
- A parametric equalizer: the band table, a fit to a target, and the constraints that say
  what the fit may produce — which filter kinds, how many, over what frequency, gain and Q,
  and whether the left and right channels may carry filter banks of their own.
- Undo and redo of the equalizer's changes.
- The sound tools: pink noise and a tone generator whose slider sweeps a band of frequencies.
- Measurements may be uploaded, with the warning that another rig's measurements do not mix.
- The model list carries the review, the price and the shop link, where the phone book knows them.
- Design authority: the web reference at squig.link, extracted into `docs/design-system.md`.
  The interface matches its chrome, layout, scaling, spacing and themes.
- Fonts: Open Sans (Regular/Bold), vendored at `crates/squiglab-core/assets/fonts/`.
- Theme: the reference's three — light, dark and high contrast. The dark theme keeps the
  native's blue accent rather than the reference's yellow, because the curves are the
  native's own coloured palette.
- Technical constraint: egui 0.36 + eframe; immediate-mode UI rebuilt per frame.
- No accessibility mandate stated; AccessKit enabled by default on eframe.

## Brand Commitments

- Product name: SquigLab.
- Reference identity: squig.link (same measurement product, web reference).
- Visual authority: squig.link, extracted into `docs/design-system.md`.
- Voice: precise, technical, measurement-focused; no marketing or emotional copy.

## Evidence on Hand

- `docs/design-system.md`: the reference's tokens, type scale, spacing, metrics and themes,
  read from the reference's own stylesheet.
- `docs/interface-parity.md`: what the reference has, what the native has, and what it does not.
- `web/`: the reference's own source, kept whole so the native can be checked against it.
- Assets: Open Sans with its OFL licence, the product's mark, measurement data samples.
- Source: `crates/squiglab-core/` (the program, with no egui in it) and `crates/squiglab/`
  (the interface that draws it).
- No testimonials, case studies, or external press recorded.

## Product Principles

1. The graph is the product. Chrome frames without competing.
2. Density is earned: tight rows with rhythmic spacing multiples, not arbitrary compression.
3. Numbers are measurements: monospace, tabular, aligned; no decorative typography.
4. The design authority is the web reference (extracted in `docs/design-system.md`), not an
   incumbent house style.

## Accessibility & Inclusion

Standard desktop accessibility via eframe's default AccessKit integration. No product-specific mandate stated; future custom interactive widgets should register roles/names and maintain ≥44×44 logical-point touch targets.
