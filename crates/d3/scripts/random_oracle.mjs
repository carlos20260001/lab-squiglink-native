// The oracle: d3-random's own JavaScript, run under Node, writing what it
// produces.
//
// Every fixture in the port's `tests/fixtures` is produced by this script, so
// that the Rust is checked against the JavaScript it was translated from
// rather than against an expectation written by hand.
//
//   node scripts/random_oracle.mjs > tests/fixtures/random.json
//
// The JavaScript is d3-random 1.1.2, the release d3 5.16.0 pins, and the
// copy under `reference/d3-random/src` is the same code: the package's own
// bundle is used here, because its sources name their neighbours without
// the `.js` extension and Node cannot load them as they are.
//
// The package is installed with:
//
//   npm install d3-random@1.1.2 --prefix /tmp/opencode/d3src

import {writeFileSync} from "fs";
import {createRequire} from "module";

const require = createRequire(import.meta.url);
const root = "/tmp/opencode/d3src/node_modules";

const d3random = require(root + "/d3-random/dist/d3-random.js");

// The bits of a number, for a number written in text and read back may not
// be the same number.
const buffer = new ArrayBuffer(8);
const view = new DataView(buffer);

function bits(value) {
  view.setFloat64(0, value, false);
  let text = "";
  for (let index = 0; index < 8; index += 1) {
    text += view.getUint8(index).toString(16).padStart(2, "0");
  }
  return text;
}

// A source of the oracle's own, so that the numbers it writes down are the
// same every time: a linear congruential generator of the kind the
// Numerical Recipes gives.
//
// The seed is written down with the numbers, so that the Rust can give the
// source the same seed and draw the same stream.
function lcg(seed) {
  let state = seed >>> 0;
  return () => {
    state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
    return state / 4294967296;
  };
}

// A source that gives the numbers it was given, one after another, and the
// last of them for ever after that.
function fixed(values) {
  let index = 0;
  return () => {
    const value = values[Math.min(index, values.length - 1)];
    index += 1;
    return value;
  };
}

// The numbers a generator gives, as their bits.
function draw(generator, count) {
  const out = [];
  for (let index = 0; index < count; index += 1) out.push(bits(generator()));
  return out;
}

// How many numbers are drawn of each kind of generator. The normal one
// keeps half of a pair of numbers between its calls, so an even number of
// them shows the keeping.
const COUNT = 6;

// The cases of each export. Every case is made with a source of its own, so
// that the cases do not lean on each other.
const out = {
  uniform: {},
  normal: {},
  logNormal: {},
  exponential: {},
  bates: {},
  irwinHall: {},
  defaultSource: {},
};

// The seed of each case, so that no two of them share a stream.
let seed = 1;

function nextSeed() {
  seed += 1;
  return seed;
}

// A case of an export, drawn from a source of the oracle's own.
function caseOf(group, name, make, count) {
  const seed = nextSeed();
  const generator = make(lcg(seed));
  out[group][name] = {
    seed,
    values: draw(generator, count || COUNT),
  };
}

// A case of an export, drawn from a source that gives the numbers it was
// given.
function fixedCase(group, name, make, values, count) {
  const generator = make(fixed(values));
  out[group][name] = {
    fixed: values.map(bits),
    values: draw(generator, count || COUNT),
  };
}

// --- randomUniform --------------------------------------------------------
//
// The JavaScript's `randomUniform` reads how many arguments it was given,
// and the cases below are the spellings of that: none of them, one of them,
// two of them, and two of them with one of them `null` or not there at all.
caseOf("uniform", "noArgs", (source) => d3random.randomUniform.source(source)());
caseOf("uniform", "oneArg", (source) => d3random.randomUniform.source(source)(5));
caseOf("uniform", "twoArgs", (source) => d3random.randomUniform.source(source)(2, 5));
caseOf("uniform", "nullArg", (source) => d3random.randomUniform.source(source)(null));
caseOf("uniform", "undefinedArg", (source) => d3random.randomUniform.source(source)(undefined));
caseOf("uniform", "nullMin", (source) => d3random.randomUniform.source(source)(null, 5));
caseOf("uniform", "undefinedMax", (source) => d3random.randomUniform.source(source)(2, undefined));
caseOf("uniform", "negative", (source) => d3random.randomUniform.source(source)(-3, -1));
caseOf("uniform", "zeroWidth", (source) => d3random.randomUniform.source(source)(2, 2));
caseOf("uniform", "maxBelowMin", (source) => d3random.randomUniform.source(source)(5, 2));
caseOf("uniform", "extraArgs", (source) => d3random.randomUniform.source(source)(1, 2, 3));

// --- randomNormal ---------------------------------------------------------
caseOf("normal", "noArgs", (source) => d3random.randomNormal.source(source)());
caseOf("normal", "mu", (source) => d3random.randomNormal.source(source)(5));
caseOf("normal", "sigma", (source) => d3random.randomNormal.source(source)(undefined, 2));
caseOf("normal", "both", (source) => d3random.randomNormal.source(source)(3, 2));
caseOf("normal", "negativeMu", (source) => d3random.randomNormal.source(source)(-3, 0.5));
caseOf("normal", "nulls", (source) => d3random.randomNormal.source(source)(null, null));

// The normal generator draws a pair of numbers and throws the pair away if
// the circle they are in is not the unit one, and it keeps the first of the
// pair for the next number. The source below gives it a pair at the centre
// of the circle, a pair outside it, and a pair inside it.
fixedCase(
  "normal",
  "fixedSource",
  (source) => d3random.randomNormal.source(source)(),
  [0.5, 0.5, 1, 0, 0.25, 0.75],
  4,
);

// --- randomLogNormal ------------------------------------------------------
caseOf("logNormal", "noArgs", (source) => d3random.randomLogNormal.source(source)());
caseOf("logNormal", "mu", (source) => d3random.randomLogNormal.source(source)(5));
caseOf("logNormal", "sigma", (source) => d3random.randomLogNormal.source(source)(undefined, 2));
caseOf("logNormal", "both", (source) => d3random.randomLogNormal.source(source)(3, 2));
caseOf("logNormal", "negativeMu", (source) => d3random.randomLogNormal.source(source)(-3, 0.5));

// --- randomExponential ----------------------------------------------------
caseOf("exponential", "one", (source) => d3random.randomExponential.source(source)(1));
caseOf("exponential", "two", (source) => d3random.randomExponential.source(source)(2));
caseOf("exponential", "half", (source) => d3random.randomExponential.source(source)(0.5));
caseOf("exponential", "zero", (source) => d3random.randomExponential.source(source)(0));
caseOf("exponential", "negative", (source) => d3random.randomExponential.source(source)(-1));
caseOf("exponential", "null", (source) => d3random.randomExponential.source(source)(null));
caseOf("exponential", "undefined", (source) => d3random.randomExponential.source(source)(undefined));

// A source that gives one, which is the end of the range the logarithm is
// taken of, and a source that gives a number that is not a number.
fixedCase(
  "exponential",
  "fixedSource",
  (source) => d3random.randomExponential.source(source)(1),
  [1, 0.5, NaN],
  3,
);

// --- randomBates and randomIrwinHall --------------------------------------
for (const n of [1, 2, 3, 5]) {
  caseOf("bates", String(n), (source) => d3random.randomBates.source(source)(n));
  caseOf("irwinHall", String(n), (source) => d3random.randomIrwinHall.source(source)(n));
}
caseOf("bates", "zero", (source) => d3random.randomBates.source(source)(0));
caseOf("bates", "negative", (source) => d3random.randomBates.source(source)(-1));
caseOf("bates", "fraction", (source) => d3random.randomBates.source(source)(2.5));
caseOf("bates", "null", (source) => d3random.randomBates.source(source)(null));
caseOf("bates", "undefined", (source) => d3random.randomBates.source(source)(undefined));
caseOf("irwinHall", "zero", (source) => d3random.randomIrwinHall.source(source)(0));
caseOf("irwinHall", "negative", (source) => d3random.randomIrwinHall.source(source)(-1));
caseOf("irwinHall", "fraction", (source) => d3random.randomIrwinHall.source(source)(2.5));
caseOf("irwinHall", "null", (source) => d3random.randomIrwinHall.source(source)(null));
caseOf("irwinHall", "undefined", (source) => d3random.randomIrwinHall.source(source)(undefined));

// --- the default source ----------------------------------------------------
//
// The JavaScript's default source is `Math.random`, which the specification
// does not give: it is a number in [0, 1), and the stream behind it is the
// engine's own. What can be written down is the range of it, and that is
// what is written down.
{
  const generator = d3random.randomUniform();
  let inRange = 0;
  const count = 1000;
  for (let index = 0; index < count; index += 1) {
    const value = generator();
    if (value >= 0 && value < 1) inRange += 1;
  }
  out.defaultSource = {count, inRange};
}

writeFileSync(
  process.argv[2] || "tests/fixtures/random.json",
  JSON.stringify(out, null, 1) + "\n");
