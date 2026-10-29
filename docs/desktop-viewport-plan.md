# Desktop viewport plan

The reference ships two stylesheets:

- `web/style-alt.css` — the modern ("Super") theme the native currently
  follows for colours and layout.
- `web/style.css` — the older desktop-oriented stylesheet with responsive
  breakpoints (aspect-ratio, max-width, etc.).

## Goal

Provide a desktop viewport that uses `style.css` so the reference can be
inspected at desktop sizes, alongside the mobile/alt viewport.

## Approach (no WebView)

Integrate breakpoints into the existing egui layout, applying the `style.css`
media queries as native layout rules. No WebView is used.

## Proposed phases

1. Inventory the `style.css` breakpoints and map them to Rust layout constants.
2. Decide crate vs. in-app integration.
3. Implement and validate side-by-side at 1440x900 and 1920x1080.

## Rule

No existing crate is removed; the new work is additive.