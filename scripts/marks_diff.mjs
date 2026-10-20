// Compares the marks the native draws with the reference's own vectors.
//
// The reference's marks are CSS masks in web/style-alt.css: an SVG path in
// each. The native's are in crates/squiglab/src/icons.rs, one per mark. This
// script pulls the paths out of both and compares them, so that a mark which
// has drifted from the reference's is found rather than guessed at.
//
//     node scripts/marks_diff.mjs
//
// It prints one line per mark: "matches" when the paths are the same, and
// the two paths when they are not. A mark that is drawn by hand (the cross
// that takes a curve off the graph, the fan that is the channel's key) is
// reported as "drawn by hand" and is not compared.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

// --- the reference's marks, out of its stylesheet's masks ---
const css = readFileSync(join(root, "web", "style-alt.css"), "utf8");
const masks = new Map();
for (const [, name, data] of css.matchAll(/--icon-([a-z0-9-]+): url\("([^"]+)"\)/g)) {
  const svg = decodeURIComponent(data);
  // The masks are written with double quotes; the two crosses are written
  // with single quotes, in a box of their own.
  const paths = [
    ...[...svg.matchAll(/<path[^>]*\sd="([^"]+)"/g)].map((match) => match[1]),
    ...[...svg.matchAll(/<path[^>]*\sd='([^']+)'/g)].map((match) => match[1]),
  ].filter((path) => path.replace(/\s+/g, "") !== "M00h24v24H0z");
  if (paths.length) {
    masks.set(name, paths);
  }
}

// --- the native's marks, out of its icons ---
const icons = readFileSync(join(root, "crates", "squiglab", "src", "icons.rs"), "utf8");
const native = new Map();
for (const [, variant, body] of icons.matchAll(/Icon::(\w+) => vec!\[([\s\S]*?)\],/g)) {
  const paths = [...body.matchAll(/"([^"]+)"/g)].map((match) => match[1]);
  if (paths.length) {
    native.set(variant, paths);
  }
}

// Which of the native's marks is which of the reference's.
const pairs = {
  Download: "download",
  Eye: "hide",
  EyeCrossed: "hidden",
  Pin: "pin",
  Settings: "eq-settings",
  Reset: "eq-reset",
  Squiggle: "squiggle",
  BaselineLine: "baseline",
  Volume: null,
  Play: "player-play",
  Stop: "player-stop",
  Remove: "remove",
  Plus: "plus",
  Baseline: null,
};

// The marks the native draws by hand, and why.
const byHand = {
  // The cross is a filled shape that cannot be filled here, so it is drawn as
  // the two bars it is made of.
  Remove: "drawn by hand",
  Plus: "drawn by hand",
  // The speaker comes from the reference's markup rather than its masks.
  Volume: "from the markup",
  // The words are the native's own.
  Baseline: "the native's own",
};

const normalise = (path) => path.replace(/\s+/g, "").replace(/,/g, "");

let same = 0;
let different = 0;
for (const [variant, name] of Object.entries(pairs)) {
  if (!name) {
    console.log(`${variant.padEnd(14)} ${byHand[variant] ?? "no counterpart"}`);
    continue;
  }
  const reference = masks.get(name);
  const ours = native.get(variant);
  if (byHand[variant]) {
    // The mark is drawn by hand here, so the reference's own path is shown
    // instead of a comparison: it is the geometry the hand has to follow.
    console.log(`${variant.padEnd(14)} ${byHand[variant]}`);
    if (reference) {
      console.log(`               the reference: ${reference[0]}`);
    }
    continue;
  }
  if (!reference) {
    console.log(`${variant.padEnd(14)} the reference has no --icon-${name}`);
    continue;
  }
  if (!ours) {
    console.log(`${variant.padEnd(14)} the native does not draw it`);
    continue;
  }
  const a = reference.map(normalise);
  const b = ours.map(normalise);
  if (a.length === b.length && a.every((path, index) => path === b[index])) {
    console.log(`${variant.padEnd(14)} matches --icon-${name}`);
    same += 1;
  } else {
    console.log(`${variant.padEnd(14)} DIFFERS from --icon-${name}`);
    for (let index = 0; index < Math.max(a.length, b.length); index += 1) {
      if (a[index] !== b[index]) {
        console.log(`               the reference: ${a[index] ?? "(none)"}`);
        console.log(`               the native:    ${b[index] ?? "(none)"}`);
        break;
      }
    }
    different += 1;
  }
}
console.log(`\n${same} marks match, ${different} differ.`);
