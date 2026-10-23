// The oracle: d3-scale's own JavaScript, run under Node, writing what it
// produces.
//
//   node scripts/scale_oracle.mjs > tests/fixtures/scale.json

import {writeFileSync} from "fs";
import {createRequire} from "module";

const require = createRequire(import.meta.url);
const root = "/tmp/opencode/d3src/node_modules";

const scale = require("/tmp/opencode/d3src/node_modules/d3-scale/dist/d3-scale.js");

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
function bn(v){ return bits(Number(v)); }

const out = {};

// Helper to evaluate a scale at samples
function evalScale(s, samples) {
  const res = [];
  for (const x of samples) res.push(bn(s(x)));
  return res;
}

const samples = [0, 0.25, 0.5, 0.75, 1, 2, 5, 10, -1, -0.5, NaN];

// Linear
{
  const s = scale.scaleLinear().domain([0,10]).range([0,1]);
  out.linear = {
    call: evalScale(s, samples),
    domain: s.domain().map(bn),
    range: s.range().map(bn),
    invert: [0,0.5,1].map(v=>bn(s.invert(v))),
    ticks: s.ticks(5).map(bn),
    tickFormat: s.ticks(5).map(d=>s.tickFormat(5)(d)),
  };
  const s2 = scale.scaleLinear().domain([0,10]).range([0,1]).clamp(true);
  out.linearClamp = evalScale(s2, [-5, 5, 15]);
}

// Identity
{
  const s = scale.scaleIdentity().domain([0,10]);
  out.identity = {
    call: evalScale(s, samples),
    domain: s.domain().map(bn),
    range: s.range().map(bn),
    invert: [0,5,10].map(v=>bn(s.invert(v))),
  };
}

// Log
{
  const s = scale.scaleLog().domain([1,100]).range([0,1]);
  out.log = {
    base: bn(s.base()),
    call: evalScale(s, [1,2,10,50,100]),
    ticks: s.ticks(5).map(bn),
  };
}

// Symlog
{
  const s = scale.scaleSymlog().constant(2).domain([-10,10]).range([0,1]);
  out.symlog = {
    constant: bn(s.constant()),
    call: evalScale(s, [-10,-5,0,5,10]),
  };
}

// Pow
{
  const s = scale.scalePow().exponent(2).domain([0,10]).range([0,1]);
  out.pow = {
    exponent: bn(s.exponent()),
    call: evalScale(s, [0,2,5,10]),
    sqrt: evalScale(scale.scaleSqrt().domain([0,10]).range([0,1]), [0,4,9]),
  };
}

// Band
{
  const s = scale.scaleBand().domain([0,1,2]).range([0,300]);
  out.band = {
    call: [0,1,2,3].map(d=> (s(d)===undefined? "undef" : bn(s(d)))),
    bandwidth: bn(s.bandwidth()),
    step: bn(s.step()),
  };
  const p = scale.scalePoint().domain([0,1,2]).range([0,300]);
  out.point = {
    bandwidth: bn(p.bandwidth()),
    step: bn(p.step()),
  };
}

// Ordinal
{
  const s = scale.scaleOrdinal().domain([0,1]).range([1,2]);
  // trigger implicit
  s(2);
  out.ordinal = {
    call: [0,1,2,3].map(d=>bn(s(d))),
    domain: s.domain().map(v=> (typeof v === "number" ? bn(v) : v)),
    unknownImplicit: s.unknown()===scale.scaleImplicit,
  };
  const s2 = scale.scaleOrdinal().domain(["a"]).range([10]).unknown(5);
  out.ordinalUnknown = { call: bn(s2("z")) };
}

// Quantile
{
  const s = scale.scaleQuantile().domain([1,2,3,4,5,6]).range([10,20,30]);
  out.quantile = {
    call: [1,3.5,6].map(v=>s(v)),
    quantiles: s.quantiles().map(bn),
    invertExtent: s.invertExtent("q1"),
  };
}
// store quantile outputs as strings not bits
out.quantile.call = [1,3.5,6].map(v=>bn(scale.scaleQuantile().domain([1,2,3,4,5,6]).range([10,20,30])(v)));

// Quantize
{
  const s = scale.scaleQuantize().domain([0,10]).range([1,2,3]);
  out.quantize = {
    call: [0,3,7,10].map(v=>bn(s(v))),
    thresholds: s.thresholds().map(bn),
    invertExtent: s.invertExtent("b"),
  };
}

// Threshold
{
  const s = scale.scaleThreshold().domain([0,5]).range([1,2,3]);
  out.threshold = {
    call: [-1,0,3,5,10].map(v=>bn(s(v))),
    invertExtent: s.invertExtent("b"),
  };
}

// Time
{
  const s = scale.scaleTime().domain([new Date(2000,0,1), new Date(2000,0,2)]);
  out.time = {
    call: bn(s(new Date(2000,0,1,12))),
    ticks: s.ticks(3).map(d=>bn(d.getTime())),
    tickFormat: s.ticks(3).map(d=>s.tickFormat(3)(d)),
  };
}

// Sequential
{
  const s = scale.scaleSequential(d=>d).domain([0,10]);
  out.sequential = {
    call: evalScale(s, [0,5,10]),
  };
  const sl = scale.scaleSequentialLog().domain([1,100]);
  out.sequentialLog = { call: evalScale(sl, [1,10,100]) };
}

// SequentialQuantile
{
  const s = scale.scaleSequentialQuantile(d=>d*2).domain([1,2,3,4]);
  out.sequentialQuantile = {
    call: [1,2.5,4].map(v=>bn(s(v))),
  };
}

// Diverging
{
  const s = scale.scaleDiverging(d=>d).domain([0,0.5,1]);
  out.diverging = { call: evalScale(s, [0,0.25,0.5,0.75,1]) };
}

// tickFormat
out.tickFormat = {
  basic: scale.tickFormat(0,10,5,",.1f")(3.14159),
  si: scale.tickFormat(0,1000,5,"s")(1234),
};

writeFileSync(process.argv[2] || "tests/fixtures/scale.json", JSON.stringify(out, null, 1) + "\n");
