# UI Parity: SquigLab → squig.link, Full Replica

A roadmap for making SquigLab's interface a faithful replica of the squig.link
web interface. This supersedes the earlier `ui-parity.md` task list, which
covered only visual polish; its remaining visual items are folded in below.

**Reference captures** (regenerate before acting — the site changes):
`docs/design-reference/squig-desktop.png`, `squig-dark.png`, `squig-mobile.png`,
`squig-equalizer.png`, `squig-onboarding-modal.png`, `export-apo-sample.txt`.

Everything marked **verified** below was measured against the live site with
Playwright during the writing of this document, not inferred from the archived
`web/` copy — the two have drifted, and the live site is the target.

---

## Verified state of the live site

Recorded so these are not re-measured. These are the facts the rest of the
document rests on.

**Layout breakpoint (verified).** At 900px and below the page is a single
column: graph full width first (274px tall at 430 wide), then the
Brands/Models/Equalizer tabs, then the manager. At 1100px and above it is two
columns: a 349px tab column on the left and the graph beside it. The transition
sits between 900 and 1100.

**Scrolling (verified).** The graph is `position: sticky` at every width tested.
On desktop the scroll container is `div.parts-primary` with `overflow-y: auto`,
and it genuinely overflows (scrollHeight 1068 vs 852 visible) — so the **whole
left column and the graph scroll as one**, with the graph pinned. On mobile the
page body scrolls instead and there is no column scroller.

**Equalizer panel (verified, see `squig-equalizer.png`).** Title
"Parametric Equalizer?" with `?`, `Reset`, and a settings gear. A phone
dropdown and a target dropdown, then `Auto EQ`. A five-row filter table headed
Type / Frequency / Gain / Q, each row an enable checkbox, a type selector, and
three number inputs. Row controls `+`, `－`, `Sort`. Then `Import`, `Export`,
and a full-width `Export Graphic EQ (Wavelet)`. Below that, a Sound Tools
section with a `Compare A | B` toggle, Music, Pink Noise, a Tone Generator, and
`+ Add Filter`. Then `Range ↺ 20 Hz 20000 Hz`, then an Upload Data warning.

**Export formats (verified, captured directly):**
- Parametric — `docs/design-reference/export-apo-sample.txt`:
  `Preamp: 0.0002 dB` then `Filter 1: ON PK Fc 500 Hz Gain -3.5 dB Q 1.2`,
  CRLF separated.
- Wavelet — `GraphicEQ: 20 -2.7; 21 -2.6; 22 -2.6; …` semicolon-separated
  frequency/gain pairs, gains to one decimal, precompensation already folded in.

**The archived `web/` copy has drifted from the live site.** Its Equalizer has
AutoEQ range inputs the live site does not, a plugin system the live site does
not expose, and ten default bands where the live site shows five. Port the live
site's behaviour, using `web/equalizer.js` only for the DSP, which is still the
authoritative source for the filter maths.

---

## Ground rules

1. **Behaviour over pixels.** A missing control the reference has matters more
   than a 2px offset.
2. **One concern per commit**, in this file's granularity. Repository convention
   is an atomic commit with a terse imperative summary and a prose body wrapped
   near 78 columns.
3. **All UI resolves to `src/theme.rs`.** No raw hex or ad-hoc sizes outside it.
4. **The DSP is ported, not reimplemented.** `web/equalizer.js` is the source of
   truth; port it numerically and test against the JS output, not by feel.
5. **The graph palette wins.** EQ curve colours must come from the existing
   palette, not new hues.
6. **Verify in both themes**, and at 430px, 900px, 1100px, and 1600px wide.

---

## P1 — Dynamic viewport and responsive layout

The native app is a fixed-width desktop window. The reference has two distinct
viewports and fluid width behaviour.

- [ ] **Introduce a desktop/compact layout mode.** Below a ~1024px content width,
      stack graph → tabs → manager. Above it, tabs left and graph right.
      *egui approach:* branch in `app.rs::ui()` on `ui.available_width()`;
      `Panel::top` on `Panel::left` becomes conditional.
- [ ] **The library becomes a tab column, not a permanent sidebar.**
      squig.link's left column is `Brands | Models | Equalizer`. Matching this
      means the library sidebar and the Equalizer both live in that tab set, so
      the Equalizer can reuse the same panel slot rather than needing a new
      window. Depends on P3.
- [ ] **Graph width tracks the viewport.** The graph should take all remaining
      width and no more, at every window size — verified at 430/700/900/1100/1600.
      The plot aspect must stay capped (see the existing `PLOT_ASPECT_RATIO`),
      but the *container* width must be fluid.
- [ ] **Minimum window width drops to ~430px** so a compact layout is actually
      reachable. Currently `with_min_inner_size([860.0, 560.0])` — this forbids
      the mobile layout entirely. Confirms as a blocker before other work.
- [ ] **Toolbar collapses gracefully.** squig.link's toolbar stays one row but
      drops controls as width falls. Decide which of SquigLab's controls are
      dropped first, and hide the rest behind an overflow control rather than
      wrapping.
- [ ] **Manager rows reflow.** On a compact viewport the row should stack
      identity and controls vertically instead of pressing them into one line.

## P2 — Dynamic scrolling and sticky regions

- [ ] **Sticky graph.** squig.link pins the graph with `position: sticky` while
      the column scrolls past it. In egui this means the graph holds a fixed
      region while the library and manager scroll independently — the inverse of
      putting all three in one scrolling panel.
- [ ] **Keep the graph visible while the library scrolls.** Currently the library
      owns its own `ScrollArea`, so scroll positions are independent; verify the
      graph does not move.
- [ ] **Independent scroll areas for library and manager.** Both need their own,
      each with its own position preserved.
- [ ] **Themed scrollbars.** squig.link styles its scrollbars from the theme
      (custom `::-webkit-scrollbar` with `--scrollbar-color` /
      `--scrollbar-accent`). Stock egui scrollbars are a visible tell. Port
      equivalent styling through `egui::style::Spacing::scroll_*` if available,
      or accept stock ones and note it.
- [ ] **Scroll position persistence** for the library list, so a selection made
      at the bottom of a long list stays visible after reload.

## P3 — Equalizer and AutoEQ

The largest single feature. Port `web/equalizer.js` numerically.

- [ ] **DSP core** — `web/equalizer.js`:
      - `lowshelf`, `highshelf`, `peaking` RBJ biquad coefficient functions,
        with the source's clamp ranges (freq 1e‑6..1, q 1e‑4..1000, gain ±40 dB).
      - `calc_gains` — magnitude in dB summed over biquads.
      - `apply` — add EQ gain onto a `[hz, db]` curve.
      - `interp` — piecewise-linear resampler. **Shared with the measurement
        loader**, not EQ-specific.
      - Only `PK`, `LSQ`, `HSQ` exist. Do not invent LP/HP filters.
- [ ] **The EQ curve is a first-class curve.** In the reference the result is a
      synthetic curve named `"<phone> EQ"` added through the same path as a
      measurement, back-linked to its parent so normalization propagates both
      ways (`p.eq` / `p.eqParent`). In Rust this means extending
      `DisplayCurve` with a source enum rather than special-casing the renderer.
      This is the key architectural decision — get it right before any UI.
- [ ] **AutoEQ** — `autoeq()` in `web/equalizer.js`:
      - Two batches: `floor(maxFilters/2) - 1` candidates at a 1 dB threshold
        with treble excluded (filters ≤ 7000 Hz), widest Q first; then the
        remaining budget from re-searching the corrected curve at 0.5 dB.
      - `optimize()` per-band coordinate descent, three coarse-to-fine passes,
        early-exit on a worsening gain, then adjacent-filter merging (same
        frequency unit, ΔQ ≤ 0.1 → summed gain) and pruning.
      - Target selection: the first active target curve, else the second active
        measurement.
      - **The riskiest part of the port.** Port it first and diff filter
        magnitudes against the JS output in tests, before building any UI.
- [ ] **Tools the reference has**: `Reset`, `Sort`, `Import`, band add/remove,
      per-band enable checkboxes, a tone generator (see below).
- [ ] **Onboarding.** squig.link opens a "How to use EQ on Squiglink" modal with
      Beginner / Intermediate / Advanced / Export options. Decide whether to
      port it verbatim or replace it with native in-app guidance.

## P4 — Export formats

Both formats are specified from live captures; implement against these exactly.

- [ ] **Parametric (Equalizer APO).**
      ```
      Preamp: 0.0002 dB
      Filter 1: ON PK Fc 500 Hz Gain -3.5 dB Q 1.2
      ```
      - CRLF line endings.
      - `Preamp` is a full-precision float. `calc_preamp(fr1, fr2)` is
        `-max(fr2 - fr1)`, computed between the base phone's channel 0 and the
        EQ child's channel 0.
      - `LSQ` → `LSC` and `HSQ` → `HSC` on export; Equalizer APO uses the
        shorter names.
      - Bands whose freq, q, and gain are all zero are omitted.
      - Filename: `"{phone name} Filters.txt"`.
- [ ] **Wavelet (GraphicEQ).**
      ```
      GraphicEQ: 20 -2.7; 21 -2.6; 22 -2.6; …
      ```
      - Semicolon-separated `freq gain` pairs over the 127-band graphic grid.
      - Gains to one decimal.
      - Precompensation folded in — the whole curve is shifted by the preamp so
        the EQ never boosts.
      - Derived from the same filter set as the parametric export; the two must
        never disagree.
- [ ] **Round-trip tests.** Import an APO file exported from squig.link and
      assert the resulting filter set is identical, and vice versa. This is the
      only way to be sure a ported profile loads in a user's real DSP software.

## P5 — Sound Tools

- [ ] **Tone generator.** Web Audio `OscillatorNode`, sine, log-mapped frequency
      slider, smooth glide on drag. **Not** an automated sweep — the reference
      plays one continuous oscillator whose pitch is dragged. Needs a native
      audio dependency (`cpal` or `rodio`); this is the only new dependency the
      whole roadmap requires, so decide it explicitly rather than by drift.
- [ ] **Pink noise.** A pink-noise generator behind a play button.
- [ ] **Music playback** with a `Compare A | B` toggle, `+ Add Music`, and a
      platform search shortcut. Depends on an audio dependency.
- [ ] **Compare A/B** applies to the whole sound-tools section, not one source.

## P6 — Upload and provenance

- [ ] **Measurement upload.** `FileReader` → REW/AudioTool TSV parse, filename
      becomes the model name, **fewer than 128 points is rejected**, then
      resampled onto the shared `f_values` grid. A file ending `" R"` gets a
      null left channel spliced in front so channels align.
- [ ] **"Uploaded" brand.** Uploaded measurements land in a synthetic brand and
      are flagged so they are excluded from share URIs.
- [ ] **Target upload** appends `" Target"`, is deduplicated against configured
      targets, and is given a negative id so it colours as a target.
- [ ] **Provenance warning.** The reference states measurements from another rig
      are not comparable with its database. Carry the equivalent warning directly
      in the upload UI rather than as a dismissible banner.

---

## Explicitly out of scope

Do not implement. These are site features, not interface layout, and porting them
would make the desktop app worse.

| Reference feature | Why excluded |
|---|---|
| Site nav (`Headphones`, `Insights`, `Merch`, `Discord`, `HiFiGo`) | Website navigation; no desktop equivalent |
| Sponsorship slots, welcome and onboarding *ads* | Monetization. The EQ onboarding modal is in scope; ad slots are not |
| `DELTA` bar and reference-target chips | A measurement feature (delta curves), not layout |
| Footer database-link pills | Website chrome |
| `+Create Squiglink`, database dropdown | Website account and multi-database features |
| Embed mode, `expandableOnly`, `headerHeight` | iframe embedding for third-party sites |
| Analytics | `web/graphAnalytics.js` is an empty stub with no GA4 ID — nothing to port |
| EQ plugin system (`extraEQplugins`, `devicePEQ`) | Not exposed on the live site; was archived-only |
| Three-state contrast theme | A theming feature, not layout. SquigLab has light/dark and that suffices |

---

## Carried over from the previous list

Still open, unchanged:

- [ ] Labels follow curve ends rather than grouping in a corner — now a design
      choice, not a defect. SquigLab groups by default and can switch to
      curve-end, so this is resolved unless you decide otherwise.
- [ ] `assets/cringraph-app-icon.svg` is still the only `cringraph`-named asset.
- [ ] The two normalization values are shown one at a time; squig.link shows
      both. A maintainer decision.

---

## Definition of done

- [ ] Every P1–P6 item is implemented, or explicitly declined in a commit.
- [ ] `cargo test` passes, including new DSP round-trip tests against
      `web/equalizer.js`, and the existing 59 tests still pass.
- [ ] AutoEQ output diffed against the JavaScript implementation for a sample
      curve, not merely eyeballed.
- [ ] Both export formats round-trip against a file the live site produced.
- [ ] Captured at 430px, 900px, 1100px, and 1600px, in both themes, with no
      clipping, overlap, or toolbar wrap at the default size.
- [ ] `DESIGN.md` updated for any token added.
- [ ] Minimum window width reduced and the compact layout confirmed usable.
- [ ] This file's checkboxes updated as work lands.

## Suggested order

The ordering is chosen to derisk the expensive parts first.

1. **P1 viewport** — the compact layout is a prerequisite for the tab column, and
   the minimum window width is a blocker hiding everything else.
2. **P3 DSP core** — port and test the biquad maths with no UI. Small, pure, and
   the only part where numerical fidelity matters.
3. **P4 parametric export** — smallest end-to-end win once the DSP exists, and it
   proves the DSP is right.
4. **P3 AutoEQ** — the highest-risk algorithm; needs its own tests and review.
5. **P4 Wavelet export** — shares the aggregate calculator with step 3.
6. **P2 scrolling** — mechanical, and better done before the tab column makes the
   layout more complex.
7. **P3 Equalizer UI** — after the DSP, export, and AutoEQ all work.
8. **P5 sound tools** — last, because it introduces the only new dependency and
   is worthless without the EQ working.
9. **P6 upload** — independent; can slot in anywhere after the tab column exists.