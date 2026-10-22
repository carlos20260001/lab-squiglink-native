import {writeFileSync} from "fs";
import {createRequire} from "module";

const require = createRequire(import.meta.url);
const root = "/tmp/opencode/d3src/node_modules";

function d3(pkg) {
  return require(root + "/" + pkg + "/src/index.js");
}

const factory = require("/tmp/opencode/d3src/node_modules/d3-chord/build/d3-chord.js");
const d3chord = factory.d3 || factory;

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

const matrix = [
  [0, 5, 6, 4, 7, 4],
  [7, 0, 5, 6, 4, 7],
  [6, 7, 0, 5, 6, 4],
  [4, 6, 7, 0, 5, 6],
  [6, 4, 6, 7, 0, 5],
  [5, 6, 4, 6, 7, 0],
];

const out = {};

// The chord, with its groups and its chords.
const chord = d3chord.chord();
const chords = chord(matrix);
const ribbon = d3chord.ribbon().radius((d) => d.radius);
out.chord = {
  groups: chords.groups.map((group) => ({
    index: group.index,
    startAngle: bits(group.startAngle),
    endAngle: bits(group.endAngle),
    value: bits(group.value),
  })),
  chords: chords.map((chord) => ({
    source: {
      index: chord.source.index,
      subindex: chord.source.subindex,
      startAngle: bits(chord.source.startAngle),
      endAngle: bits(chord.source.endAngle),
      value: bits(chord.source.value),
    },
    target: {
      index: chord.target.index,
      subindex: chord.target.subindex,
      startAngle: bits(chord.target.startAngle),
      endAngle: bits(chord.target.endAngle),
      value: bits(chord.target.value),
    },
  })),
};

// The chord with a pad angle.
const chordPad = d3chord.chord().padAngle(0.05);
const chordsPad = chordPad(matrix);
out.chordPad = {
  groups: chordsPad.groups.map((group) => ({
    index: group.index,
    startAngle: group.startAngle,
    endAngle: group.endAngle,
    value: group.value,
  })),
  chords: chordsPad.map((chord) => ({
    source: chord.source,
    target: chord.target,
  })),
};

// The chord with the groups sorted.
const chordSort = d3chord.chord().sortGroups(d3chord.chordDescending);
const chordsSort = chordSort(matrix);
out.chordSort = {
  groups: chordsSort.groups.map((group) => ({
    index: group.index,
    startAngle: group.startAngle,
    endAngle: group.endAngle,
    value: group.value,
  })),
  chords: chordsSort.map((chord) => ({
    source: chord.source,
    target: chord.target,
  })),
};

// The chord with the subgroups sorted.
const chordSub = d3chord.chord().sortSubgroups(d3chord.chordDescending);
const chordsSub = chordSub(matrix);
out.chordSub = {
  groups: chordsSub.groups.map((group) => ({
    index: group.index,
    startAngle: group.startAngle,
    endAngle: group.endAngle,
    value: group.value,
  })),
  chords: chordsSub.map((chord) => ({
    source: chord.source,
    target: chord.target,
  })),
};

// The chord with the chords sorted.
const chordChord = d3chord.chord().sortChords(d3chord.chordDescending);
const chordsChord = chordChord(matrix);
out.chordChord = {
  groups: chordsChord.groups.map((group) => ({
    index: group.index,
    startAngle: group.startAngle,
    endAngle: group.endAngle,
    value: group.value,
  })),
  chords: chordsChord.map((chord) => ({
    source: chord.source,
    target: chord.target,
  })),
};

// The ribbon.
const chord0 = chords[0];
out.ribbon = {
  path: ribbon({
    source: Object.assign({}, chord0.source, {radius: 1}),
    target: Object.assign({}, chord0.target, {radius: 1}),
  }),
  source: chord0.source,
  target: chord0.target,
};

// The ribbon with a different radius.
const ribbon2 = d3chord.ribbon().radius((d) => d.radius);
const chord0r = {
  source: Object.assign({}, chord0.source, {radius: 50}),
  target: Object.assign({}, chord0.target, {radius: 50}),
};
out.ribbonRadius = {
  path: ribbon2(chord0r),
  source: chord0r.source,
  target: chord0r.target,
};

writeFileSync(
  process.argv[2] || "tests/fixtures/chord.json",
  JSON.stringify(out, null, 1) + "\n"
);
