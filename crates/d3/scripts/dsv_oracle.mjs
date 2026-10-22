// The oracle: d3-dsv's own JavaScript, run under Node, writing what it
// produces.
//
//   node scripts/dsv_oracle.mjs > tests/fixtures/dsv.json
//
// The dates are read in the timezone of Greenwich, so that the fixture
// is the same wherever it is made — for the JavaScript reads a date
// without a timezone in the timezone of the program that runs it, and
// the port reads it in the timezone it is given.

import {writeFileSync} from "fs";

process.env.TZ = "UTC";

const root = "/tmp/opencode/d3src/node_modules";
const d3dsv = await import(root + "/d3-dsv/src/index.js");

// The bits of a number, which the fixture writes down rather than the
// number itself: a number written in text and read back may not be the
// same number.
function bits(value) {
  const buffer = new ArrayBuffer(8);
  new Float64Array(buffer)[0] = value;
  return Array.from(new Uint8Array(buffer), (byte) =>
    byte.toString(16).padStart(2, "0")).reverse().join("");
}

// A value of a row, which the fixture holds with its own kind.
function encode(value) {
  if (value === null || value === undefined) return {null: true};
  if (typeof value === "boolean") return {bool: value};
  if (typeof value === "number") return {bits: bits(value)};
  if (value instanceof Date) {
    return {date: {bits: bits(value.getTime()), text: d3dsv.csvFormatValue(value)}};
  }
  return {text: String(value)};
}

// A row of a table, which is the names of its values and the values
// themselves, in the order the object has them in.
function encodeRow(row) {
  return Object.keys(row).map((name) => [name, encode(row[name])]);
}

// The rows of a table.
function encodeRows(rows) {
  return rows.map(encodeRow);
}

// The texts of a run of luck, which are made of the characters that the
// parser looks at: a quote, the delimiter, the newline, the carriage
// return, and the like. The seed is fixed, so the texts are the same
// wherever the fixture is made.
function fuzzTexts(count) {
  const alphabet = ["a", "b", '"', ",", "\n", "\r", "\t", " ", "é", "\u00a0", "\\"];
  let seed = 12345;
  const next = () => {
    seed = (seed * 1103515245 + 12345) & 0x7fffffff;
    return seed;
  };
  const texts = [];
  for (let i = 0; i < count; i += 1) {
    const length = next() % 24;
    let text = "";
    for (let j = 0; j < length; j += 1) text += alphabet[next() % alphabet.length];
    texts.push(text);
  }
  return texts;
}

// The texts that are read, which are the same for every delimiter but
// for the delimiter itself.
function texts(delimiter) {
  return [
    "",
    "\n",
    `a${delimiter}b`,
    `a${delimiter}b\n1${delimiter}2`,
    `a${delimiter}b\n1${delimiter}2\n3${delimiter}4`,
    `a${delimiter}b\n1${delimiter}2\n`,
    `a${delimiter}b\n`,
    `a${delimiter}b\n\n1${delimiter}2`,
    `a${delimiter}b\r\n1${delimiter}2\r\n`,
    `a${delimiter}b\r1${delimiter}2`,
    `a${delimiter}b\n1${delimiter}2\r\n3${delimiter}4`,
    `a${delimiter}b${delimiter}c\n1${delimiter}2${delimiter}3`,
    `a${delimiter}b\n1`,
    `a${delimiter}b\n1${delimiter}2${delimiter}3`,
    `a${delimiter}${delimiter}b\n1${delimiter}${delimiter}2`,
    `  a  ${delimiter}  b  \n1${delimiter}2`,
    `"a"${delimiter}"b"\n"1"${delimiter}"2"`,
    `a${delimiter}"b${delimiter}c"\n1${delimiter}2`,
    `"a\nb"${delimiter}c\n1${delimiter}2`,
    `"a""b"${delimiter}c\n1${delimiter}2`,
    `"unclosed${delimiter}1${delimiter}2`,
    `"a"b${delimiter}c\n1${delimiter}2`,
    `a${delimiter}b\n""${delimiter}x`,
    `a${delimiter}b\né${delimiter}è`,
    `a${delimiter}b\n"éè"${delimiter}x`,
    `a${delimiter}a${delimiter}b\n1${delimiter}2${delimiter}3`,
    `a${delimiter}b\n1${delimiter}2\n3${delimiter}4\n5${delimiter}6`,
    `a${delimiter}b\n"x""y"${delimiter}2`,
    `a${delimiter}b\n"x"""${delimiter}2`,
    `﻿a${delimiter}b\n1${delimiter}2`,
    `a${delimiter}b\n"multi\nline"${delimiter}2`,
    // The names of the columns are the names of an object, and an
    // object keeps the names that are numbers first.
    `2${delimiter}a${delimiter}1\nx${delimiter}y${delimiter}z`,
    `b${delimiter}1${delimiter}a\nx${delimiter}y${delimiter}z`,
    `0${delimiter}1\nx${delimiter}y`,
    `01${delimiter}1\nx${delimiter}y`,
    `__proto__${delimiter}b\nx${delimiter}y`,
    // A character outside the Basic Multilingual Plane, which the
    // JavaScript counts as two characters and the port as one.
    '"😀",x\n1,2',
    '"a""😀b",c\n1,2',
    "😀,b\n1,2",
    ...fuzzTexts(50),
    // A comma is a character of its own in a text that is not read with
    // a comma as its delimiter.
    "a,b\n1,2",
    // A tab is a character of its own in a text that is not read with a
    // tab as its delimiter.
    "a\tb\n1\t2",
  ];
}

// The rows of a table that are written out, which are the rows of every
// kind of value and every kind of name.
const rows = [
  [],
  [{a: 1, b: 2}],
  [{a: 1, b: "x,y"}, {a: null, b: undefined, c: 'q"z'}],
  [{b: 1}, {a: 2}],
  [{2: "x", a: "y", 1: "z"}],
  [{0: "x", b: "y"}],
  [{"01": "x", b: "y"}],
  [{a: new Date(Date.UTC(2019, 0, 2, 3, 4, 5, 6))}],
  [{a: true, b: false}],
  [{a: NaN, b: Infinity, c: -Infinity}],
  [{a: 1e21, b: -0, c: 0.1}],
  [{a: "", b: "x\ny"}],
];

// The columns the rows are written out with, which are the columns
// themselves and the columns that are not there.
const columns = [null, ["b", "a"], ["b", "a", "missing"], ["a"], [], ["a,b"], ["a\nb"]];

// The rows of values that are written out.
const valueRows = [
  [],
  [1, "a,b"],
  [null, undefined],
  [new Date(Date.UTC(2019, 0, 2, 3, 4, 5, 6))],
  [new Date(Date.UTC(2019, 0, 2))],
  [new Date(NaN)],
  [true, false],
  [NaN, Infinity, -Infinity],
  [1e21, -0, 0.1],
  ["", "x\ny", 'q"z'],
];

// The values that are written out, which are the values of every kind.
const values = [
  null,
  undefined,
  "",
  "a",
  "a,b",
  'q"z',
  "x\ny",
  "x\ry",
  "x\ty",
  0,
  -0,
  1,
  -1,
  0.1,
  1e21,
  NaN,
  Infinity,
  -Infinity,
  true,
  false,
  new Date(Date.UTC(2019, 0, 2, 3, 4, 5, 6)),
  new Date(Date.UTC(2019, 0, 2)),
  new Date(Date.UTC(2019, 0, 2, 3, 4)),
  new Date(Date.UTC(1969, 11, 31, 23, 59, 59, 999)),
  new Date(NaN),
];

// The values that are read, which are the values of every kind that the
// JavaScript's own reads.
const read = [
  "",
  " ",
  "\t",
  " ",
  "﻿",
  "true",
  "false",
  "True",
  "FALSE",
  "NaN",
  "nan",
  "0",
  "1",
  "-1",
  "+1",
  "1.5",
  "-1.5",
  "1e3",
  "1E3",
  "1e-3",
  "0x1F",
  "0X1f",
  "0b101",
  "0o17",
  "Infinity",
  "+Infinity",
  "-Infinity",
  "inf",
  "infinity",
  "NaN",
  "1_000",
  ".5",
  "5.",
  " 12 ",
  "12 34",
  "0.0000001",
  "12345678901234567890",
  "2019",
  "2019-01",
  "2019-01-02",
  "2019-01-02T03:04",
  "2019-01-02T03:04:05",
  "2019-01-02T03:04:05.123",
  "2019-01-02T03:04:05Z",
  "2019-01-02T03:04:05.123Z",
  "2019-01-02T03:04:05+02:00",
  "2019-01-02T03:04:05-05:30",
  "2019-01-02T03:04:05+0200",
  "2019-01-02T24:00",
  "2019-01-02T24:00:00",
  "2019-01-02T25:00",
  "2019-01-02T03:60",
  "2019-01-02T03:04:05.1",
  "2019-13-02",
  "2019-02-31",
  "2019-00-02",
  "2019-01-32",
  "2019-01-00",
  "+002019-01-02",
  "-002019-01-02",
  "+275760-09-13",
  "+275760-09-14",
  "20190102",
  "2019-1-2",
  "2019-01-02 03:04:05",
  "2019-01-02T03",
  "9999",
  "10000",
  "-1",
  "-2019",
  "1-2",
  "2019-01-02t03:04:05",
];

// The row that is read with the JavaScript's own `autoType`.
function typed(row) {
  return d3dsv.autoType(row);
}

// The row that is read with a function that leaves out the first row of
// the table.
function skipped(row, index) {
  return index === 0 ? null : row;
}

// The row of values that is read with a function that leaves out every
// other row and writes a mark on the ones that are left.
function marked(row, index) {
  return index % 2 === 0 ? null : row.map((value) => `${value}!`);
}

const out = {
  delimiters: {},
  texts: {},
  tables: rows.map(encodeRows),
  columns,
  values: values.map(encode),
  valueRows: valueRows.map((row) => row.map(encode)),
  autoType: [],
  csv: {},
  tsv: {},
  semi: {},
};

// The values that are read, one at a time.
for (const value of read) {
  out.autoType.push({text: value, value: encode(d3dsv.autoType({v: value}).v)});
}

// The three delimiters, which are the comma, the tab and the semicolon.
for (const [name, delimiter] of Object.entries({csv: ",", tsv: "\t", semi: ";"})) {
  const dsv = d3dsv.dsvFormat(delimiter);
  out.delimiters[name] = delimiter;
  out.texts[name] = texts(delimiter);

  const section = {};

  // The rows of the texts, as they are read.
  section.parseRows = out.texts[name].map((text) => dsv.parseRows(text));

  // The rows of the texts, under the names of the first row.
  section.parse = out.texts[name].map((text) => {
    const parsed = dsv.parse(text);
    return {columns: parsed.columns, rows: encodeRows(parsed)};
  });

  // The rows of the texts, with the values read as they are.
  section.parseWith = out.texts[name].map((text) => {
    const parsed = dsv.parse(text, typed);
    return {columns: parsed.columns, rows: encodeRows(parsed)};
  });

  // The rows of the texts, with a row left out.
  section.parseSkip = out.texts[name].map((text) => {
    const parsed = dsv.parse(text, skipped);
    return {columns: parsed.columns, rows: encodeRows(parsed)};
  });

  // The rows of values of the texts, with a row left out.
  section.parseRowsWith = out.texts[name].map((text) => dsv.parseRows(text, marked));

  // The tables that are written out, which are the tables of the
  // fixture and the columns they are written out with, in that order.
  section.format = [];
  section.formatBody = [];
  for (const row of rows) {
    for (const column of columns) {
      section.format.push(dsv.format(row, column));
      section.formatBody.push(dsv.formatBody(row, column));
    }
  }

  // The rows of values that are written out.
  section.formatRows = dsv.formatRows(valueRows);
  section.formatRow = valueRows.map((row) => dsv.formatRow(row));

  // The values that are written out.
  section.formatValue = values.map((value) => dsv.formatValue(value));

  out[name] = section;
}

// The delimiters that are not the ones of a table: an empty one, one
// with two characters in it, and the ones that make the expression of
// the JavaScript — the one that says whether a value is to be quoted —
// do something of their own.
const oddDelimiters = ["", "|", "ab", "]", "a-z", "\\"];
const oddTexts = ["a,b\n1,2", "a|b\n1|2", "a]b\n1]2", "azb\n1z2", "a\\b\n1\\2"];
const oddValues = ["a,b", "a|b", "a]b", "azb", "a\\b", "a-b", "az"];
out.odd = oddDelimiters.map((delimiter) => {
  const dsv = d3dsv.dsvFormat(delimiter);
  return {
    delimiter,
    texts: oddTexts,
    parseRows: oddTexts.map((text) => dsv.parseRows(text)),
    values: oddValues,
    formatValue: oddValues.map((value) => dsv.formatValue(value)),
  };
});

writeFileSync(
  process.argv[2] || "tests/fixtures/dsv.json",
  JSON.stringify(out, null, 1) + "\n");
