import { writeFileSync } from "fs";
import { createRequire } from "module";

// A fixed linear congruential generator, which stands in for Math.random so
// that the jiggle values are the same every time. The seed is 1, and the
// first call advances the state first, as the Rust port's own generator does.
let state = 1;
Math.random = () => {
  state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
  return state / 4294967296;
};

const require = createRequire(import.meta.url);
const d3 = require("/tmp/opencode/d3src/node_modules/d3-force/dist/d3-force.js");

const buffer = new ArrayBuffer(8);
const view = new DataView(buffer);
function bits(value) {
  if (value === undefined || value === null) value = 0;
  view.setFloat64(0, value, false);
  let text = "";
  for (let i = 0; i < 8; i++) {
    text += view.getUint8(i).toString(16).padStart(2, "0");
  }
  return text;
}

function nodeOut(n) {
  return {
    x: bits(n.x === undefined ? 0 : n.x),
    y: bits(n.y === undefined ? 0 : n.y),
    vx: bits(n.vx === undefined ? 0 : n.vx),
    vy: bits(n.vy === undefined ? 0 : n.vy),
  };
}

const out = {};

// Center
state = 1;
{
  const nodes = [{ x: 1, y: 2, vx: 0, vy: 0 }, { x: 3, y: 4, vx: 0, vy: 0 }, { x: 5, y: 6, vx: 0, vy: 0 }];
  const f = d3.forceCenter(0, 0);
  nodes.forEach((n, i) => { n.index = i; });
  f.initialize(nodes);
  f(1);
  out.center = nodes.map(nodeOut);
}

// X
state = 1;
{
  const nodes = [{ x: 10, y: 5, vx: 0, vy: 0 }, { x: 2, y: 8, vx: 0, vy: 0 }];
  const f = d3.forceX(4);
  nodes.forEach((n, i) => { n.index = i; });
  f.initialize(nodes);
  f(0.5);
  out.x = nodes.map(nodeOut);
}

// Y
state = 1;
{
  const nodes = [{ x: 10, y: 5, vx: 0, vy: 0 }, { x: 2, y: 8, vx: 0, vy: 0 }];
  const f = d3.forceY(3);
  nodes.forEach((n, i) => { n.index = i; });
  f.initialize(nodes);
  f(0.5);
  out.y = nodes.map(nodeOut);
}

// Many-body
state = 1;
{
  const nodes = [{ x: 0, y: 0, vx: 0, vy: 0 }, { x: 1, y: 0, vx: 0, vy: 0 }, { x: 0, y: 1, vx: 0, vy: 0 }];
  const f = d3.forceManyBody();
  nodes.forEach((n, i) => { n.index = i; });
  f.initialize(nodes);
  f(0.1);
  out.manyBody = nodes.map(nodeOut);
}

// Collide
state = 1;
{
  const nodes = [{ x: 0, y: 0, vx: 0, vy: 0 }, { x: 0.5, y: 0, vx: 0, vy: 0 }];
  const f = d3.forceCollide(1);
  nodes.forEach((n, i) => { n.index = i; });
  f.initialize(nodes);
  f();
  out.collide = nodes.map(nodeOut);
}

// Link
state = 1;
{
  const nodes = [{ x: 0, y: 0, vx: 0, vy: 0 }, { x: 10, y: 0, vx: 0, vy: 0 }];
  const links = [{ source: 0, target: 1 }];
  const f = d3.forceLink(links);
  nodes.forEach((n, i) => { n.index = i; });
  f.initialize(nodes);
  f(0.5);
  out.link = nodes.map(nodeOut);
  out.linkLinks = links.map((l) => ({ source: l.source.index ?? l.source, target: l.target.index ?? l.target }));
}

// Radial
state = 1;
{
  const nodes = [{ x: 1, y: 1, vx: 0, vy: 0 }, { x: 2, y: 2, vx: 0, vy: 0 }];
  const f = d3.forceRadial(5, 0, 0);
  nodes.forEach((n, i) => { n.index = i; });
  f.initialize(nodes);
  f(0.2);
  out.radial = nodes.map(nodeOut);
}

// Simulation
state = 1;
{
  const nodes = [{ x: 0, y: 0, vx: 0, vy: 0 }, { x: 10, y: 0, vx: 0, vy: 0 }, { x: 0, y: 10, vx: 0, vy: 0 }];
  const sim = d3.forceSimulation(nodes)
    .force("center", d3.forceCenter(5, 5))
    .force("x", d3.forceX(5))
    .force("charge", d3.forceManyBody().strength(-10));
  sim.tick(5);
  out.simulation = nodes.map(nodeOut);
  out.simulationAlpha = bits(sim.alpha());
}

// Jiggle
state = 1;
// (no force has triggered it, so the stream is still at its start)
{
  const values = [];
  for (let i = 0; i < 5; i++) {
    values.push(bits((Math.random() - 0.5) * 1e-6));
  }
  out.jiggle = values;
}

writeFileSync(process.argv[2], JSON.stringify(out, null, 1) + "\n");
