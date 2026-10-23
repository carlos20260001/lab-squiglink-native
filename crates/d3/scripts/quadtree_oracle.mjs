import {writeFileSync} from "fs";
const root = "/tmp/opencode/d3src/node_modules";
import {createRequire} from "module";
const require = createRequire(import.meta.url);
const d3 = require(root + "/d3-quadtree/dist/d3-quadtree.js");

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

const pts = [[0,0],[1,1],[2,2],[0.5,0.5],[3,3],[1,0],[0,1]];
const out = {};

// build tree
const q = d3.quadtree(pts);
out.size = q.size();
out.data = q.data().map(d => [bits(d[0]), bits(d[1])]);
out.extent = q.extent().map(p => p.map(v => bits(v)));
out.rootType = Array.isArray(q.root()) ? "branch" : (q.root() ? "leaf" : "undefined");

// find
out.find00 = q.find(0,0);
out.find00 = out.find00 ? [bits(out.find00[0]), bits(out.find00[1])] : null;
out.findNear = q.find(0.6,0.6, 0.5);
out.findNear = out.findNear ? [bits(out.findNear[0]), bits(out.findNear[1])] : null;

// cover
const q2 = d3.quadtree().x(d=>d[0]).y(d=>d[1]);
q2.cover(5,5).cover(-1,-1);
out.coverExtent = q2.extent().map(p => p.map(v => bits(v)));

// remove
const q3 = d3.quadtree(pts.slice());
q3.remove(pts[1]);
out.sizeAfterRemove = q3.size();
out.dataAfterRemove = q3.data().map(d=>[bits(d[0]),bits(d[1])]);

// visit
const visited = [];
q.visit((node, x0, y0, x1, y1) => {
  visited.push({isLeaf: !Array.isArray(node), x0: bits(x0), y0: bits(y0), x1: bits(x1), y1: bits(y1), size: Array.isArray(node) ? 4 : (node.next? 2:1)});
});
out.visitedCount = visited.length;
out.visited = visited.slice(0,5);

// copy
const qc = q.copy();
out.copySize = qc.size();
out.copyEqual = qc.size() === q.size();

// accessors
const q4 = d3.quadtree().x(d=>d.x).y(d=>d.y);
const arr = [{x:1,y:2},{x:3,y:4}];
q4.addAll(arr);
out.accData = q4.data().map(d=>[bits(d.x),bits(d.y)]);

writeFileSync(process.argv[2] || "tests/fixtures/quadtree.json", JSON.stringify(out, null, 1) + "\n");
