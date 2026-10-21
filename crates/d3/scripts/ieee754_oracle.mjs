// The oracle: V8's own mathematics, run under Node, writing what it
// produces.
//
// Every fixture in the port's `tests/fixtures` is produced by this script, so
// that the Rust is checked against the JavaScript it was translated from
// rather than against an expectation written by hand.
//
// The port's functions are the ones V8 carries, `src/base/ieee754.cc`, which
// are the fdlibm of Sun Microsystems and not the platform's own `libm`.
// The bits are written as they are, for a number that is written as a
// decimal may not come back the same.
//
//   node scripts/ieee754_oracle.mjs > tests/fixtures/ieee754.json

import {writeFileSync} from "fs";

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

function hex(bits) {
  const bytes = new Uint8Array(8);
  for (let index = 0; index < 8; index += 1) {
    bytes[index] = parseInt(bits.slice(index * 2, index * 2 + 2), 16);
  }
  return new DataView(bytes.buffer).getFloat64(0, false);
}

// The inputs, which are the ones that show the difference between the
// functions: the small, the large, the ones near a boundary, the ones
// that are not a number, and the ones that are infinite.
const values = [
  0, -0, 1, -1, 0.5, -0.5, 2, -2, 3, 10, 100, 1000, 1e6, 1e10,
  0.1, 0.2, 0.3, 1e-300, 1e-100, 1e-10, 1e-5,
  0.7071067811865476, 0.8660254037844387, 0.5773502691896257,
  1.5707963267948966, 3.141592653589793, 6.283185307179586,
  1.0471975511965979, 2.0943951023931953,
  1.0471975511965979 - 1e-15, 3.141592653589793 - 1e-15,
  1.5, 2.5, 3.5, 4.5, 0.25, 0.75, 1.25,
  1.0000000000000002, 1.0000000000000004, 0.9999999999999999,
  88, 89, 700, 709, 710, 745, 746,
  -88, -89, -700, -709, -710,
  1.7976931348623157e308, 2.2250738585072014e-308, 4.9406564584124654e-324,
];

const pairs = [];
for (const y of values) {
  for (const x of [-3, -1, -0.5, 0, 0.5, 1, 3, 1e6, 1e-6]) {
    pairs.push([y, x]);
  }
}

const unary = {
  sin: Math.sin,
  cos: Math.cos,
  tan: Math.tan,
  asin: Math.asin,
  acos: Math.acos,
  atan: Math.atan,
  exp: Math.exp,
  log: Math.log,
  log2: Math.log2,
  log10: Math.log10,
  log1p: Math.log1p,
  cbrt: Math.cbrt,
  expm1: Math.expm1,
  sinh: Math.sinh,
  cosh: Math.cosh,
  tanh: Math.tanh,
  asinh: Math.asinh,
  acosh: Math.acosh,
  atanh: Math.atanh,
};

const cases = {};

for (const [name, function_] of Object.entries(unary)) {
  const result = {};
  for (const value of values) {
    result[bits(value)] = bits(function_(value));
  }
  cases[name] = result;
}

for (const [name, function_] of Object.entries({
  atan2: Math.atan2,
  pow: Math.pow,
})) {
  const result = {};
  for (const [y, x] of pairs) {
    result[bits(y) + "/" + bits(x)] = bits(function_(y, x));
  }
  cases[name] = result;
}

// The powers, which have a case of their own for a whole number and
// one for a fraction, and for a base below one and one above it.
{
  const result = {};
  const exponents = [0, 1, 2, 3, 4, 10, 0.5, 1.5, 2.4, -1, -0.5, -2.5,
                     1e-300, 1e300];
  for (const x of [0.5, 0.25, 2, 3, 10, 0.1, 1e-10, 1e10, 0, 1, -1, -2]) {
    for (const y of exponents) {
      result[bits(x) + "/" + bits(y)] = bits(Math.pow(x, y));
    }
  }
  cases["pow"] = result;
}

// The `sqrt`, and the rest, are the same in every JavaScript and in
// Rust, for they are the platform's own.
cases["sqrt"] = Object.fromEntries(
  values.map((value) => [bits(value), bits(Math.sqrt(value))]));

writeFileSync(
  process.argv[2] || "tests/fixtures/ieee754.json",
  JSON.stringify(cases, null, 1) + "\n");
