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

## The port carries its own mathematics

The JavaScript this crate is a port of runs on V8, and V8's `Math.cos`,
`Math.sin`, `Math.exp` and the rest are V8's own copy of fdlibm rather
than the platform's. The two differ: on this machine the platform's `cos`
differs from V8's on about one argument in twenty-five, by one unit in
the last place — and a unit in the last place is enough to change the
last digit of a path.

So the crate carries its own copy in [`ieee754`](src/ieee754.rs),
translated from V8's `src/base/ieee754.cc` by
`scripts/translate_ieee754.py`, and the shapes call it rather than the
platform's own. The fixture under `tests/fixtures/ieee754.json` was
produced by running Node — which is V8 — and writing down the bits of
what it produced, and the test requires the port to give the same bits.

The licence of that translation is in
[`LICENSE.ieee754`](LICENSE.ieee754).

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
| `ieee754` | V8's own mathematics, which d3 runs on |
| `interpolate` | d3-interpolate |
| `js` | the JavaScript's own number semantics |
| `path` | d3-path |
| `polygon` | d3-polygon |
| `quadtree` | d3-quadtree |
| `random` | d3-random |
| `scale` | d3-scale |
| `scale_chromatic` | d3-scale-chromatic |
| `selection` | d3-selection |
| `shape` | d3-shape |
| `time` | d3-time, and the timezone it is read in |
| `time` | d3-time |
| `time_format` | d3-time-format |
| `timer` | d3-timer |
| `transition` | d3-transition |
| `voronoi` | d3-voronoi |
| `zoom` | d3-zoom |

The modules that are written are `array`, `collection`, `color`, `format`, `ieee754`,
`interpolate`, `js`, `number`, `path`, `polygon`, `shape`, `time` and `time_format`;
the rest are named in `src/lib.rs` with a note that they are not yet written, and the
order they are to be written in is the order they depend on each other.

The formats write the numbers with the language's own rules, which are
in [`number`](src/number.rs), for the language rounds a half away from
zero and Rust rounds it to even.

The transform interpolation reads a transform out of a string with a
document, in the JavaScript, and the port asks for it through a trait —
`interpolate::transform::TransformParser` — which a program implements
for whatever it draws with. The matrix's own taking apart is tested
against the JavaScript, for it needs no document.

## The timezone the dates are read in

The `d3-time` package is the JavaScript's own calendar, and its local
intervals — the day, the week, the month and the year — are defined by
the timezone the browser is in: the day of a date is the day it is in
the reader's own town, and not the day it is in Greenwich.

The port reads the timezone in the way the program is built for:

- A program that runs **natively** reads the **operating system's own**
  timezone: the C library on a Unix, and the operating system itself on
  Windows. It is the timezone the browser would have been in.
- A program that runs in a **browser** reads the **browser's own**
  timezone, which the host of the program gives it.

Which of the two is used is settled when the program is built, for the
two are the program's own kind. The crate's `time` module holds the
choice, and the offset it gives is the one the reader would say it in —
the other way about from the JavaScript's `getTimezoneOffset`.

## The commands

```sh
cargo test --offline -p d3      # the port against the JavaScript
cargo clippy --offline -p d3 --all-targets
cargo fmt --all
```
