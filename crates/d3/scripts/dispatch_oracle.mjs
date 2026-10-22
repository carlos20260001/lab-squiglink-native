// The oracle: d3-dispatch's own JavaScript, run under Node, writing what
// it produces.
//
//   node scripts/dispatch_oracle.mjs > tests/fixtures/dispatch.json
//
// The scenarios are written down here as steps, and the same steps are in
// the fixture, so that the Rust test replays the very same steps and can
// be compared with what the JavaScript did with them.

import {writeFileSync} from "fs";
import {createRequire} from "module";

const require = createRequire(import.meta.url);
const root = "/tmp/opencode/d3src/node_modules";

const {dispatch} = require(root + "/d3-dispatch/src/index.js");

// The bits of a number, for a number written in text and read back may not
// be the same number.
const buffer = new ArrayBuffer(8);
const view = new DataView(buffer);

function bits(value) {
  view.setFloat64(0, value, false);
  let text = "";
  for (let index = 0; index < 8; index += 1) {
    text += view.getUint8(index).toString(16).padStart(2, "0");
  }
  return text;
}

// The description of an argument, which the port's own is the same as.
function describe(value) {
  if (typeof value === "number") return "n:" + bits(value);
  return "s:" + String(value);
}

// A number of a step, written down as its bits.
function n(value) {
  return {bits: bits(value)};
}

// A string of a step.
function s(text) {
  return {text};
}

// The reading of an argument of a step.
function decode(encoded) {
  if (encoded.bits !== undefined) {
    view.setBigUint64(0, BigInt("0x" + encoded.bits), false);
    return view.getFloat64(0, false);
  }
  return encoded.text;
}

// The steps of a scenario. Each of them is one thing done to a dispatch,
// and the log says what came of it.
const scenarios = {
  // The types the dispatch refuses.
  "the types are refused": [
    {do: "dispatch", as: "d", types: ["start"]},
    {do: "dispatch", as: "d", types: ["start", "end"]},
    {do: "dispatch", as: "d", types: []},
    {do: "dispatch", as: "d", types: ["start", "start"]},
    {do: "dispatch", as: "d", types: ["toString"]},
    {do: "dispatch", as: "d", types: ["constructor"]},
    {do: "dispatch", as: "d", types: ["hasOwnProperty"]},
    {do: "dispatch", as: "d", types: ["isPrototypeOf"]},
    {do: "dispatch", as: "d", types: ["propertyIsEnumerable"]},
    {do: "dispatch", as: "d", types: ["toLocaleString"]},
    {do: "dispatch", as: "d", types: ["valueOf"]},
    {do: "dispatch", as: "d", types: ["__proto__"]},
    {do: "dispatch", as: "d", types: ["__defineGetter__"]},
    {do: "dispatch", as: "d", types: ["__defineSetter__"]},
    {do: "dispatch", as: "d", types: ["__lookupGetter__"]},
    {do: "dispatch", as: "d", types: ["__lookupSetter__"]},
    {do: "dispatch", as: "d", types: ["a b"]},
    {do: "dispatch", as: "d", types: ["a\tb"]},
    {do: "dispatch", as: "d", types: [" start "]},
    {do: "dispatch", as: "d", types: ["a.b"]},
    {do: "dispatch", as: "d", types: [""]},
    {do: "dispatch", as: "d", types: ["0"]},
    {do: "dispatch", as: "d", types: ["start", "end", "middle"]},
  ],

  // A callback is set and called.
  "a callback is called": [
    {do: "dispatch", as: "d", types: ["start", "end"]},
    {do: "on", typename: "start", callback: "a"},
    {do: "on", typename: "end", callback: "b"},
    {do: "call", typename: "start", args: [n(1), s("two")]},
    {do: "call", typename: "end", args: []},
    {do: "apply", typename: "start", args: [n(1), s("two")]},
  ],

  // The callbacks are called in the order they were set in, and a callback
  // that is set again goes to the end.
  "the callbacks are called in order": [
    {do: "dispatch", as: "d", types: ["start"]},
    {do: "on", typename: "start.a", callback: "a"},
    {do: "on", typename: "start.b", callback: "b"},
    {do: "on", typename: "start.a", callback: "a2"},
    {do: "call", typename: "start", args: []},
  ],

  // The callback of a type and a name is given back.
  "the callback is given back": [
    {do: "dispatch", as: "d", types: ["start", "end"]},
    {do: "on", typename: "start", callback: "a"},
    {do: "on", typename: "start.named", callback: "n"},
    {do: "on_get", typename: "start"},
    {do: "on_get", typename: "start.named"},
    {do: "on_get", typename: "start.missing"},
    {do: "on_get", typename: "end"},
    {do: "on_get", typename: "end start"},
    {do: "on_get", typename: "nope"},
  ],

  // A callback is taken away.
  "a callback is taken away": [
    {do: "dispatch", as: "d", types: ["start", "end"]},
    {do: "on", typename: "start", callback: "s"},
    {do: "on", typename: "end", callback: "e"},
    {do: "on_null", typename: "start"},
    {do: "call", typename: "start", args: []},
    {do: "call", typename: "end", args: []},
  ],

  // A named callback is taken away, from one type and from every type.
  "a named callback is taken away": [
    {do: "dispatch", as: "d", types: ["start", "end"]},
    {do: "on", typename: "start.foo", callback: "sf"},
    {do: "on", typename: "end.foo", callback: "ef"},
    {do: "on_null", typename: "start.foo"},
    {do: "call", typename: "start", args: []},
    {do: "call", typename: "end", args: []},
    {do: "on_null", typename: ".foo"},
    {do: "call", typename: "start", args: []},
    {do: "call", typename: "end", args: []},
  ],

  // Several types are given at once.
  "several types at once": [
    {do: "dispatch", as: "d", types: ["start", "end", "middle"]},
    {do: "on", typename: "start end", callback: "se"},
    {do: "call", typename: "start", args: []},
    {do: "call", typename: "end", args: []},
    {do: "call", typename: "middle", args: []},
    {do: "on", typename: "start.foo end.bar", callback: "x"},
    {do: "on", typename: "start.baz", callback: "y"},
    {do: "call", typename: "start", args: []},
    {do: "call", typename: "end", args: []},
  ],

  // The copy of a dispatch is a dispatch of its own.
  "the copy is a copy": [
    {do: "dispatch", as: "d", types: ["start"]},
    {do: "on", typename: "start", callback: "a"},
    {do: "copy", as: "second", from: "d"},
    {do: "use", as: "second"},
    {do: "on", typename: "start", callback: "b"},
    {do: "call", typename: "start", args: []},
    {do: "use", as: "d"},
    {do: "call", typename: "start", args: []},
  ],

  // The types the dispatch does not have.
  "the unknown type": [
    {do: "dispatch", as: "d", types: ["start"]},
    {do: "call", typename: "nope", args: []},
    {do: "apply", typename: "nope", args: []},
    {do: "on", typename: "nope", callback: "x"},
    {do: "on_get", typename: "nope"},
    {do: "on_null", typename: "nope"},
    {do: "on", typename: "start nope", callback: "x"},
    {do: "on", typename: "start. nope", callback: "x"},
    {do: "call", typename: "toString", args: []},
    {do: "on", typename: "toString", callback: "x"},
  ],

  // The arguments and the object the dispatch was called on.
  "the arguments and the context": [
    {do: "dispatch", as: "d", types: ["start"]},
    {do: "on", typename: "start", callback: "a"},
    {do: "call", typename: "start", args: [n(1), n(2.5), s("three")]},
    {do: "apply", typename: "start", args: [n(1), n(2.5), s("three")]},
    {do: "call", typename: "start", args: [], that: "one"},
    {do: "apply", typename: "start", args: [], that: "two"},
  ],

  // The whitespace of a typename.
  "the whitespace of a typename": [
    {do: "dispatch", as: "d", types: ["start", "end"]},
    {do: "on", typename: " start  end ", callback: "se"},
    {do: "call", typename: "start", args: []},
    {do: "call", typename: "end", args: []},
    {do: "on", typename: "\tstart\tend\n", callback: "tab"},
    {do: "call", typename: "start", args: []},
    {do: "call", typename: "end", args: []},
  ],

  // The typename with nothing in it, and the typename with a full stop at
  // the end of it.
  "the typename with nothing in it": [
    {do: "dispatch", as: "d", types: ["start"]},
    {do: "on", typename: "start", callback: "s"},
    {do: "on", typename: "", callback: "x"},
    {do: "on_null", typename: ""},
    {do: "on", typename: ".foo", callback: "y"},
    {do: "call", typename: "start", args: []},
    {do: "on", typename: "start.", callback: "dot"},
    {do: "call", typename: "start", args: []},
    {do: "on", typename: "start..two", callback: "two"},
    {do: "call", typename: "start", args: []},
  ],
};

// The callback of a name, which says what it was called with.
function callbackFor(name, log) {
  return {
    [name]: function (...args) {
      const that = this;
      let line = name + "(" + (that === null ? "t:null" : "t:" + that.tag);
      for (const argument of args) line += "," + describe(argument);
      log.push(line + ")");
    },
  }[name];
}

// The run of the steps of a scenario, which gives back the log.
function run(steps) {
  const log = [];
  const dispatches = {};
  let current = null;

  for (const step of steps) {
    switch (step.do) {
      case "dispatch": {
        try {
          dispatches[step.as] = dispatch(...step.types);
          current = step.as;
          log.push("dispatch " + step.as);
        } catch (error) {
          log.push("dispatch " + step.as + " ERROR " + error.message);
        }
        break;
      }
      case "on": {
        try {
          dispatches[current].on(step.typename, callbackFor(step.callback, log));
          log.push("on " + step.typename + " " + step.callback);
        } catch (error) {
          log.push("on " + step.typename + " ERROR " + error.message);
        }
        break;
      }
      case "on_get": {
        try {
          const found = dispatches[current].on(step.typename);
          log.push("get " + step.typename + " " + (found ? "found" : "none"));
          // The callback that was found is called, so that the log says
          // which one it was.
          if (found) found.call(null);
        } catch (error) {
          log.push("get " + step.typename + " ERROR " + error.message);
        }
        break;
      }
      case "on_null": {
        try {
          dispatches[current].on(step.typename, null);
          log.push("remove " + step.typename);
        } catch (error) {
          log.push("remove " + step.typename + " ERROR " + error.message);
        }
        break;
      }
      case "call":
      case "apply": {
        const that = step.that === undefined ? null : {tag: step.that};
        const args = (step.args || []).map(decode);
        log.push((step.do === "call" ? "call " : "apply ") + step.typename);
        try {
          if (step.do === "call") {
            dispatches[current].call(step.typename, that, ...args);
          } else {
            dispatches[current].apply(step.typename, that, args);
          }
        } catch (error) {
          log.push((step.do === "call" ? "call " : "apply ") + step.typename + " ERROR " + error.message);
        }
        break;
      }
      case "copy": {
        dispatches[step.as] = dispatches[step.from].copy();
        log.push("copy " + step.as);
        break;
      }
      case "use": {
        current = step.as;
        log.push("use " + step.as);
        break;
      }
      default:
        throw new Error("the step " + step.do + " is not known");
    }
  }

  return log;
}

const out = {
  // The names of the properties of the JavaScript's `Object.prototype`,
  // which the dispatch refuses as types.
  prototypeNames: Object.getOwnPropertyNames(Object.prototype),
  scenarios: {},
};

for (const [name, steps] of Object.entries(scenarios)) {
  out.scenarios[name] = {steps, log: run(steps)};
}

writeFileSync(
  process.argv[2] || "tests/fixtures/dispatch.json",
  JSON.stringify(out, null, 1) + "\n");
