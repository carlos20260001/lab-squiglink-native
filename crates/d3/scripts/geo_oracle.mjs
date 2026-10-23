import { writeFileSync } from "fs";
import { createRequire } from "module";
const require = createRequire(import.meta.url);
const root = "/tmp/opencode/d3src/node_modules";
const d3 = require(root + "/d3-geo/dist/d3-geo.js");

function bits(value) {
  if (value == null) return null;
  if (Array.isArray(value)) return value.map(bits);
  if (typeof value === "object") {
    const out = {};
    for (const k in value) out[k] = bits(value[k]);
    return out;
  }
  const b = new ArrayBuffer(8);
  const v = new DataView(b);
  v.setFloat64(0, value, false);
  let text = "";
  for (let i = 0; i < 8; i++) text += v.getUint8(i).toString(16).padStart(2, "0");
  return text;
}

const out = {};
const poly = { type: "Polygon", coordinates: [[[0, 0], [10, 0], [10, 10], [0, 10], [0, 0]]] };
const line = { type: "LineString", coordinates: [[0, 0], [10, 0]] };

out.distance = bits(d3.geoDistance([0, 0], [1, 1]));
out.length = bits(d3.geoLength(line));
out.area = bits(d3.geoArea(poly));
const c = d3.geoCentroid(poly);
out.centroid = [bits(c[0]), bits(c[1])];
const b = d3.geoBounds(poly);
out.bounds = [[bits(b[0][0]), bits(b[0][1])], [bits(b[1][0]), bits(b[1][1])]];
out.contains = d3.geoContains(poly, [5, 5]);
const circ = d3.geoCircle().radius(5)([0, 0]);
out.circleType = circ.type;
out.circleCoordsLen = circ.coordinates[0].length;
const g = d3.geoGraticule10();
out.graticuleType = g.type;
const interp = d3.geoInterpolate([0, 0], [10, 10]);
const ip = interp(0.5);
out.interpolate = [bits(ip[0]), bits(ip[1])];
const rot = d3.geoRotation([10, 20, 30]);
const r = rot([0, 0]);
out.rotation = [bits(r[0]), bits(r[1])];
const t = d3.geoTransform({ point: function(x, y) { this.stream.point(x * 2, y * 2); } });
out.transformDefined = t != null;
const id = d3.geoIdentity();
out.identityScale = bits(id.scale());
out.identityTranslate = id.translate().map(bits);

writeFileSync(process.argv[2] || "tests/fixtures/geo.json", JSON.stringify(out, null, 1) + "\n");
