// The oracle: d3-collection's own JavaScript, run under Node, writing
// what it produces.
//
//   node scripts/collection_oracle.mjs > tests/fixtures/collection.json

import {writeFileSync} from "fs";
import {createRequire} from "module";

const require = createRequire(import.meta.url);
const root = "/tmp/opencode/d3src/node_modules";

function d3(pkg) {
  return require(root + "/" + pkg + "/src/index.js");
}

// The package's own bundle, for its own sources are written for a
// bundler and name their neighbours without the extension.
const d3collection = require(root + "/d3-collection/dist/d3-collection.js");

const out = {map: {}, set: {}, nest: {}};

// The map, which is made of an array or of an object.
{
  const fromArray = d3collection.map([1, 2, 3]);
  out.map["array"] = {
    keys: fromArray.keys(),
    values: fromArray.values(),
    size: fromArray.size(),
    empty: fromArray.empty(),
    has: [fromArray.has(0), fromArray.has(9)],
  };
  const byKey = d3collection.map([1, 2, 3], (d, i) => `k${i}`);
  out.map["byKey"] = {
    keys: byKey.keys(),
    values: byKey.values(),
    entries: byKey.entries().map((entry) => [entry.key, entry.value]),
  };
  const object = d3collection.map({a: 1, b: 2});
  out.map["object"] = {
    keys: object.keys(),
    values: object.values(),
    size: object.size(),
  };
  const set = d3collection.map([1, 2, 3]);
  set.set(0, 10);
  out.map["set"] = {
    keys: set.keys(),
    values: set.values(),
    size: set.size(),
  };
  set.remove(0);
  out.map["removed"] = {
    keys: set.keys(),
    size: set.size(),
  };
  set.clear();
  out.map["cleared"] = {size: set.size(), empty: set.empty()};
}

// The set, which is made of an array.
{
  const set = d3collection.set(["a", "b", "a"]);
  out.set["values"] = set.values();
  out.set["size"] = set.size();
  out.set["has"] = [set.has("a"), set.has("z")];
  const byKey = d3collection.set([1, 2, 3], (d) => `k${d}`);
  out.set["byKey"] = byKey.values();
  set.remove("a");
  out.set["removed"] = set.values();
  set.clear();
  out.set["cleared"] = {size: set.size(), empty: set.empty()};
}

// The nest, which groups the values by their keys.
{
  const values = [
    {year: 2020, month: "jan", value: 1},
    {year: 2020, month: "feb", value: 2},
    {year: 2021, month: "jan", value: 3},
  ];
  const nest = d3collection.nest()
    .key((d) => d.year)
    .key((d) => d.month);
  // The entries of a map, which are the keys and the values.
  const entriesOf = (map) => [...map.entries()].map((e) => [e.key, e.value]);
  // The map of the nest, which is the keys and the values under them.
  const map = nest.map(values);
  out.nest["map"] = JSON.stringify(
    entriesOf(map).map(([key, value]) => [
      key,
      entriesOf(value).map(([key, value]) => [key, value.length]),
    ]));
  // The object of the nest, which is the keys and the values under them.
  const object = nest.object(values);
  out.nest["object"] = JSON.stringify(
    Object.entries(object).map(([key, value]) => [
      key,
      Object.entries(value).map(([key, value]) => [key, value.length]),
    ]));
  // The entries of the nest, which are the keys and the keys under them.
  const entries = nest.entries(values);
  out.nest["entries"] = JSON.stringify(
    entries.map((entry) => [
      entry.key,
      entry.values.map((value) => [value.key, value.values.length]),
    ]));
}

writeFileSync(
  process.argv[2] || "tests/fixtures/collection.json",
  JSON.stringify(out, null, 1) + "\n");
