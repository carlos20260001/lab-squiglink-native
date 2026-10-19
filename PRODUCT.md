# Product

<!-- impeccable:product-schema 1 -->

## Platform

egui

## Users

Audio engineers and in-ear monitor reviewers who compare frequency-response measurements against reference targets. They work in measurement-focused sessions (often dark rooms, long durations) and rely on dense, accurate curve visualization rather than decorative chrome.

## Product Purpose

CrinGraph is a native desktop tool for comparing frequency-response measurements of in-ear monitors. Users select measurements and reference targets, adjust processing (normalization, smoothing, zoom, channels), and read the resulting curves against each other. Success means accurate, scannable comparison at a glance with no visual interference from the interface.

## Positioning

The same measurement-comparison logic as the web reference at squig.link, rebuilt in a dense native Rust/egui layout where the measurement palette—not the chrome—owns the visual attention. The dark default, tight spacing, and monospace data columns are first-class design choices, not port artifacts.

## Operating Context

- Long comparison sessions in controlled lighting (dark default preferred).
- Multiple measurements loaded at once; rows are tight by design.
- Native desktop environment (Windows/macOS/Linux) via eframe.
- Reference for visual language: squig.link web tool (same product, different surface).

## Capabilities and Constraints

- Curve comparison with selectable measurements and reference targets.
- Processing controls: normalization, smoothing, zoom, channels.
- Dense native layout with measurement palette as the hero visual.
- Design authority: DESIGN.md (existing, fully tokenized). No redesign requested.
- Fonts: Open Sans (Regular/Bold) vendored at `assets/fonts/`; Hack for monospace data.
- Theme: dark default, light available; tokens tested at ≥4.5:1 contrast.
- Technical constraint: egui 0.36/0.37 + eframe; immediate-mode UI rebuilt per frame.
- No accessibility mandate stated; AccessKit enabled by default on eframe.

## Brand Commitments

- Product name: CrinGraph.
- Reference identity: squig.link (same measurement product, web reference).
- Visual authority preserved from DESIGN.md and assets/fonts/.
- Voice: precise, technical, measurement-focused; no marketing or emotional copy.

## Evidence on Hand

- DESIGN.md: complete design system (tokens, type scale, spacing, states, themes).
- Assets: `assets/fonts/` (Open Sans, Hack, OFL license), measurement data samples.
- Source: `src/` (Rust/egui app), `Cargo.toml` (egui 0.36/0.37 + eframe).
- No testimonials, case studies, or external press recorded.

## Product Principles

1. The graph is the product. Chrome frames without competing.
2. Density is earned: tight rows with rhythmic spacing multiples, not arbitrary compression.
3. Numbers are measurements: monospace, tabular, aligned; no decorative typography.
4. Preserve the incumbent design authority (DESIGN.md) for any future refinement or extension.

## Accessibility & Inclusion

Standard desktop accessibility via eframe's default AccessKit integration. No product-specific mandate stated; future custom interactive widgets should register roles/names and maintain ≥44×44 logical-point touch targets.
