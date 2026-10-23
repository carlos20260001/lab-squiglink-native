// The oracle: d3-interpolate's own JavaScript, run under Node, writing
// what it produces.
//
// Every fixture in the port's `tests/fixtures` is produced by this script, so
// that the Rust is checked against the JavaScript it was translated from
// rather than against an expectation written by hand.
//
//   node scripts/interpolate_oracle.mjs > tests/fixtures/interpolate.json

import {writeFileSync} from "fs";
import {createRequire} from "module";

const require = createRequire(import.meta.url);
const root = "/tmp/opencode/d3src/node_modules";

function d3(pkg) {
  return require(root + "/" + pkg + "/src/index.js");
}

const d3interpolate = d3("d3-interpolate");

// The bits of a number, for a number written in text and read back may
// not be the same number.
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

const out = {};

// The numbers, at a few points of the way between.
const points = [0, 0.1, 0.25, 0.5, 0.75, 0.9, 1];

// The numbers.
out.number = {};
for (const a of [0, 1, -1, 2.5, 100]) {
  for (const b of [0, 1, 10, -3.5]) {
    const i = d3interpolate.interpolateNumber(a, b);
    out.number[`${a}/${b}`] = points.map((t) => i(t));
  }
}

// The numbers with a number that is not a number in them.
out.numberNaN = {};
for (const [a, b] of [[NaN, NaN], [NaN, 1], [1, NaN], [NaN, Infinity], [Infinity, NaN]]) {
  const i = d3interpolate.interpolateNumber(a, b);
  out.numberNaN[`${a}/${b}`] = bits(i(0.5));
}

// The hue with a number that is not a number in it.
out.hueNaN = {};
for (const [a, b] of [[NaN, 120], [120, NaN], [NaN, NaN]]) {
  const i = d3interpolate.interpolateHue(a, b);
  out.hueNaN[`${a}/${b}`] = bits(i(0.5));
}

// The numbers, rounded.
out.round = {};
for (const a of [0, 1, -1, 2.5, 100]) {
  for (const b of [0, 1, 10, -3.5]) {
    const i = d3interpolate.interpolateRound(a, b);
    out.round[`${a}/${b}`] = points.map((t) => i(t));
  }
}

// The dates.
out.date = {};
for (const a of [0, 1000, 1600000000000]) {
  for (const b of [0, 500, 1700000000000]) {
    const i = d3interpolate.interpolateDate(a, b);
    // The JavaScript's date is one date, which it changes and gives
    // back, so the oracle takes the time of it at each point.
    out.date[`${a}/${b}`] = points.map((t) => +i(t));
  }
}

// The lists of numbers.
out.numberArray = {};
for (const [a, b] of [[[0, 10], [10, 20]], [[1, 2, 3], [4, 5, 6]], [[], [1, 2]], [[1, 2], []]]) {
  const i = d3interpolate.interpolateNumberArray(a, b);
  // The JavaScript's list is one list, which it changes and gives back,
  // so the oracle takes a copy of it at each point.
  out.numberArray[`${a}/${b}`] =
    points.map((t) => JSON.parse(JSON.stringify(i(t))));
}

// The lists of anything.
out.array = {};
for (const [a, b] of [[[0, 10], [10, 20]], [["a", "b"], ["c", "d"]], [[0], [1, 2]]]) {
  const i = d3interpolate.interpolateArray(a, b);
  // The JavaScript's array is one array, which it changes and gives
  // back, so the oracle takes a copy of it at each point.
  out.array[`${a}/${b}`] = points.map((t) => JSON.parse(JSON.stringify(i(t))));
}

// The objects.
out.object = {};
for (const [a, b] of [
  [{x: 0, y: 10}, {x: 10, y: 20}],
  [{x: 0}, {x: 10, z: 5}],
  [{x: 0, y: 1}, {x: 1, y: 0}],
]) {
  const i = d3interpolate.interpolateObject(a, b);
  // The JavaScript's object is one object, which it changes and gives
  // back, so the oracle takes a copy of it at each point.
  out.object[`${JSON.stringify(a)}/${JSON.stringify(b)}`] =
    points.map((t) => JSON.parse(JSON.stringify(i(t))));
}

// The strings, which are drawn between the numbers in them.
out.string = {};
for (const [a, b] of [
  ["0", "10"],
  ["0px", "10px"],
  ["translate(0,0)", "translate(10,20)"],
  ["a1b2c", "a3b4c"],
  ["rgb(0,0,0)", "rgb(255,255,255)"],
  ["hello", "world"],
  ["1e3", "2e3"],
  ["-1.5", "+2.25"],
  ["0%", "100%"],
  ["", "5"],
]) {
  const i = d3interpolate.interpolateString(a, b);
  out.string[`${a}/${b}`] = points.map((t) => i(t));
}

// The colours.
out.rgb = {};
for (const [a, b] of [
  ["red", "blue"],
  ["#ff0000", "#0000ff"],
  ["rgb(255, 0, 0)", "rgba(0, 0, 255, 0.5)"],
  ["hsl(0, 100%, 50%)", "hsl(240, 100%, 50%)"],
]) {
  const i = d3interpolate.interpolateRgb(a, b);
  out.rgb[`${a}/${b}`] = points.map((t) => i(t));
}

// The colours with a gamma.
out.rgbGamma = {};
for (const [a, b] of [["red", "blue"], ["#ff0000", "#00ff00"]]) {
  const i = d3interpolate.interpolateRgb.gamma(2.2)(a, b);
  out.rgbGamma[`${a}/${b}`] = points.map((t) => i(t));
}

// The other colour spaces.
out.colours = {};
for (const [name, function_] of Object.entries({
  hsl: d3interpolate.interpolateHsl,
  hslLong: d3interpolate.interpolateHslLong,
  lab: d3interpolate.interpolateLab,
  hcl: d3interpolate.interpolateHcl,
  hclLong: d3interpolate.interpolateHclLong,
  cubehelix: d3interpolate.interpolateCubehelix,
  cubehelixLong: d3interpolate.interpolateCubehelixLong,
})) {
  for (const [a, b] of [
    ["red", "blue"],
    ["#ff0000", "#00ff00"],
    ["hsl(0, 100%, 50%)", "hsl(120, 50%, 50%)"],
  ]) {
    const i = function_(a, b);
    out.colours[`${name}/${a}/${b}`] = points.map((t) => i(t));
  }
}

// The colours along a curve.
out.colourCurves = {};
for (const [name, function_] of Object.entries({
  rgbBasis: d3interpolate.interpolateRgbBasis,
  rgbBasisClosed: d3interpolate.interpolateRgbBasisClosed,
})) {
  const i = function_(["red", "green", "blue"]);
  out.colourCurves[name] = points.map((t) => i(t));
}

// The hue.
out.hue = {};
for (const [a, b] of [[0, 120], [0, 240], [350, 10], [10, 10]]) {
  const i = d3interpolate.interpolateHue(a, b);
  out.hue[`${a}/${b}`] = points.map((t) => i(t));
}

// The curves of numbers.
out.curves = {};
for (const values of [[0, 10, 20], [0, 100], [1, 2, 3, 4, 5]]) {
  for (const [name, function_] of Object.entries({
    basis: d3interpolate.interpolateBasis,
    basisClosed: d3interpolate.interpolateBasisClosed,
  })) {
    const i = function_(values);
    out.curves[`${name}/${values}`] = points.map((t) => i(t));
  }
}

// The discrete.
out.discrete = {};
{
  const i = d3interpolate.interpolateDiscrete(["a", "b", "c"]);
  out.discrete = points.map((t) => i(t));
}

// The piecewise.
out.piecewise = {};
{
  const i = d3interpolate.piecewise(d3interpolate.interpolateNumber, [0, 10, 20]);
  out.piecewise = points.map((t) => i(t));
}

// The quantize.
out.quantize = {};
{
  const i = d3interpolate.quantize(d3interpolate.interpolateNumber(0, 10), 5);
  out.quantize = i;
}

// The zoom.
out.zoom = {};
for (const [a, b] of [
  [[0, 0, 1], [10, 10, 2]],
  [[0, 0, 1], [0, 0, 1]],
  [[0, 0, 1], [100, 0, 1000]],
]) {
  const i = d3interpolate.interpolateZoom(a, b);
  out.zoom[`${a}/${b}`] = points.map((t) => i(t));
}

// The numbers are written down as their bits, for a number written in
// text and read back may not be the same number.
function encode(value) {
  if (typeof value === "number") return bits(value);
  if (value instanceof Date) return bits(+value);
  if (Array.isArray(value)) return value.map(encode);
  if (value && typeof value === "object") {
    const out = {};
    for (const key of Object.keys(value)) out[key] = encode(value[key]);
    return out;
  }
  return value;
}

// A value of the JavaScript's own, which the port has as a `Value`.
function encodeValue(value) {
  if (value === null || value === undefined) return {kind: "null"};
  if (typeof value === "number") return {kind: "number", value: bits(value)};
  if (typeof value === "boolean") return {kind: "boolean", value};
  if (typeof value === "string") return {kind: "string", value};
  if (value instanceof Date) return {kind: "date", value: bits(+value)};
  if (Array.isArray(value)) {
    return {kind: "array", value: value.map(encodeValue)};
  }
  if (value && typeof value === "object") {
    const out = {kind: "object", value: {}};
    for (const key of Object.keys(value)) {
      out.value[key] = encodeValue(value[key]);
    }
    return out;
  }
  return {kind: "other"};
}

out.value = {};
for (const [a, b] of [
  [0, 10],
  [0, "10"],
  ["a", "b"],
  ["red", "blue"],
  [0, true],
  [0, null],
  [[0, 10], [10, 20]],
  [{x: 0}, {x: 10}],
]) {
  const i = d3interpolate.interpolate(a, b);
  out.value[`${JSON.stringify(a)}/${JSON.stringify(b)}`] =
    points.map((t) => encodeValue(i(t)));
}

// The decompose of a matrix, which is the one the transform
// interpolation works with.
{
  const decompose = require(root + "/d3-interpolate/src/transform/decompose.js");
  const result = {};
  for (const m of [
    [1, 0, 0, 1, 0, 0],
    [1, 0, 0, 1, 10, 20],
    [2, 0, 0, 2, 10, 20],
    [0.5, 0.5, -0.5, 0.5, 0, 0],
    [1, 0.2, 0.3, 1, 5, 6],
    [0, 1, -1, 0, 0, 0],
  ]) {
    const d = decompose.default(...m);
    result[m.join("/")] = {
      translateX: bits(d.translateX),
      translateY: bits(d.translateY),
      rotate: bits(d.rotate),
      skewX: bits(d.skewX),
      scaleX: bits(d.scaleX),
      scaleY: bits(d.scaleY),
    };
  }
  out.decompose = result;
}

writeFileSync(
  process.argv[2] || "tests/fixtures/interpolate.json",
  JSON.stringify(encode(out), null, 1) + "\n");
