// The oracle: d3-color's own JavaScript, run under Node, writing what it
// produces.
//
// Every fixture in the port's `tests/fixtures` is produced by this script, so
// that the Rust is checked against the JavaScript it was translated from
// rather than against an expectation written by hand.
//
//   node scripts/color_oracle.mjs > tests/fixtures/color.json

import {writeFileSync} from "fs";
import {createRequire} from "module";

const require = createRequire(import.meta.url);
const root = "/tmp/opencode/d3src/node_modules";

function d3(pkg) {
  return require(root + "/" + pkg + "/src/index.js");
}

const d3color = d3("d3-color");

// The bits of a number, for a number written in text and read back may
// not be the same number.
const buffer = new ArrayBuffer(8);
const view = new DataView(buffer);

function bits(value) {
  if (typeof value === "number") {
    view.setFloat64(0, value, false);
    let text = "";
    for (let index = 0; index < 8; index += 1) {
      text += view.getUint8(index).toString(16).padStart(2, "0");
    }
    return text;
  }
  return value;
}

const formats = [
  "#ff0000", "#f00", "#ff000000", "#f000", "#FF0000", "  #ff0000  ",
  "rgb(255, 0, 0)", "rgb(100%, 0%, 0%)", "rgba(255, 0, 0, 1)",
  "rgba(255, 0, 0, 0.5)", "rgba(100%, 0%, 0%, 0.5)",
  "hsl(120, 50%, 50%)", "hsla(120, 50%, 50%, 1)", "hsla(120, 50%, 50%, 0.5)",
  "red", "rebeccapurple", "transparent", "white", "black",
  "not a colour", "", "#ff00", "#ff00000000", "rgb(255, 0)", "rgb(1, 2, 3, 4)",
  "hsl(120, 50, 50)", "rgb( 255 , 0 , 0 )", "RGB(255, 0, 0)",
];

const out = {};

out.parsed = {};
for (const format of formats) {
  const colour = d3color.color(format);
  // The colour's own channels, which are the ones of its own kind: an
  // `Hsl` has no red, and a `Lab` has no hue.
  const channels = {};
  if (colour !== null) {
    for (const key of Object.keys(colour)) {
      channels[key] = bits(colour[key]);
    }
  }
  out.parsed[format] = colour === null ? null : {
    channels,
    formatHex: colour.formatHex(),
    formatRgb: colour.formatRgb(),
    formatHsl: colour.formatHsl(),
    displayable: colour.displayable(),
  };
}

// The conversions between the spaces.
const colour = d3color.color("steelblue");
out.conversions = {
  rgb: [bits(colour.r), bits(colour.g), bits(colour.b), bits(colour.opacity)],
  hsl: (() => {
    const hsl = d3color.hsl(colour);
    return [bits(hsl.h), bits(hsl.s), bits(hsl.l), bits(hsl.opacity)];
  })(),
  lab: (() => {
    const lab = d3color.lab(colour);
    return [bits(lab.l), bits(lab.a), bits(lab.b), bits(lab.opacity)];
  })(),
  hcl: (() => {
    const hcl = d3color.hcl(colour);
    return [bits(hcl.h), bits(hcl.c), bits(hcl.l), bits(hcl.opacity)];
  })(),
  cubehelix: (() => {
    const cubehelix = d3color.cubehelix(colour);
    return [bits(cubehelix.h), bits(cubehelix.s), bits(cubehelix.l),
      bits(cubehelix.opacity)];
  })(),
};

// The brighter and the darker.
out.brighter = [];
out.darker = [];
for (const format of ["steelblue", "#ff0000", "hsl(120, 50%, 50%)", "transparent"]) {
  const c = d3color.color(format);
  const brighter = c.brighter();
  const darker = c.darker();
  out.brighter.push([
    brighter.formatRgb(), brighter.formatHex(), brighter.formatHsl(),
  ]);
  out.darker.push([darker.formatRgb(), darker.formatHex(), darker.formatHsl()]);
}

// The colours the JavaScript makes itself.
out.made = {
  rgb: d3color.rgb(1, 2, 3).formatRgb(),
  rgbOpacity: d3color.rgb(1, 2, 3, 0.5).formatRgb(),
  hsl: d3color.hsl(120, 0.5, 0.5).formatHsl(),
  hslOpacity: d3color.hsl(120, 0.5, 0.5, 0.5).formatHsl(),
  lab: d3color.lab(50, 20, -30).formatRgb(),
  gray: d3color.gray(50).formatRgb(),
  hcl: d3color.hcl(120, 40, 50).formatRgb(),
  lch: d3color.lch(50, 40, 120).formatRgb(),
  cubehelix: d3color.cubehelix(120, 0.5, 0.5).formatRgb(),
};

writeFileSync(
  process.argv[2] || "tests/fixtures/color.json",
  JSON.stringify(out, null, 1) + "\n");
