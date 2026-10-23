import { writeFileSync } from "fs";
import { createRequire } from "module";
const require = createRequire(import.meta.url);
const root = "/tmp/opencode/d3src/node_modules";
const d3 = require(root + "/d3-hierarchy/dist/d3-hierarchy.js");

function bits(value) {
  const b = new ArrayBuffer(8);
  const v = new DataView(b);
  v.setFloat64(0, value == null ? 0 : value, false);
  let text = "";
  for (let i = 0; i < 8; i++) text += v.getUint8(i).toString(16).padStart(2, "0");
  return text;
}

const data = { name: "root", children: [
  { name: "a", value: 1, children: [{ name: "a1", value: 1 }] },
  { name: "b", value: 2, children: [{ name: "b1", value: 2 }] },
] };
const h = d3.hierarchy(data);
h.sum && h.sum(d => d.value || 1);
h.count && h.count();
const out = {
  descendantsCount: h.descendants ? h.descendants().length : 0,
  leavesCount: h.leaves ? h.leaves().length : 0,
  rootDepth: 0,
  rootHeight: h.height || 0,
};
writeFileSync(process.argv[2] || "tests/fixtures/hierarchy.json", JSON.stringify(out, null, 1) + "\n");
