import {writeFileSync} from "fs";
const root = "/tmp/opencode/d3src/node_modules";
import {createRequire} from "module";
const require = createRequire(import.meta.url);
const d3 = require(root + "/d3-voronoi/build/d3-voronoi.js");

function bits(value) {
  if (typeof value === "number") {
    const b = new ArrayBuffer(8);
    const v = new DataView(b);
    v.setFloat64(0, value, false);
    let text = "";
    for (let i = 0; i < 8; i++) {
      text += v.getUint8(i).toString(16).padStart(2, "0");
    }
    return text;
  }
  return value;
}

function pt(p) { return p == null ? null : [bits(p[0]), bits(p[1])]; }

function pts(arr) { return arr.map(p => pt(p)); }

const out = {};

// basic no extent
const data = [[0,0],[1,0],[0,1],[1,1],[0.5,0.5]];
const v = d3.voronoi();
const dia = v(data);
out.basic = {
  edges: dia.edges.map(e => e == null ? null : [pt(e[0]), pt(e[1]), pt(e.left), pt(e.right)]),
  cells: dia.cells.map(c => c == null ? null : {site: pt(c.site), halfedges: c.halfedges.slice()}),
  polygons: dia.polygons().map(p => p == null ? null : {points: pts(p), data: pt(p.data)}),
  triangles: dia.triangles().map(t => t.map(pt)),
  links: dia.links().map(l => ({source: pt(l.source), target: pt(l.target)})),
  find00: pt(dia.find(0,0)),
  findFar: pt(dia.find(10,10,1)),
  findNull: dia.find(10,10,1) == null ? null : "x",
};

// extent
const ve = d3.voronoi().extent([[0,0],[2,2]]);
const diae = ve(data);
out.extent = {
  polygons: diae.polygons().map(p => p == null ? null : pts(p)),
  triangles: diae.triangles().map(t => t.map(pt)),
  links: diae.links().map(l => ({source: pt(l.source), target: pt(l.target)})),
  findNear: pt(diae.find(0.1,0.1)),
  findFar: diae.find(10,10,1) == null ? null : "x",
};

// size
const vs = d3.voronoi().size([2,2]);
const dias = vs(data);
out.size = {
  extent: vs.extent() ? vs.extent().map(p => p.map(bits)) : null,
  polygonsCount: dias.polygons().length,
};

// x/y constant
const vc = d3.voronoi().x(5).y(7);
const datac = [[0,0],[1,0]];
const diac = vc(datac);
out.constant = {
  polygons: diac.polygons().map(p => p == null ? null : pts(p)),
};

// custom accessor
const va = d3.voronoi().x(d => d.a).y(d => d.b);
const dataa = [{a:0,b:0},{a:2,b:0},{a:1,b:2}];
const diaa = va(dataa);
out.accessor = {
  polygons: diaa.polygons().map(p => p == null ? null : {points: pts(p), data: [bits(p.data.a), bits(p.data.b)]}),
  triangles: diaa.triangles().map(t => t.map(d => [bits(d.a), bits(d.b)])),
  links: diaa.links().map(l => ({source: [bits(l.source.a), bits(l.source.b)], target: [bits(l.target.a), bits(l.target.b)]})),
};

// single point
const single = [[5,5]];
const dias1 = d3.voronoi().extent([[0,0],[10,10]])(single);
out.single = {
  polygons: dias1.polygons().map(p => p == null ? null : pts(p)),
  triangles: dias1.triangles(),
  links: dias1.links(),
};

// duplicates
const dup = [[0,0],[0,0],[1,1]];
const diad = d3.voronoi().extent([[0,0],[2,2]])(dup);
out.duplicates = {
  polygons: diad.polygons().map(p => p == null ? null : pts(p)),
  triangles: diad.triangles().map(t => t.map(pt)),
};

// empty
const empty = [];
const dia0 = d3.voronoi()(empty);
out.empty = {
  edges: dia0.edges.length,
  cells: dia0.cells.length,
  polygons: dia0.polygons().length,
  triangles: dia0.triangles().length,
  links: dia0.links().length,
};

// getters
out.getters = {
  x: (d3.voronoi().x(d=>d[0]).x() ? "function" : "x"),
  extentNull: d3.voronoi().extent() ? "some" : "null",
  sizeNull: d3.voronoi().size() ? "some" : "null",
};

writeFileSync(process.argv[2], JSON.stringify(out, null, 1) + "\n");
