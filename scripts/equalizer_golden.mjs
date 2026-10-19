// Regenerates crates/squiglab-core/src/equalizer_golden.json, the fixture
// the equalizer port is tested against.
//
// The fixture pins the Rust port in src/equalizer.rs against this repository's
// copy of the reference implementation, web/equalizer.js, which is the source
// of truth for the filter maths. Run it from anywhere with:
//
//     node scripts/equalizer_golden.mjs
//
// and commit the regenerated file. Edit the reference first if the numbers look
// wrong: the fixture is captured from it, never authored by hand.
//
// The sample curves are synthetic but structured — a sub-bass shelf, a
// mid-bass dip, a presence peak, and a treble rolloff against a smoothed
// preference target — because a flat or noise-shaped response would let a
// wrong port pass by trivially matching. The grids and curves are written into
// the fixture so the Rust tests replay exactly these inputs.

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const repositoryRoot = join(scriptDirectory, "..");

const referenceSource = readFileSync(
    join(repositoryRoot, "web", "equalizer.js"),
    "utf8",
);
// The reference assigns to an implicit global inside an IIFE. Running it as a
// function body keeps that assignment sloppy-mode, so it lands on globalThis.
const Equalizer = new Function(`${referenceSource}; return Equalizer;`)();

// A 1/6-octave grid, dense enough for AutoEQ to find real structure.
const GRID_POINTS = 121;
const grid = Array.from(
    { length: GRID_POINTS },
    (_, index) => 20 * Math.pow(20000 / 20, index / (GRID_POINTS - 1)),
);

const gaussian = (frequency, center, width) =>
    Math.exp(-Math.pow(Math.log(frequency / center) / width, 2));

const curveDb = (frequency) =>
    8 * gaussian(frequency, 90, 1.1) -
    5 * gaussian(frequency, 320, 0.5) +
    4 * gaussian(frequency, 2800, 0.45) -
    9 * gaussian(frequency, 8000, 1.4) +
    1.5 * Math.sin(3 * Math.log(frequency));

const targetDb = (frequency) =>
    9 * gaussian(frequency, 70, 1.3) -
    2 * gaussian(frequency, 400, 0.6) +
    5 * gaussian(frequency, 3500, 0.5) -
    6 * gaussian(frequency, 9000, 1.5) +
    2 * Math.sin(2 * Math.log(frequency));

const curve = grid.map((frequency) => [frequency, curveDb(frequency)]);
const target = grid.map((frequency) => [frequency, targetDb(frequency)]);

// One filter of each type, with the peaking filter taken from the captured
// Equalizer APO sample in docs/design-reference.
const filters = [
    { type: "PK", freq: 500, q: 1.2, gain: -3.5 },
    { type: "LSQ", freq: 105, q: 0.71, gain: 6 },
    { type: "HSQ", freq: 9000, q: 0.71, gain: -4 },
];

// filters_to_coeffs is private in the reference; rebuild it from the exported
// coefficient functions, dropping inert filters the same way.
const coefficients = (set) =>
    set
        .filter((f) => f.freq && f.gain && f.q)
        .map((f) => {
            if (f.type === "LSQ") return Equalizer.lowshelf(f.freq, f.q, f.gain);
            if (f.type === "HSQ") return Equalizer.highshelf(f.freq, f.q, f.gain);
            return Equalizer.peaking(f.freq, f.q, f.gain);
        });

const interpCurve = [
    [20, 0.5],
    [100, -1.25],
    [1000, 2.75],
    [10000, -4.5],
    [20000, -6.0],
];

// The reference does not export calc_distance, so this is a verbatim
// transcription of it from web/equalizer.js: the golden value still comes from
// the original algorithm rather than from this port.
const calcDistance = (fr1, fr2) => {
    let distance = 0;
    for (let i = 0; i < fr1.length; ++i) {
        const d = Math.abs(fr1[i][1] - fr2[i][1]);
        distance += d >= 0.1 ? d : 0;
    }
    return distance / fr1.length;
};

const equalized = Equalizer.apply(curve, filters);

const fixture = {
    description:
        "Captured from web/equalizer.js by scripts/equalizer_golden.mjs. The Rust port in src/equalizer.rs is tested against these values.",
    sampleRate: Equalizer.config.DefaultSampleRate,
    curve,
    target,
    filters,
    lowshelf: Equalizer.lowshelf(105, 0.71, 6),
    highshelf: Equalizer.highshelf(9000, 0.71, -4),
    peaking: Equalizer.peaking(500, 1.2, -3.5),
    gains: Equalizer.calc_gains(
        grid,
        coefficients(filters),
    ),
    apply: equalized,
    interp: {
        frequencies: [20, 50, 120, 317.227766, 900, 2500, 8000, 19999],
        curve: interpCurve,
        result: Equalizer.interp(
            [20, 50, 120, 317.227766, 900, 2500, 8000, 19999],
            interpCurve,
        ),
    },
    calcPreamp: Equalizer.calc_preamp(curve, equalized),
    calcDistance: calcDistance(curve, target),
    asGraphicEq: Equalizer.as_graphic_eq(filters),
    autoeq: {
        maxFilters: 10,
        filters: Equalizer.autoeq(curve, target, 10),
    },
    autoeqSmall: {
        maxFilters: 5,
        filters: Equalizer.autoeq(curve, target, 5),
    },
};

const output = join(
    repositoryRoot,
    "crates",
    "squiglab-core",
    "src",
    "equalizer_golden.json",
);
writeFileSync(output, `${JSON.stringify(fixture, null, 2)}\n`);
console.log(`Wrote ${output}`);
