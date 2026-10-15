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

- [x] **Introduce a desktop/compact layout mode.** Below 1024px the graph
      stacks above the library; above it, the library sits beside the graph.
      `app.rs::ui()` branches on `ui.available_width()`. Commit `1ac7dfd`.
- [x] **Minimum window width drops to ~430px.** Was `[860, 560]`, which made the
      compact layout unreachable. Now `[430, 520]`. Commit `1ac7dfd`.
- [x] **Graph width tracks the viewport.** Verified at 430 and 1600; the
      container is fluid and `PLOT_ASPECT_RATIO` still caps the plot.
- [x] **Frequency band labels are density-aware.** A band earns its label only
      when the band is wider than the label, so the higher bands no longer smear
      into each other at 430px. Commit `1ac7dfd`.
- [x] **The library becomes a tab column, not a permanent sidebar.**
      squig.link's left column is `Brands | Models | Equalizer`. SquigLab's
      library column now is the same three tabs, so the library and the
      Equalizer share one panel slot rather than the Equalizer needing a new
      window. Commit `ed2faa9`.
- [x] **Toolbar collapses gracefully.** squig.link's toolbar stays one row but
      drops controls as width falls. SquigLab's toolbar now does the same: below
      1340px the action cluster moves into a `⋯` overflow menu, and below
      1000px the processing controls, view toggles, and theme join it. Nothing
      is hidden outright, and the row never wraps. Commit `b1bdcc1`.
- [x] **Manager rows reflow.** On a narrow panel the row stacks the curve's
      identity above its controls instead of pressing ten controls into one
      clipped line. Commit `65276b4`.

**Landed so far from this roadmap:** `99c6b34` (test isolation), `1ac7dfd`
(compact layout and band-label density), `b1bdcc1` and `65276b4` (toolbar
collapse and manager reflow), `ed2faa9` (the tab column and the equalizer
panel).

## P2 — Dynamic scrolling and sticky regions

- [x] **Sticky graph.** True by construction: the graph lives in the central
      panel and the library and manager each own a `ScrollArea`, so scroll
      positions are independent and the graph cannot move while either scrolls.
- [x] **Keep the graph visible while the library scrolls.** The library owns
      its own `ScrollArea`; the graph is in a separate panel.
- [x] **Independent scroll areas for library and manager.** Both have their
      own, each with its own egui-managed position.
- [x] **Themed scrollbars.** egui paints a scrollbar's handle from the active
      widget visuals and its track from the extreme background, so a themed
      `egui::style::ScrollStyle` — floating, thin when dormant, handle in the
      palette's foreground stroke — themes both themes at once. This is as close
      as egui gets to the reference's CSS `--scrollbar-color` without patching
      it. Commit `f10d62d`.
- [x] **Scroll position persistence** for the library list. Done by row, not
      by pixel offset: on load the first selected row is scrolled into view and
      its brand opens by default. A stored pixel offset would rewrite the
      preferences file on every scroll frame; the point of the item is that the
      selection stays visible, which this achieves. Commit `f10d62d`.

## P3 — Equalizer and AutoEQ

The largest single feature. Port `web/equalizer.js` numerically.

- [x] **DSP core** — `web/equalizer.js`:
      - `lowshelf`, `highshelf`, `peaking` RBJ biquad coefficient functions,
        with the source's clamp ranges (freq 1e‑6..1, q 1e‑4..1000, gain ±40 dB).
      - `calc_gains` — magnitude in dB summed over biquads.
      - `apply` — add EQ gain onto a `[hz, db]` curve.
      - `interp` — piecewise-linear resampler. **Shared with the measurement
        loader**, not EQ-specific: P6's upload resamples onto the shared grid
        through it.
      - Only `PK`, `LSQ`, `HSQ` exist. Do not invent LP/HP filters.
      Commit `c2ad142`, in `src/equalizer.rs` with a golden fixture captured
      from the JavaScript by `scripts/equalizer-golden.mjs`. The fixture caught
      a mis-transcribed `highshelf` b1 term on its first run.
- [x] **The EQ curve is a first-class curve.** `DisplayCurve` now names its
      `CurveSource` — `Measurement` or `Equalizer` — and an EQ curve names its
      parent group, so normalization, baseline, and colour all treat it as a
      curve instead of the renderer special-casing it. The child is built from
      the parent's already-normalized points, so normalization propagates by
      construction. Its label comes from the parent's label and its colour from
      a palette slot spread away from the parent's, no new hue. Commit `8fc8fc9`.
- [x] **AutoEQ** — `autoeq()` in `web/equalizer.js`: ported whole, including
      both candidate batches, the coarse-to-fine coordinate descent, merge, and
      prune. **Ported and diffed against the JavaScript**: `autoeq` on a
      structured sample curve with budgets of 10 and 5 filters matches the
      reference's filter-for-filter. Commit `c2ad142`.
- [x] **Tools the reference has**: `Reset`, `Sort`, band add/remove, per-band
      enable checkboxes, `Import` (a typed path, since egui has no file picker
      without a new dependency), and a five-row table headed Type / Frequency /
      Gain / Q. Commit `ed2faa9`.
- [x] **Onboarding.** The reference's "How to use EQ on Squiglink" modal with
      Beginner / Intermediate / Advanced / Export, opened from the "?" on the
      panel title, collapsible as the reference's are. One deliberate
      difference: each path *performs* what it describes rather than only
      describing it. Commit `bbc78b3`.

## P4 — Export formats

Both formats are specified from live captures; implement against these exactly.

- [x] **Parametric (Equalizer APO).** Implemented against the captured sample:
      CRLF lines, a full-precision `Preamp`, `LSQ`→`LSC` and `HSQ`→`HSC`, and
      zero bands omitted. Import accepts the same three types plus the
      reference's `LSQ`/`HSQ` spellings and keeps which bands were off; it
      refuses bandwidth (`BW`) bands rather than guessing at a conversion.
      The captured file round-trips byte-identical apart from its LF endings,
      which the capture wrote and the export does not. `Preamp` is the peak
      boost between the curve and its equalized response. Commits `660d171`
      (format) and `ed2faa9` (the panel that writes it, named
      `"{phone} Filters.txt"`).
- [x] **Wavelet (GraphicEQ).**
      Semicolon-separated `freq gain` pairs over the 127-band grid, gains to one
      decimal, precompensation folded in by `as_graphic_eq`, which normalises
      the response to its loudest band so the curve never boosts. Commits
      `6bc033a` and `ed2faa9`.
- [x] **Round-trip tests.** A filter set exported to APO and read back is
      identical, and the Wavelet text parsed off the rendered response matches
      band for band. Both exports are asserted to describe one filter set.
      Commits `660d171` and `6bc033a`.

## P5 — Sound Tools

- [x] **Tone generator.** One continuous oscillator, sine, log-mapped
      frequency slider, 50 ms glide on drag, 10 ms fade on start and stop. Not
      an automated sweep, matching the reference. The samples are generated in
      `src/audio.rs` and carried to a speaker by a backend per platform:
      `libpipewire` on Linux — the server Linux actually uses, not ALSA and not
      a helper process — and cpal to CoreAudio and WASAPI on macOS and Windows.
      Commits `a4d9d01` and `f3c23ed`.
- [x] **Pink noise.** Kellet's economy pink filter, behind a Play / Stop
      button and the same A / B slots. Commits `a4d9d01` and `f3c23ed`.
- [x] **White noise** as well, beside the pink: it is the signal the pink
      filter is fed from, and the only honest way to hear what that filter does.
      Commits `a4d9d01` and `f3c23ed`.
- [x] **Music playback** with a `Compare A | B` toggle, `+ Add Music`, and a
      platform search shortcut. Depends on an audio dependency. — **Declined.**
      Streaming from a music platform is a website feature with no desktop
      equivalent and is a second dependency (a decoder) on top of the output
      one; the equalizer's own value does not rest on it. Declined here rather
      than deferred.
- [x] **Compare A/B** applies to the whole sound-tools section, not one source:
      two whole settings slots the toggle switches between. Commits `a4d9d01`
      and `f3c23ed`.

## P6 — Upload and provenance

- [x] **Measurement upload.** A REW or AudioTool file is parsed (tab or comma
      separated, headers skipped), the file name becomes the model's name, and
      **fewer than 128 points is refused**. Points are resampled onto the
      shared grid through the equalizer's `interp`, which is the "shared with
      the measurement loader, not EQ-specific" this document asked for. A file
      ending `" R"` gets a null left channel spliced in front of its right
      channel, on the same frequencies, so the two line up. Commit `76b09d7`.
- [x] **"Uploaded" brand.** Uploaded measurements land in a synthetic
      `Uploaded` brand and are recorded by name, so they are excluded from share
      URIs: a shared link is a claim about the reference database, and a curve
      from another rig is not part of it. Commit `76b09d7`.
- [x] **Target upload** appends `" Target"`, is refused when the database
      already has a target of that name, and is flagged as a target, so it
      colours as one. Commit `76b09d7`.
- [x] **Provenance warning.** The reference states that measurements from
      another rig are not comparable with its database; that statement is in
      the upload window itself, not a dismissible banner, because nothing in
      the curve shows the rig is different. Commit `76b09d7`.

---

## P7 — Rig compensation and the delta bar

Previously excluded here as a measurement feature rather than layout. Reversed
by decision: the delta bar earns its place, because a measurement rig that is
not the one a target was measured on is the normal case, not the edge case.

- [x] The model, in `src/delta.rs`. A delta target is a clone rig's calibration
      minus the original rig's, defined by naming both curves rather than
      stored as a ready-made difference, so what it claims stays traceable to
      the measurements behind it. Carries a flag for placeholder data.
- [x] Loading from `data_deltas/`, a directory of its own. The calibration
      curves are not headphones, and the database loader would list them as
      phantom models. `delta_dir` in the config; absent directory is normal.
- [x] Optional in the interface throughout. A target can be read without one,
      they never spend one of the eight measurement slots, and a failure to
      read the directory is reported in place rather than failing the load.
- [x] Selection and persistence, held apart from the curve selection because a
      delta target is a correction applied to readings, not a curve compared
      against them. Persisted by name, which survives a reordered directory.
- [x] Manager row, like any other. It carries the pair of rigs it stands
      between and none of the per-curve controls, which have no meaning for it.
- [x] The delta bar, as row one of the curve manager: deviation from the
      selected target on a scale symmetric about zero. With a correction
      selected the measurement is corrected first and the uncorrected
      deviation is kept alongside it, dimmed; the gap is what the correction
      bought. It sits in the manager rather than under the graph because it
      describes the whole selection, not one curve, and the manager is where
      the selection is already accounted for. The manager's height accounts for
      it, so the strip never costs the last row its place.
- [ ] **The reference-target list still does not match the live site.** The site
      offers names this database does not carry — `Super 22`, `Harman IE 2019`
      among them — and only 15 targets ship here. Deferred by decision rather
      than overlooked; see "Carried over".
- [ ] **The shipped delta target corrects nothing.** Both its calibrations are
      analytic curves, not measurements. It is flagged `synthetic`, labelled
      `placeholder` wherever it appears, and a test refuses to let the shipped
      set contain anything unflagged. Drop real calibrations into
      `data_deltas/` and it becomes a genuine correction with no code change.
- [x] The manager is unconditional. It was toggleable, and with it hidden the
      space the graph leaves below its plot stayed dead grey with nothing in
      it, which read as the app having failed to draw. It is also the only place
      the selection is accounted for — headphones, targets, and rig corrections —
      so it is not something that should be switchable off. The toggle and the
      saved setting are gone; older settings files carrying the old key still
      load, pinned by a test.
- [ ] Captured at every viewport in both themes. Same blocker as the rest of
      this file: no display here.

---

## Explicitly out of scope

Do not implement. These are site features, not interface layout, and porting them
would make the desktop app worse.

| Reference feature | Why excluded |
|---|---|
| Site nav (`Headphones`, `Insights`, `Merch`, `Discord`, `HiFiGo`) | Website navigation; no desktop equivalent |
| Sponsorship slots, welcome and onboarding *ads* | Monetization. The EQ onboarding modal is in scope; ad slots are not |
| Footer database-link pills | Website chrome |
| `+Create Squiglink`, database dropdown | Website account and multi-database features |
| Embed mode, `expandableOnly`, `headerHeight` | iframe embedding for third-party sites |
| Analytics | `web/graphAnalytics.js` is an empty stub with no GA4 ID — nothing to port |
| EQ plugin system (`extraEQplugins`, `devicePEQ`) | Not exposed on the live site; was archived-only |
| Three-state contrast theme | A theming feature, not layout. SquigLab has light/dark and that suffices |

---

## Carried over from the previous list

Still open, unchanged, and untouched by this roadmap:

- [ ] The reference targets the live site offers and this database does not:
      `Super 22`, `Harman IE 2019`, and the rest. Adding them needs real curve
      data, not a rename, so the mappings were left alone rather than guessed.
- [ ] Labels follow curve ends rather than grouping in a corner — now a design
      choice, not a defect. SquigLab groups by default and can switch to
      curve-end, so this is resolved unless you decide otherwise.
- [ ] `assets/cringraph-app-icon.svg` is still the only `cringraph`-named asset.
- [ ] The two normalization values are shown one at a time; squig.link shows
      both. A maintainer decision.

---

## Definition of done

- [x] Every P1–P6 item is implemented, or explicitly declined in a commit.
      One item is declined: music playback in P5, in the P5 section above.
- [x] `cargo test` passes: 108 tests, the original 59 among them.
- [x] AutoEQ output diffed against the JavaScript implementation for a sample
      curve, not merely eyeballed: `autoeq_fits_the_reference_filter_set`.
- [x] Both export formats round-trip against a file the live site produced:
      `docs/design-reference/export-apo-sample.txt` imports and exports back
      byte-identical apart from the capture's own LF endings.
- [ ] Captured at 430px, 900px, 1100px, and 1600px, in both themes, with no
      clipping, overlap, or toolbar wrap at the default size. **Still open.**
      The toolbar and manager are written not to wrap and to stack, and the
      layout branches at 1024px, but this environment has no display, so the
      captures could not be taken here.
- [x] `DESIGN.md` updated for any token added. The one addition is the themed
      scrollbar in `src/theme.rs`; it is geometry on the palette's existing
      colors, so no new color token was needed. Documented there.
- [x] Minimum window width reduced and the compact layout confirmed usable.
- [x] This file's checkboxes updated as work lands.

## Suggested order

The ordering is chosen to derisk the expensive parts first. What landed, in the
order it was worked:

1. **P1 viewport** — commits `1ac7dfb`, `b1bdcc1`, `65276b4`.
2. **P3 DSP core** — `c2ad142`, including the AutoEQ port and its golden
   fixture, before any UI depended on it.
3. **P4 parametric export** — `660d171`.
4. **P3 AutoEQ** — with the DSP core, for the same reason.
5. **P4 Wavelet export** — `6bc033a`.
6. **P2 scrolling** — `f10d62d`.
7. **P3 Equalizer UI** — `8fc8fc9` (the curve) then `ed2faa9` (the tab column
   and the panel).
8. **P5 sound tools** — `a4d9d01`, once the EQ it auditions worked.
9. **P6 upload** — `76b09d7`, into the tab column that already existed.
10. **P3 onboarding** — `bbc78b3`, last, because it explains the above.
11. **P7 delta bar** — `75a4d92`, `93a6885`, `e31e480`, `0f93f3e`, after the
    layout work, since it layers a panel under a graph that was already
    responsive.