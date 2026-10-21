# d3

A faithful port of [d3](https://d3js.org), the JavaScript data visualisation
library, to Rust.

This is a translation of Mike Bostock's d3, version **5.16.0** — the version
the web reference loads from `https://d3js.org/d3.v5.min.js`. The JavaScript
it was translated from is vendored under `reference/`, and that JavaScript is
the authority: where a reading of it and a reading of this crate could
differ, the JavaScript is right.

Nothing is dropped. Every package of d3 is here, with its whole surface, so
that this crate can stand in for d3 in any project that uses it — including
the parts that need a document, which are written against a trait
(`d3::dom`) that a project implements for whatever it draws with.

## The licence

d3 is BSD-3-Clause. That licence permits the port, and requires that the
copyright notice, the list of conditions and the disclaimer travel with it.
They are in [`LICENSE`](LICENSE), and the notices of the individual packages
are in [`LICENSE.d3-packages`](LICENSE.d3-packages).

The vendored JavaScript is the same code under the same licence, and each
package's own `LICENSE` is beside it under `reference/`.

## How the port is kept faithful

Every module is checked against the JavaScript it was translated from. The
fixtures under `tests/fixtures/` were produced by running the real d3 under
Node — the same bundles the browser runs — and writing down what it
produced; the tests then require the Rust to give the same answers.

The oracle that produces them is `scripts/d3_oracle.mjs`. It is a
development tool, not part of the build: the fixtures are committed, so
`cargo test` needs neither Node nor a network.

The vendored JavaScript is checked against `reference/MANIFEST.json`, which
holds the SHA-256 of every file and the integrity hash of the npm package
it came from. The npm `d3@5.16.0` bundle is byte for byte the
`d3.v5.min.js` that the reference loads, which is the strongest check
available that the two are the same code.

## The shape of the port

The JavaScript's names are kept: a scale is a scale, a curve is a curve, a
path is a path. Where the JavaScript's functions take a list and often a
function that reads a value out of a datum, the port takes a slice and a
closure, and the two are told apart by their names — a function without a
`_by` suffix reads the datum itself, and one with the suffix takes the
reader.

Where the JavaScript returns `undefined` — an empty list, or a list with
nothing comparable in it — the port returns `None`. Where the JavaScript
returns `NaN`, the port returns `f64::NAN`.

The JavaScript's number semantics are in [`js`](src/js.rs), because the
JavaScript's own results depend on them.

## The modules

| Module | The package |
|---|---|
| `array` | d3-array |
| `axis` | d3-axis |
| `brush` | d3-brush |
| `chord` | d3-chord |
| `collection` | d3-collection |
| `color` | d3-color |
| `contour` | d3-contour |
| `dispatch` | d3-dispatch |
| `drag` | d3-drag |
| `dsv` | d3-dsv |
| `ease` | d3-ease |
| `fetch` | d3-fetch |
| `force` | d3-force |
| `format` | d3-format |
| `geo` | d3-geo |
| `hierarchy` | d3-hierarchy |
| `interpolate` | d3-interpolate |
| `path` | d3-path |
| `polygon` | d3-polygon |
| `quadtree` | d3-quadtree |
| `random` | d3-random |
| `scale` | d3-scale |
| `scale_chromatic` | d3-scale-chromatic |
| `selection` | d3-selection |
| `shape` | d3-shape |
| `time` | d3-time |
| `time_format` | d3-time-format |
| `timer` | d3-timer |
| `transition` | d3-transition |
| `voronoi` | d3-voronoi |
| `zoom` | d3-zoom |

## The commands

```sh
cargo test --offline -p d3      # the port against the JavaScript
cargo clippy --offline -p d3 --all-targets
cargo fmt --all
```
