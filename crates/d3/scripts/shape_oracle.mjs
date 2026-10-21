// The oracle for d3-shape: the real d3, run under Node, writing down what
// it produced.
//
//   node scripts/shape_oracle.mjs > tests/fixtures/shape.json
import {createRequire} from "module";

const require = createRequire(import.meta.url);
const d3 = require("/tmp/opencode/d3src/node_modules/d3-shape/dist/d3-shape.js");

const points = [];
for (let i = 0; i < 12; i++) {
  points.push([i * 37.5, Math.sin(i * 0.7) * 23 + i * 1.5]);
}

// The JavaScript's builders change the object they are given and give it
// back, so each case needs an object of its own.
const line = () => d3.line().x(d => d[0]).y(d => d[1]);
const area = () => d3.area().x(d => d[0]).y0(d => d[1] - 5).y1(d => d[1] + 5);

// The numbers are written down as their bits, because a number written
// in text and read back may not be the same number: the reader may
// round it to the nearest one, and the nearest one is not always the
// one that was written.
const bits = x => {
  const b = new ArrayBuffer(8);
  new Float64Array(b)[0] = x;
  // The bytes are written down most significant first, which is the order
  // the Rust reads them in.
  return Array.from(new Uint8Array(b))
    .reverse()
    .map(v => v.toString(16).padStart(2, "0"))
    .join("");
};

const out = {};

// The line, with every curve.
out.line = line()(points);
out.lineNatural = line().curve(d3.curveNatural)(points);
out.lineCardinal = line().curve(d3.curveCardinal.tension(0.5))(points);
out.lineCardinalZero = line().curve(d3.curveCardinal)(points);
out.lineCardinalOpen = line().curve(d3.curveCardinalOpen.tension(0.5))(points);
out.lineCardinalClosed = line().curve(d3.curveCardinalClosed.tension(0.5))(points);
out.lineMonotoneX = line().curve(d3.curveMonotoneX)(points);
out.lineMonotoneY = line().curve(d3.curveMonotoneY)(points);
out.lineBasis = line().curve(d3.curveBasis)(points);
out.lineBasisOpen = line().curve(d3.curveBasisOpen)(points);
out.lineBasisClosed = line().curve(d3.curveBasisClosed)(points);
out.lineBundle = line().curve(d3.curveBundle.beta(0.5))(points);
out.lineBundleOne = line().curve(d3.curveBundle.beta(1))(points);
out.lineCatmullRom = line().curve(d3.curveCatmullRom.alpha(0.5))(points);
out.lineCatmullRomOpen = line().curve(d3.curveCatmullRomOpen.alpha(0.5))(points);
out.lineCatmullRomClosed = line().curve(d3.curveCatmullRomClosed.alpha(0.5))(points);
out.lineStep = line().curve(d3.curveStep)(points);
out.lineStepAfter = line().curve(d3.curveStepAfter)(points);
out.lineStepBefore = line().curve(d3.curveStepBefore)(points);
out.lineLinearClosed = line().curve(d3.curveLinearClosed)(points);

// A line with a break in it, and a line of nothing.
out.lineDefined = line().defined((d, i) => i % 3 !== 1)(points);
out.lineEmpty = line()([]);

// The area, with every curve.
out.area = area()(points);
out.areaNatural = area().curve(d3.curveNatural)(points);
out.areaCardinal = area().curve(d3.curveCardinal.tension(0.5))(points);
out.areaStep = area().curve(d3.curveStep)(points);
out.areaDefined = area().defined((d, i) => i % 3 !== 1)(points);
out.areaEmpty = area()([]);

// The symbols.
out.symbolCircle = d3.symbol().type(d3.symbolCircle).size(64)();
out.symbolCross = d3.symbol().type(d3.symbolCross).size(64)();
out.symbolDiamond = d3.symbol().type(d3.symbolDiamond).size(64)();
out.symbolSquare = d3.symbol().type(d3.symbolSquare).size(64)();
out.symbolStar = d3.symbol().type(d3.symbolStar).size(64)();
out.symbolTriangle = d3.symbol().type(d3.symbolTriangle).size(64)();
out.symbolWye = d3.symbol().type(d3.symbolWye).size(64)();
out.symbolSize = d3.symbol().type(d3.symbolCircle).size(100)();

// The pie.
out.pie = d3.pie().value(d => d)([1, 2, 3, 4]).map(a => ({
  index: a.index,
  value: bits(a.value),
  startAngle: bits(a.startAngle),
  endAngle: bits(a.endAngle),
  padAngle: bits(a.padAngle),
}));
out.pieSortValues = d3.pie().value(d => d).sortValues((a, b) => a - b)([1, 2, 3, 4])
  .map(a => ({index: a.index, value: bits(a.value), startAngle: bits(a.startAngle),
              endAngle: bits(a.endAngle)}));
out.piePad = d3.pie().value(d => d).padAngle(0.1)([1, 2, 3, 4])
  .map(a => ({index: a.index, startAngle: bits(a.startAngle), endAngle: bits(a.endAngle),
              padAngle: bits(a.padAngle)}));

// The arc.
out.arc = d3.arc().innerRadius(10).outerRadius(50)({startAngle: 0, endAngle: Math.PI / 2});
out.arcCentroid = d3.arc().innerRadius(10).outerRadius(50)
  .centroid({startAngle: 0, endAngle: Math.PI / 2}).map(bits);
out.arcCorner = d3.arc().innerRadius(20).outerRadius(50).cornerRadius(5)
  ({startAngle: 0, endAngle: Math.PI / 2});
out.arcPad = d3.arc().innerRadius(20).outerRadius(50).padAngle(0.1)
  ({startAngle: 0, endAngle: Math.PI / 2});
out.arcCircle = d3.arc().innerRadius(0).outerRadius(50)
  ({startAngle: 0, endAngle: 2 * Math.PI});
out.arcPoint = d3.arc().innerRadius(0).outerRadius(0)
  ({startAngle: 0, endAngle: Math.PI / 2});

// The links.
// The link's x and y read the end of the link, so the accessors take the
// end rather than the whole link.
// The link is called with one datum at a time: its source and its target.
out.linkHorizontal = d3.linkHorizontal().x(d => d[0]).y(d => d[1])
  ({source: [0, 0], target: [10, 10]});
out.linkVertical = d3.linkVertical().x(d => d[0]).y(d => d[1])
  ({source: [0, 0], target: [10, 10]});
out.linkRadial = d3.linkRadial().angle(d => d[0]).radius(d => d[1])
  ({source: [0, 0], target: [1, 10]});

// The stack.
const data = [
  {0: 1, 1: 2, 2: 3},
  {0: 2, 1: 1, 2: 2},
  {0: 3, 1: 3, 2: 1},
];
const stackOf = f => d3.stack().keys(["0", "1", "2"]).value((d, k) => d[k]).order(f)(data)
  .map(s => ({key: s.key, index: s.index, points: s.map(p => [bits(p[0]), bits(p[1])])}));
out.stack = stackOf(null);
out.stackOrderAscending = stackOf(d3.stackOrderAscending);
out.stackOrderDescending = stackOf(d3.stackOrderDescending);
out.stackOrderReverse = stackOf(d3.stackOrderReverse);
out.stackOrderInsideOut = stackOf(d3.stackOrderInsideOut);
out.stackOrderAppearance = stackOf(d3.stackOrderAppearance);
out.stackOffsetExpand = d3.stack().keys(["0", "1", "2"]).value((d, k) => d[k])
  .offset(d3.stackOffsetExpand)(data)
  .map(s => ({key: s.key, points: s.map(p => [bits(p[0]), bits(p[1])])}));
out.stackOffsetSilhouette = d3.stack().keys(["0", "1", "2"]).value((d, k) => d[k])
  .offset(d3.stackOffsetSilhouette)(data)
  .map(s => ({key: s.key, points: s.map(p => [bits(p[0]), bits(p[1])])}));
out.stackOffsetWiggle = d3.stack().keys(["0", "1", "2"]).value((d, k) => d[k])
  .offset(d3.stackOffsetWiggle)(data)
  .map(s => ({key: s.key, points: s.map(p => [bits(p[0]), bits(p[1])])}));
out.stackOffsetDiverging = d3.stack().keys(["0", "1", "2"]).value((d, k) => d[k])
  .offset(d3.stackOffsetDiverging)(data)
  .map(s => ({key: s.key, points: s.map(p => [bits(p[0]), bits(p[1])])}));

process.stdout.write(JSON.stringify(out, null, 2) + "\n");
