// The oracle: d3's own JavaScript, run under Node, writing what it produces.
//
// Every fixture in the port's `tests/fixtures` is produced by this script, so
// that the Rust is checked against the JavaScript it was translated from
// rather than against an expectation written by hand.
//
//   node scripts/d3_oracle.mjs <case> [...] > fixtures/<case>.json

import {readFileSync, writeFileSync} from "fs";
import {createRequire} from "module";

const require = createRequire(import.meta.url);
const root = "/tmp/opencode/d3src/node_modules";

function d3(pkg) {
  return require(root + "/" + pkg + "/src/index.js");
}

const cases = {};

// --- d3-array ------------------------------------------------------------
{
  const d3array = d3("d3-array");
  cases["array"] = {
    range: d3array.range(0, 10, 2),
    rangeNegative: d3array.range(10, 0, -2),
    rangeFloat: d3array.range(0, 1, 0.25),
    min: d3array.min([3, 1, 2]),
    minEmpty: d3array.min([]),
    minWithNaN: d3array.min([3, NaN, 2]),
    minAccessor: d3array.min([{v: 3}, {v: 1}], d => d.v),
    max: d3array.max([3, 1, 2]),
    maxEmpty: d3array.max([]),
    extent: d3array.extent([3, 1, 2]),
    extentEmpty: d3array.extent([]),
    sum: d3array.sum([1, 2, 3]),
    sumEmpty: d3array.sum([]),
    mean: d3array.mean([1, 2, 3]),
    meanEmpty: d3array.mean([]),
    median: d3array.median([1, 3, 2]),
    medianEven: d3array.median([1, 2, 3, 4]),
    quantile: d3array.quantile([1, 2, 3, 4], 0.5),
    variance: d3array.variance([1, 2, 3, 4]),
    deviation: d3array.deviation([1, 2, 3, 4]),
    merge: d3array.merge([[1, 2], [3], [4, 5]]),
    transpose: d3array.transpose([[1, 2, 3], [4, 5, 6]]),
    flatten: d3array.flatten([[1], [2, 3]]),
    flatMap: d3array.flatMap([[1, 2], [3]], d => d),
    group: Array.from(d3array.group([1, 2, 3, 4], d => d % 2 ? "odd" : "even"), ([k, v]) => [k, v]),
    rollup: Array.from(d3array.rollup([1, 2, 3, 4], v => v.length, d => d % 2 ? "odd" : "even"), ([k, v]) => [k, v]),
    partition: d3array.partition([1, 2, 3, 4], d => d % 2 === 0),
    filter: d3array.filter([1, 2, 3, 4], d => d > 2),
    map: d3array.map([1, 2, 3], d => d * 2),
    reduce: d3array.reduce([1, 2, 3], (a, b) => a + b, 0),
    count: d3array.count([1, 2, 3, 4], d => d > 2),
    every: d3array.every([1, 2, 3], d => d > 0),
    some: d3array.some([1, 2, 3], d => d > 2),
    cross: d3array.cross([1, 2], ["a", "b"]),
    pairs: d3array.pairs([1, 2, 3, 4]),
    ticks: d3array.ticks(0, 10, 5),
    tickIncrement: d3array.tickIncrement(0, 10, 5),
    tickStep: d3array.tickStep(0, 10, 5),
    bisect: d3array.bisect([1, 2, 3, 4], 3),
    bisectRight: d3array.bisectRight([1, 2, 3, 4], 3),
    bisectLeft: d3array.bisectLeft([1, 2, 3, 4], 3),
    bisectorLeft: d3array.bisectorLeft([1, 2, 3, 4])(3),
    bisectorRight: d3array.bisectorRight([1, 2, 3, 4])(3),
    ascending: d3array.ascending([3, 1, 2]),
    descending: d3array.descending([3, 1, 2]),
    least: d3array.least([1, 2, 3]),
    greatest: d3array.greatest([1, 2, 3]),
    leastIndex: d3array.leastIndex([1, 2, 3]),
    greatestIndex: d3array.greatestIndex([1, 2, 3]),
    extentAccessor: d3array.extent([{v: 1}, {v: 9}], d => d.v),
    quantileSorted: d3array.quantileSorted([1, 2, 3, 4, 5], 0.25),
    rank: d3array.rank([1, 2, 3], 2),
    cumsum: d3array.cumsum([1, 2, 3]),
    cumsumOffset: d3array.cumsum([1, 2, 3], -1),
    differences: d3array.differences([1, 3, 6, 10]),
    blur: d3array.blur(2, 0.5)([1, 2, 3]),
    blurRound: d3array.blur(2, 0.5).round([1, 2, 3]),
    groupSort: (() => {
      const g = d3array.group([{k: "b", v: 1}, {k: "a", v: 2}]);
      return Array.from(g, ([k, v]) => [k, v.length]);
    })(),
    groups: (() => {
      const g = d3array.groups([{k: "b", v: 1}, {k: "a", v: 2}]);
      return Array.from(g, ([k, v]) => [k, v.length]);
    })(),
  };
}

// --- d3-path -------------------------------------------------------------
{
  const d3path = d3("d3-path");
  const p = d3path.path();
  p.moveTo(0, 0);
  p.lineTo(1, 2);
  p.closePath();
  const q = d3path.path();
  q.moveTo(0, 0);
  q.arc(1, 2, 3, 0, Math.PI);
  cases["path"] = {
    basic: p.toString(),
    arc: q.toString(),
    digits: (() => {
      const r = d3path.path();
      r.moveTo(1.23456789, 2.3456789);
      r.lineTo(3.456789, 4.56789);
      return r.toString(3);
    })(),
  };
}

// --- d3-shape ------------------------------------------------------------
{
  const d3shape = d3("d3-shape");
  const points = [];
  for (let i = 0; i < 12; i++) {
    points.push([i * 37.5, Math.sin(i * 0.7) * 23 + i * 1.5]);
  }
  const line = d3shape.line().x(d => d[0]).y(d => d[1]);
  const area = d3shape.area().x(d => d[0]).y0(d => d[1] - 5).y1(d => d[1] + 5);
  const symbols = ["circle", "cross", "diamond", "square", "star", "triangle", "wye", "plus"];
  cases["shape"] = {
    line: line(points),
    lineNatural: line.curve(d3shape.curveNatural)(points),
    lineCardinal: line.curve(d3shape.curveCardinal.tension(0.5))(points),
    lineCardinalZero: line.curve(d3shape.curveCardinal)(points),
    lineMonotoneX: line.curve(d3shape.curveMonotoneX)(points),
    lineMonotoneY: line.curve(d3shape.curveMonotoneY)(points),
    lineBasis: line.curve(d3shape.curveBasis)(points),
    lineBundle: line.curve(d3shape.curveBundle.beta(0.5))(points),
    lineCatmullRom: line.curve(d3shape.curveCatmullRom.alpha(0.5))(points),
    lineStep: line.curve(d3shape.curveStep)(points),
    lineStepAfter: line.curve(d3shape.curveStepAfter)(points),
    lineStepBefore: line.curve(d3shape.curveStepBefore)(points),
    lineLinearClosed: line.curve(d3shape.curveLinearClosed)(points),
    areaNatural: area.curve(d3shape.curveNatural)(points),
    areaCardinal: area.curve(d3shape.curveCardinal.tension(0.5))(points),
    areaStep: area.curve(d3shape.curveStep)(points),
    symbolCircle: d3shape.symbol().type(d3shape.symbolCircle).size(64)(),
    symbolCross: d3shape.symbol().type(d3shape.symbolCross).size(64)(),
    symbolDiamond: d3shape.symbol().type(d3shape.symbolDiamond).size(64)(),
    symbolSquare: d3shape.symbol().type(d3shape.symbolSquare).size(64)(),
    symbolStar: d3shape.symbol().type(d3shape.symbolStar).size(64)(),
    symbolTriangle: d3shape.symbol().type(d3shape.symbolTriangle).size(64)(),
    symbolWye: d3shape.symbol().type(d3shape.symbolWye).size(64)(),
    symbolPlus: d3shape.symbol().type(d3shape.symbolPlus).size(64)(),
    symbolCirclePoints: d3shape.symbol().type(d3shape.symbolCircle).size(64).points(),
    stack: (() => {
      const s = d3shape.stack();
      const data = [
        {0: 1, 1: 2, 2: 3},
        {0: 2, 1: 1, 2: 2},
        {0: 3, 1: 3, 2: 1},
      ];
      return s(data);
    })(),
    stackOffsetExpand: (() => {
      const s = d3shape.stack().offset(d3shape.stackOffsetExpand);
      return s([
        {0: 1, 1: 2, 2: 3},
        {0: 2, 1: 1, 2: 2},
        {0: 3, 1: 3, 2: 1},
      ]);
    })(),
    stackOffsetSilhouette: (() => {
      const s = d3shape.stack().offset(d3shape.stackOffsetSilhouette);
      return s([
        {0: 1, 1: 2, 2: 3},
        {0: 2, 1: 1, 2: 2},
        {0: 3, 1: 3, 2: 1},
      ]);
    })(),
    stackOffsetWiggle: (() => {
      const s = d3shape.stack().offset(d3shape.stackOffsetWiggle);
      return s([
        {0: 1, 1: 2, 2: 3},
        {0: 2, 1: 1, 2: 2},
        {0: 3, 1: 3, 2: 1},
      ]);
    })(),
    stackOrderInsideOut: (() => {
      const s = d3shape.stack().order(d3shape.stackOrderInsideOut);
      return s([
        {0: 1, 1: 2, 2: 3},
        {0: 2, 1: 1, 2: 2},
        {0: 3, 1: 3, 2: 1},
      ]);
    })(),
    stackOrderReverse: (() => {
      const s = d3shape.stack().order(d3shape.stackOrderReverse);
      return s([
        {0: 1, 1: 2, 2: 3},
        {0: 2, 1: 1, 2: 2},
        {0: 3, 1: 3, 2: 1},
      ]);
    })(),
    link: (() => {
      const l = d3shape.linkHorizontal().x(d => d[0]).y(d => d[1]);
      return l([{source: [0, 0], target: [10, 10]}]);
    })(),
    linkVertical: (() => {
      const l = d3shape.linkVertical().x(d => d[0]).y(d => d[1]);
      return l([{source: [0, 0], target: [10, 10]}]);
    })(),
    linkRadial: (() => {
      const l = d3shape.linkRadial().angle(d => d[0]).radius(d => d[1]);
      return l([{source: [0, 0], target: [1, 10]}]);
    })(),
    pie: (() => {
      const p = d3shape.pie().value(d => d);
      return p([1, 2, 3, 4]);
    })(),
    pieSort: (() => {
      const p = d3shape.pie().value(d => d).sortValues((a, b) => b - a);
      return p([1, 2, 3, 4]);
    })(),
    arc: (() => {
      const a = d3shape.arc().innerRadius(10).outerRadius(50);
      return a({startAngle: 0, endAngle: Math.PI / 2});
    })(),
    arcCentroid: d3shape.arc().centroid({startAngle: 0, endAngle: Math.PI / 2}),
    radialLine: (() => {
      const l = d3shape.radialLine().angle(d => d[0]).radius(d => d[1]);
      return l([[0, 10], [1, 20]]);
    })(),
    radialArea: (() => {
      const a = d3shape.radialArea().angle(d => d[0]).innerRadius(d => d[1]).outerRadius(d => d[1] + 5);
      return a([[0, 10], [1, 20]]);
    })(),
  };
}

const which = process.argv.slice(2);
const out = {};
if (which.length === 0) {
  Object.assign(out, cases);
} else {
  for (const name of which) {
    if (!cases[name]) throw new Error("no such case: " + name);
    out[name] = cases[name];
  }
}
process.stdout.write(JSON.stringify(out, null, 2) + "\n");
