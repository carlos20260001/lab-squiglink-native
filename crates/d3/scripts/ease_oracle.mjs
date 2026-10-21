// The oracle: d3-ease's own JavaScript, run under Node, writing what it
// produces.
//
// Every fixture in the port's `tests/fixtures` is produced by this script, so
// that the Rust is checked against the JavaScript it was translated from
// rather than against an expectation written by hand.
//
//   node scripts/ease_oracle.mjs > tests/fixtures/ease.json

import {writeFileSync} from "fs";
import {createRequire} from "module";

const require = createRequire(import.meta.url);
const root = "/tmp/opencode/d3src/node_modules";

const ease = require(root + "/d3-ease/src/index.js");

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

// The points the eases are drawn at: the two ends of the range they are
// meant for, the middle, the outside of it, and the numbers that are not
// numbers.
const points = [
  0, -0, 1, -1, 0.5, -0.5, 0.1, 0.25, 0.75, 0.9,
  1.5, 2, -2, 1 / 3, 2 / 3, 1e-10, 1e10,
  Math.PI, Math.sqrt(2), NaN, Infinity, -Infinity,
];

// The values of an ease, at the points.
function draw(ease_) {
  return points.map((t) => bits(ease_(t)));
}

const out = {
  points: points.map(bits),
  eases: {},
  poly: {},
  back: {},
  elastic: {},
  setters: {elastic: {}},
};

// The exports of the package, which are thirty eases.
for (const name of Object.keys(ease).sort()) {
  out.eases[name] = draw(ease[name]);
}

// The polys, which have an exponent. The JavaScript's own setter is used
// to give it, because that is the way a caller gives it.
for (const name of ["easePolyIn", "easePolyOut", "easePolyInOut"]) {
  out.poly[name] = {};
  for (const exponent of [0, 1, 2, 3, 4, 0.5, -1, -2.5]) {
    out.poly[name][String(exponent)] = draw(ease[name].exponent(exponent));
  }
}

// The backs, which have an overshoot.
for (const name of ["easeBackIn", "easeBackOut", "easeBackInOut"]) {
  out.back[name] = {};
  for (const overshoot of [0, 1, 1.70158, 2, 10]) {
    out.back[name][String(overshoot)] = draw(ease[name].overshoot(overshoot));
  }
}

// The elastic ones, which have an amplitude and a period. The JavaScript's
// own setters are used to give them, because that is the way a caller
// gives them.
for (const name of ["easeElasticIn", "easeElasticOut", "easeElasticInOut"]) {
  out.elastic[name] = {};
  for (const [amplitude, period] of [
    [1, 0.3], [2, 0.3], [1, 0.5], [5, 1], [1, 1], [0.5, 0.3],
  ]) {
    out.elastic[name][`${amplitude}/${period}`] =
      draw(ease[name].amplitude(amplitude).period(period));
  }
}

// The setters of the elastic ones, each of which keeps the other setting.
for (const name of ["easeElasticIn", "easeElasticOut", "easeElasticInOut"]) {
  out.setters.elastic[name] = {amplitude: {}, period: {}};
  for (const amplitude of [2, 5]) {
    out.setters.elastic[name].amplitude[String(amplitude)] =
      draw(ease[name].amplitude(amplitude));
  }
  for (const period of [0.5, 1]) {
    out.setters.elastic[name].period[String(period)] =
      draw(ease[name].period(period));
  }
}

writeFileSync(
  process.argv[2] || "tests/fixtures/ease.json",
  JSON.stringify(out, null, 1) + "\n");
