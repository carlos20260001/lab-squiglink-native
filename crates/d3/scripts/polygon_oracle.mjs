// The oracle for d3-polygon: the real d3-polygon, run under Node, writing
// down what it produced.
//
//   node scripts/polygon_oracle.mjs > tests/fixtures/polygon.json
import {writeFileSync} from "fs";
import {createRequire} from "module";

const require = createRequire(import.meta.url);
const root = "/tmp/opencode/d3src/node_modules";
const {polygonArea, polygonCentroid, polygonContains, polygonHull, polygonLength} =
  require(root + "/d3-polygon/src/index.js");

// The numbers are written down as their bits, because a number written in
// text and read back may not be the same number: the reader may round it
// to the nearest one, and the nearest one is not always the one that was
// written. The bits also keep the numbers that JSON cannot hold — the
// `NaN` of a centroid with no area, and the sign of a zero.
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

// A point, as the bits of its two numbers.
const point = p => [bits(p[0]), bits(p[1])];

// A list of points, as their bits.
const points = ps => ps.map(point);

// What a call gives when it throws, which the JavaScript does when the
// polygon has no points in it.
const thrown = f => {
  try {
    return f();
  } catch (error) {
    return {error: `${error.name}: ${error.message}`};
  }
};

const out = {hull: {}, area: {}, centroid: {}, contains: {}, length: {}};

// The hull.
//
// The first three are the ones the JavaScript's own tests use: a list of
// fewer than three points has no hull at all.
out.hull["empty"] = polygonHull([]);
out.hull["one"] = polygonHull([[0, 1]]);
out.hull["two"] = polygonHull([[0, 1], [1, 0]]);
out.hull["triangle"] = points(polygonHull([[200, 200], [760, 300], [500, 500]]));
out.hull["triangleWithInner"] = points(
  polygonHull([[200, 200], [760, 300], [500, 500], [400, 400]]));
out.hull["duplicateOrdinates"] = points(
  polygonHull([[-10, -10], [10, 10], [10, -10], [-10, 10]]));
out.hull["overlapping"] = points(
  polygonHull([[0, -10], [0, 10], [0, 0], [10, 0], [-10, 0]]));

// The three that come from the JavaScript's own tests, which are taken
// from the problem 681 of the Online Judge.
out.hull["uvaFirst"] = points(polygonHull(
  [[60, 20], [250, 140], [180, 170], [79, 140], [50, 60], [60, 20]]));
out.hull["uvaSecond"] = points(polygonHull(
  [[50, 60], [60, 20], [70, 45], [100, 70], [125, 90], [200, 113], [250, 140],
   [180, 170], [105, 140], [79, 140], [60, 85], [50, 60]]));
out.hull["uvaThird"] = points(polygonHull(
  [[30, 30], [50, 60], [60, 20], [70, 45], [86, 39], [112, 60], [200, 113],
   [250, 50], [300, 200], [130, 240], [76, 150], [47, 76], [36, 40], [33, 35],
   [30, 30]]));

// The points that are equal to one another, which the JavaScript keeps
// one of, and which one it keeps depends on the order they came in.
out.hull["equalPoints"] = points(polygonHull([[0, 0], [0, 0], [1, 1]]));
out.hull["equalPointsThree"] = points(polygonHull([[1, 1], [1, 1], [1, 1]]));
out.hull["equalPointsMixed"] = points(
  polygonHull([[0, 0], [0, 0], [0, 0], [1, 0], [0, 1]]));
out.hull["equalPointsApart"] = points(
  polygonHull([[0, 0], [1, 1], [0, 0], [1, 1], [2, 2]]));

// The points that are on a line, which the hull does not keep, for only
// the ends of the line are on it.
out.hull["collinear"] = points(polygonHull([[0, 0], [1, 1], [2, 2]]));
out.hull["collinearWithOne"] = points(
  polygonHull([[0, 0], [1, 1], [2, 2], [3, 3], [4, 4]]));

// The points that share an x, which the JavaScript sorts by the y.
out.hull["sameX"] = points(polygonHull([[0, 0], [0, 1], [0, 2], [1, 1]]));
out.hull["sameXAndY"] = points(
  polygonHull([[0, 0], [0, 1], [1, 1], [1, 0], [0, 0]]));

// The negative zero, which the JavaScript's own coercion keeps.
out.hull["signedZero"] = points(polygonHull([[-0, 0], [0, 0], [1, 1]]));

// A point that is not a number, which the JavaScript's own sort puts
// where its comparison cannot say.
out.hull["withNaN"] = points(polygonHull([[0, 0], [NaN, 1], [1, 1], [2, 2]]));

// A ring of points, which is the hull of itself.
{
  const ring = [];
  for (let i = 0; i < 16; i++) {
    const a = (i / 16) * 2 * Math.PI;
    ring.push([Math.cos(a), Math.sin(a)]);
  }
  out.hull["ring"] = points(polygonHull(ring));
}

// The area.
out.area["empty"] = bits(polygonArea([]));
out.area["one"] = bits(polygonArea([[0, 0]]));
out.area["two"] = bits(polygonArea([[0, 0], [1, 0]]));
out.area["counterclockwise"] = bits(
  polygonArea([[0, 0], [0, 1], [1, 1], [1, 0], [0, 0]]));
out.area["clockwise"] = bits(
  polygonArea([[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]]));
out.area["openCounterclockwise"] = bits(polygonArea([[0, 0], [0, 1], [1, 1], [1, 0]]));
out.area["openClockwise"] = bits(polygonArea([[0, 0], [1, 0], [1, 1], [0, 1]]));
out.area["triangle"] = bits(polygonArea([[1, 1], [3, 2], [2, 3]]));
out.area["withNaN"] = bits(polygonArea([[0, 0], [NaN, 1], [1, 1]]));

// A polygon so large that the sum of its parts is not the whole of it.
{
  const stop = 1e8, step = 1e4;
  const large = [];
  for (let value = 0; value < stop; value += step) large.push([0, value]);
  for (let value = 0; value < stop; value += step) large.push([value, stop]);
  for (let value = stop - step; value >= 0; value -= step) large.push([stop, value]);
  for (let value = stop - step; value >= 0; value -= step) large.push([value, 0]);
  out.area["large"] = bits(polygonArea(large));
  out.centroid["large"] = point(polygonCentroid(large));
}

// The centroid.
out.centroid["empty"] = point(polygonCentroid([]));
out.centroid["counterclockwise"] = point(
  polygonCentroid([[0, 0], [0, 1], [1, 1], [1, 0], [0, 0]]));
out.centroid["clockwise"] = point(
  polygonCentroid([[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]]));
out.centroid["openCounterclockwise"] = point(
  polygonCentroid([[0, 0], [0, 1], [1, 1], [1, 0]]));
out.centroid["openClockwise"] = point(
  polygonCentroid([[0, 0], [1, 0], [1, 1], [0, 1]]));
out.centroid["triangle"] = point(polygonCentroid([[1, 1], [3, 2], [2, 3]]));

// A polygon with no area at all has no centre either.
out.centroid["collinear"] = point(polygonCentroid([[0, 0], [1, 1], [2, 2]]));

// The containment.
{
  const counterclockwise = [[0, 0], [0, 1], [1, 1], [1, 0], [0, 0]];
  const clockwise = [[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]];
  out.contains["inside"] = polygonContains(counterclockwise, [0.5, 0.5]);
  out.contains["outsideRight"] = polygonContains(counterclockwise, [1.5, 0.5]);
  out.contains["outsideLeft"] = polygonContains(counterclockwise, [-0.5, 0.5]);
  out.contains["outsideTop"] = polygonContains(counterclockwise, [0.5, 1.5]);
  out.contains["outsideBottom"] = polygonContains(counterclockwise, [0.5, -0.5]);
  out.contains["onAVertex"] = polygonContains(counterclockwise, [0, 0]);
  out.contains["onAnEdge"] = polygonContains(counterclockwise, [0.5, 0]);
  out.contains["clockwiseInside"] = polygonContains(clockwise, [0.5, 0.5]);
  out.contains["triangleInside"] = polygonContains([[1, 1], [3, 2], [2, 3]], [1.5, 1.5]);
  out.contains["triangleOutside"] = polygonContains([[1, 1], [3, 2], [2, 3]], [0.5, 0.5]);
}

// The perimeter.
out.length["empty"] = thrown(() => polygonLength([]));
out.length["one"] = bits(polygonLength([[0, 0]]));
out.length["two"] = bits(polygonLength([[0, 0], [3, 4]]));
out.length["counterclockwise"] = bits(
  polygonLength([[0, 0], [0, 1], [1, 1], [1, 0], [0, 0]]));
out.length["clockwise"] = bits(
  polygonLength([[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]]));
out.length["openCounterclockwise"] = bits(polygonLength([[0, 0], [0, 1], [1, 1], [1, 0]]));
out.length["openClockwise"] = bits(polygonLength([[0, 0], [1, 0], [1, 1], [0, 1]]));
out.length["triangle"] = bits(polygonLength([[1, 1], [3, 2], [2, 3]]));

writeFileSync(
  process.argv[2] || "tests/fixtures/polygon.json",
  JSON.stringify(out, null, 1) + "\n");
