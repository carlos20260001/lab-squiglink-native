// The oracle: d3-format's own JavaScript, run under Node, writing what
// it produces.
//
//   node scripts/format_oracle.mjs > tests/fixtures/format.json

import {writeFileSync} from "fs";
import {createRequire} from "module";

const require = createRequire(import.meta.url);
const root = "/tmp/opencode/d3src/node_modules";

function d3(pkg) {
  return require(root + "/" + pkg + "/src/index.js");
}

const d3format = d3("d3-format");

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

const specifiers = [
  "", ".0", ".1", ".2", ".3", ".6", ".10", ".20", ".21",
  ",", ",.0", ",.1", ",.2", ",.3", ",.6",
  ".2d", ".3d", ".6d", ".10d",
  ".1e", ".2e", ".3e", ".6e",
  ".1f", ".2f", ".3f", ".6f", ".10f", ".20f",
  ".1g", ".2g", ".3g", ".6g", ".10g", ".21g",
  ".1r", ".2r", ".3r", ".6r", ".10r",
  ".1s", ".2s", ".3s", ".6s",
  ".1p", ".2p", ".3p", ".6p",
  ".1%", ".2%", ".3%", ".6%",
  "b", "o", "x", "X", "c", "n", ".1n", ".2n",
  "$", "$,.2", "$,.3", "$,.2f", "#.0f", "#x", "#o", "#b",
  "+", "+.2", "+,.2", " .2", "(.2", "(,.2", "-.2",
  ">", "<", "^", "=", "0", "10", "10.2", "010.2", "0,10.2",
  "~", ".2~", "~s", "~g", ".2~g", ".12~g", ".0~f",
  "*>10,.2f", "*<10,.2f", "*^10,.2f", "*=10,.2f",
  "$,.2f", "$,.2e", "$,.2g", "$,.3s",
  ">+10,.2f", " >+10,.2f", "*>+10,.2f",
];

const values = [
  0, -0, 1, -1, 0.5, -0.5, 2, -2, 3.14159, -3.14159, 100, -100,
  1000, 1234.5678, -1234.5678, 0.00001234, -0.00001234,
  1e21, -1e21, 1e-7, 123456789, 0.1, 0.2, 0.3, 1e6, 1e-6,
  999999999999999900000, 0.000001234, 42, 1e-21, 1.5e-10,
];

const out = {format: {}, prefix: {}, specifier: {}, locale: {}, precision: {}};

// The values the formats were written for, which the test reads.
out.values = values.map((value) => bits(value));

for (const specifier of specifiers) {
  // A specification the JavaScript cannot read is one the port cannot
  // read either.
  try {
    const format = d3format.format(specifier);
    out.format[specifier] = values.map((value) => format(value));
  } catch (error) {
    out.format[specifier] = null;
  }
}

// The prefix, which is a format and a value.
for (const specifier of [".0e", ".1e", ".2e", ".3e", ".0f", ".2f", ".3f", ".0r", ".2s"]) {
  for (const value of [1, 10, 100, 1000, 1e6, 1e9, 1e12, 0.001, 0.0001, 1e-9]) {
    const format = d3format.formatPrefix(specifier, value);
    out.prefix[`${specifier}/${value}`] = format(value);
  }
}

// The specification, which is read into its parts.
for (const specifier of specifiers) {
  let read;
  try {
    read = d3format.formatSpecifier(specifier);
  } catch (error) {
    out.specifier[specifier] = null;
    continue;
  }
  out.specifier[specifier] = {
    fill: read.fill,
    align: read.align,
    sign: read.sign,
    symbol: read.symbol,
    zero: read.zero,
    width: read.width === undefined ? null : read.width,
    comma: read.comma,
    precision: read.precision === undefined ? null : read.precision,
    trim: read.trim,
    type: read.type,
  };
}

// A locale of one's own.
{
  const locale = d3format.formatLocale({
    decimal: ",",
    thousands: ".",
    grouping: [3, 2],
    currency: ["€", ""],
    minus: "−",
    percent: "%",
  });
  const format = locale.format(",.2f");
  out.locale["format"] = values.map((value) => format(value));
  out.locale["prefix"] = locale.formatPrefix(".2f", 1234.5)(1234.5);
}

// The counts of the digits.
{
  const step = 0.1;
  out.precision["fixed"] = values.map((value) =>
    d3format.precisionFixed(step, value));
  out.precision["prefix"] = values.map((value) =>
    d3format.precisionPrefix(step, value));
  out.precision["round"] = values.map((value) =>
    d3format.precisionRound(step, value));
}

writeFileSync(
  process.argv[2] || "tests/fixtures/format.json",
  JSON.stringify(out, null, 1) + "\n");
