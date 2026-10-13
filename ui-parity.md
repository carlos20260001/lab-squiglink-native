# UI Parity: SquigLab ↔ squig.link

A task list for closing the visual and layout gap between the native app and the
reference web tool it was ported from.

**Sources of truth**

- `docs/design-reference/native-current.png` — SquigLab at 1600x1100, three
  measurements and one target selected.
- `docs/design-reference/squig-desktop.png` / `squig-dark.png` — squig.link at
  1600x1100, same selection shape.

Regenerate the native capture before acting on this list, so it describes the
current build and not an old one:

```bash
Xvfb :99 -screen 0 1600x1100x24 &
DISPLAY=:99 env -u WAYLAND_DISPLAY LIBGL_ALWAYS_SOFTWARE=1 WGPU_BACKEND=gl \
  cargo run &
sleep 15
DISPLAY=:99 xdotool search --name SquigLab | head -1 | \
  xargs -I{} env DISPLAY=:99 xdotool windowsize {} 1600 1100
sleep 3
DISPLAY=:99 ffmpeg -f x11grab -video_size 1600x1100 -i :99+0,0 -frames:v 1 -y \
  docs/design-reference/native-current.png
```

## Rules for this work

1. **Parity is about layout and affordance, not copying website chrome.** The site
   nav bar, sponsor slots, `DELTA`, footer database links, and `+Create Squiglink`
   are site features. See "Out of scope" at the bottom — do not implement them.
2. **Behaviour before pixel-nudging.** A missing control the reference has is a
   bigger parity gap than a 2px offset. Fix the items in order.
3. **One concern per commit**, in this file's granularity. The repository's
   convention is atomic commits with a terse imperative summary and a prose body
   wrapped near 78 columns.
4. **Every UI change resolves to a token** in `src/theme.rs`. Do not introduce raw
   hex values or ad-hoc font sizes outside that module.
5. **Verify visually before committing.** Capture both themes, and check at
   880px wide (the window's minimum) where layout regressions show up first.

---

## P0 — Broken in the current build

These are defects visible in `native-current.png`, not preferences.

- [ ] **Curve labels clip off the right edge of the plot.**
      With three curves selected, `GAR5 · GarG5 · R` and `AA01 · AA01 · R` are
      cut off mid-word at the right margin. squig.link keeps every label fully
      inside the plot by nudging it left when the curve ends near the edge.
      *Fix:* in `graph.rs`, the label placement already clamps x to
      `plot_rect.right() - size.x`; it is not accounting for labels wider than
      the plot or for the label anchor. Verify with long names such as
      "Softears Karma (L)".

- [ ] **Curve labels duplicate the brand and model.**
      Labels read `GAR5 · GarG5 · R` and `AA01 · AA01 · R` — the model name is
      repeated. squig.link shows `Softears Karma (L)` / `Softears Karma (R)`.
      *Fix:* in `app.rs`, the label is composed as
      `format!("{} · {} · {}", group.name, variant.name, channel_name)`. Emit
      `"{variant or model} ({channel})"` and only include the brand when the
      variant name does not already contain it.

- [ ] **Target labels sit far from the curve and collide with the bottom.**
      `Harman · Target` is drawn at the bottom-right corner, detached from the
      dashed target curve, and below the last curve label. squig.link keeps
      target labels stacked with the other labels in the same corner block.
      *Fix:* route targets through the same label placement as measurements;
      they already go through `label_candidates`.

- [ ] **A dashed target curve is drawn far outside the visible dB range.**
      In the capture the Harman target runs from ~70 dB down off the bottom of
      the plot, forcing the y-axis to stretch to 10 dB and squashing every
      measurement curve into the top fifth of the graph. squig.link clips target
      curves to the plotted range.
      *Fix:* this is a normalisation bug, not a styling one. Check how target
      curves are normalized in `app.rs` when a target is selected — the target is
      being offset rather than normalized to the reference, dragging the y-range.
      This one change will make the graph look dramatically closer to the
      reference.

---

## P1 — Present on squig.link, missing in SquigLab

Ordered by how much they change the impression of the app.

- [ ] **Normalize shows both values at once.** squig.link shows
      `NORMALIZE: [dB][60] [Hz][500]` — both the phon and Hz fields side by side.
      SquigLab shows only the active mode's field, having made them mutually
      exclusive at some earlier point.
      *Fix:* render both fields and highlight the active mode. Reverts a prior
      judgement call, so confirm with the maintainer first.

- [ ] **Zoom has no explicit "Full".** squig.link offers only
      `[Bass][Mids][Treble]`; the full range is the implicit default and is
      reached by not zooming. SquigLab has a fourth `Full` button.
      *Fix:* drop `Full` from the segmented control; treat "no band selected" as
      full range. Needs a state change, since `ZoomRegion::Full` is currently a
      variant.

- [ ] **Cluster labels are missing from the toolbar.** squig.link prefixes each
      group with a small uppercase label: `NORMALIZE:`, `SMOOTH:`, `ZOOM:`.
      SquigLab dropped these when compacting the toolbar to fit one row.
      *Fix:* restore them via the existing `cgui::cluster_label`, and reclaim the
      width by shortening the action labels (`Recolor` → `Colour`, `Share` →
      `Link`) if the row overflows at 1440px.

- [ ] **The frequency guide is a modal window, not a toggle.**
      squig.link puts the frequency tutorial permanently under the graph as
      labelled bands (already ported) and opens the guide in place. SquigLab
      opens a separate floating window from the `Guide` button.
      *Fix:* render the guide in the central panel rather than a popup window.

- [ ] **Copy is named "Copy URL", not "Share".** SquigLab's button opens a window
      with a multiline text box and a "Restore pasted state" field.
      *Fix:* make `Share` copy the URI to the clipboard directly, and move
      paste-to-restore behind a small secondary control.

---

## P2 — Presentational differences

Lower impact, but they are what makes the two screens look like one product.

- [ ] **No watermark in the plot.** squig.link draws a large, very faint logo
      behind the curves plus a `squig.link/` wordmark at the lower right.
      *Fix:* draw the logo from `assets/` at roughly 6% opacity, centred in the
      plot rect. Do not replicate the web logo specifically — use SquigLab's.

- [ ] **Labels are not grouped in a corner.** squig.link stacks up to five
      labels together in one corner (bottom-left by default) so they read as a
      legend. SquigLab places each label at the end of its own curve, which
      scatters them and causes the clipping in P0.
      *Fix:* this also solves the clipping problem. Port the `labelsPosition`
      corner-grouping behaviour (`top-left`, `bottom-left`, `bottom-right`) with
      collision avoidance, keeping free placement when the user drags a label.

- [ ] **Targets are grey; measurements take the curve colour.** squig.link
      renders target labels in muted grey so they visually recede from the
      measurements. SquigLab gives every curve a saturated palette colour.
      *Fix:* target curves already use `color::target_color()`; use the same
      muted tone for their labels.

- [ ] **No channel thumbnail in the manager row.** squig.link shows a small
      waveform swatch per row that doubles as the channel key, plus
      `L ONLY` / `R ONLY` text for multichannel curves.
      *Fix:* draw a miniature curve thumbnail from the already-loaded points in
      the manager row. This is the single most "native-feeling vs web-feeling"
      difference left in the manager.

- [ ] **Manager icons instead of checkboxes.** squig.link uses a row of compact
      icon buttons: channel key, hide (eye), pin, download, remove (×).
      SquigLab uses text checkboxes for Baseline/Pin/Hide.
      *Fix:* keep the labels — the web's icons are less legible and `egui` has no
      icon font — but present them as a tighter, aligned cluster so the row reads
      as a set of toggles rather than a sentence.

- [ ] **No brand colour strip in the library.** squig.link draws a thin coloured
      rule down the left edge of each brand row.
      *Fix:* assign each brand a stable colour from the existing palette
      (`color::get_curve_color_with_offset`) keyed by name, so the same brand is
      always the same colour across sessions.

---

## P3 — Density and fit

- [ ] **The graph gives up 54px at the bottom** for the band row even when band
      labels are not shown. Reserve the space only when bands are visible.
- [ ] **The toolbar wraps at 1440px** once the P1 cluster labels come back.
      Confirm at the 880px minimum and 1600px that it stays one row, and accept
      two rows below 1280px if necessary.
- [ ] **The inspector readout card can overlap the curve labels** at the top
      left of the plot. Give it a fixed corner and let labels avoid it.

---

## Out of scope — do not implement

These appear in the reference screenshot but are website features, not app
layout. Porting them would make the desktop app worse.

| Reference feature | Why it is out of scope |
|---|---|
| Site nav bar (`Headphones`, `Earbuds`, `Insights`, `Merch`, `Discord`) | Website navigation; no desktop equivalent |
| Sponsor creative and the welcome modal | Monetization; already excluded by the maintainer |
| `DELTA` bar with reference-target chips | A measurement feature (averaged delta curves), not layout |
| Footer database link pills | Website chrome |
| `+Create Squiglink` | Website upload flow |
| Database dropdown (`Super* Review`) | Multi-database switching; needs a data-model change |
| `Brands` / `Models` / `Equalizer` tabs | The library sidebar and the EQ feature; the EQ is not ported yet |

---

## Definition of done

- [ ] Every P0 and P1 item is either implemented or explicitly declined in a
      commit message.
- [ ] `cargo test` passes and no test was weakened to accommodate a change.
- [ ] Native capture regenerated in both themes at 1600px and at the 880px
      minimum; no clipping, overlap, or toolbar wrapping at the default size.
- [ ] `DESIGN.md` updated if any token was added or changed.
- [ ] This file's checkboxes are updated as work lands.