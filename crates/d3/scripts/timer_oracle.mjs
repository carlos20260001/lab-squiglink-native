// The oracle: d3-timer's own JavaScript, run under Node, writing what it
// produces.
//
//   node scripts/timer_oracle.mjs
//
// Every fixture in the port's `tests/fixtures` is produced by a script like
// this one, so that the Rust is checked against the JavaScript it was
// translated from rather than against an expectation written by hand.
//
// The scenarios are written down here as steps, and the same steps are in
// the fixture, so that the Rust test replays the very same steps and can be
// compared with what the JavaScript did with them.
//
// The JavaScript is d3-timer 1.0.10, the release d3 5.16.0 pins, and the
// copy under `reference/d3-timer/src` is the same code: the oracle checks
// that the two are the same, file by file, and stops if they are not.
//
// The package is installed with:
//
//   npm install d3-timer@1.0.10 --prefix /tmp/opencode/d3src
//
// The JavaScript's clock is `performance.now` and its event loop is the
// browser's own, and neither of them can be told what to do from here. The
// oracle therefore gives the JavaScript a clock of its own and a loop of
// its own, and the port is written against the same two things.

import {copyFileSync, mkdtempSync, readFileSync, writeFileSync} from "fs";
import {createRequire} from "module";
import {tmpdir} from "os";
import {join} from "path";
import {fileURLToPath} from "url";

const require = createRequire(import.meta.url);
const root = "/tmp/opencode/d3src/node_modules";
const packageRoot = join(root, "d3-timer");
const vendoredRoot = fileURLToPath(new URL("../reference/d3-timer/src/", import.meta.url));

// The vendored copy is the authority, and the installed one must be the
// same code.
for (const name of ["index.js", "interval.js", "timeout.js", "timer.js"]) {
  const installed = readFileSync(join(packageRoot, "src", name));
  const vendored = readFileSync(join(vendoredRoot, name));
  if (!installed.equals(vendored)) {
    throw new Error("the installed d3-timer is not the vendored one: " + name);
  }
}

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

// A number of a step, written down as its bits.
function n(value) {
  return {bits: bits(value)};
}

// The reading of a number of a step.
function decode(encoded) {
  if (encoded === null || encoded === undefined) return null;
  view.setBigUint64(0, BigInt("0x" + encoded.bits), false);
  return view.getFloat64(0, false);
}

// The clock of the oracle's own, and the loop of the oracle's own, which
// stand in for the JavaScript's `performance.now` and for the browser's own
// loop.
//
// The loop is told what the JavaScript asks of it, and it does what it is
// told when the scenario says so: a timeout fires when the clock has gone
// past its time, an interval fires when the clock has gone past a whole
// period of it, and a frame fires when the scenario runs one.
const loop = {
  clock: 0,
  timeout: null,
  interval: null,
  frames: [],
  log: [],
};

globalThis.performance.now = () => loop.clock;

globalThis.setTimeout = (callback, delay) => {
  loop.timeout = {due: loop.clock + delay, callback};
  loop.log.push("setTimeout n:" + bits(delay));
  return 1;
};

globalThis.clearTimeout = () => {
  loop.timeout = null;
  loop.log.push("clearTimeout");
  return undefined;
};

globalThis.setInterval = (callback, delay) => {
  loop.interval = {period: delay, last: loop.clock, callback};
  loop.log.push("setInterval n:" + bits(delay));
  return 1;
};

globalThis.clearInterval = () => {
  loop.interval = null;
  loop.log.push("clearInterval");
  return undefined;
};

// The browser's own `requestAnimationFrame`, which the JavaScript's
// `setFrame` is when the browser has one.
globalThis.window = {
  requestAnimationFrame(callback) {
    loop.frames.push(callback);
    loop.log.push("ask frame");
    return loop.frames.length;
  },
};

// The JavaScript's module keeps the timers that are waiting in itself, and
// a scenario wants a module of its own, so the bundle is copied afresh for
// each one.
function fresh() {
  const directory = mkdtempSync(join(tmpdir(), "d3-timer-"));
  const file = join(directory, "d3-timer.js");
  copyFileSync(join(packageRoot, "dist", "d3-timer.js"), file);
  return require(file);
}

// The steps of the scenarios. Each of them is one thing done to the
// timers, and the log says what came of it.
const scenarios = {
  // The clock is read, and the time of the frame is kept until the next
  // frame comes.
  "the clock is read": [
    {do: "set", clock: n(1000)},
    {do: "now"},
    {do: "advance", by: n(5)},
    {do: "now"},
    {do: "frame"},
    {do: "now"},
  ],

  // A timer fires when its time comes, and it goes on firing until it is
  // stopped.
  "a timer fires when its time comes": [
    {do: "set", clock: n(1000)},
    {do: "timer", id: 0, delay: n(100), time: null},
    {do: "flush"},
    {do: "advance", by: n(100)},
    {do: "frame"},
    {do: "advance", by: n(100)},
    {do: "frame"},
  ],

  // A timer with a time of its own is due at that time and not before it.
  "a timer with a time of its own": [
    {do: "set", clock: n(1000)},
    {do: "timer", id: 0, delay: n(0), time: n(2000)},
    {do: "flush"},
    {do: "advance", by: n(1000)},
    {do: "frame"},
  ],

  // A timer with no delay is due at once.
  "a timer with no delay": [
    {do: "set", clock: n(1000)},
    {do: "timer", id: 0, delay: null, time: null},
    {do: "flush"},
  ],

  // A timer whose time has passed is due at once, and is given the time
  // that has passed since it was due.
  "a timer whose time has passed": [
    {do: "set", clock: n(1000)},
    {do: "timer", id: 0, delay: n(-100), time: null},
    {do: "flush"},
  ],

  // A timer that is stopped does not fire, and it is taken out of the list
  // of the timers that are waiting.
  "a timer that is stopped does not fire": [
    {do: "set", clock: n(1000)},
    {do: "timer", id: 0, delay: n(100), time: null},
    {do: "stop", id: 0},
    {do: "advance", by: n(200)},
    {do: "frame"},
    {do: "flush"},
  ],

  // A timer that is stopped and started again is put at the end of the
  // list of the timers that are waiting.
  "a timer that is stopped and started again": [
    {do: "set", clock: n(1000)},
    {do: "timer", id: 0, delay: n(100), time: null},
    {do: "stop", id: 0},
    {do: "frame"},
    {do: "restart", id: 0, delay: n(10), time: null},
    {do: "advance", by: n(20)},
    {do: "frame"},
  ],

  // A timeout fires once, and it is given the time that has passed since
  // it was made.
  "a timeout fires once": [
    {do: "set", clock: n(1000)},
    {do: "timeout", id: 0, delay: n(100), time: null},
    {do: "advance", by: n(100)},
    {do: "frame"},
    {do: "frame"},
  ],

  // A timeout with no delay is due at once.
  "a timeout with no delay": [
    {do: "set", clock: n(1000)},
    {do: "timeout", id: 0, delay: null, time: null},
    {do: "flush"},
  ],

  // An interval goes on firing, and it is given the time that has passed
  // since it was made.
  "an interval goes on firing": [
    {do: "set", clock: n(1000)},
    {do: "interval", id: 0, delay: n(100), time: null},
    {do: "advance", by: n(100)},
    {do: "frame"},
    {do: "advance", by: n(100)},
    {do: "frame"},
    {do: "advance", by: n(100)},
    {do: "frame"},
  ],

  // An interval with no delay is a plain timer.
  "an interval with no delay": [
    {do: "set", clock: n(1000)},
    {do: "interval", id: 0, delay: null, time: null},
    {do: "flush"},
    {do: "flush"},
  ],

  // The timers are called in the order they were made in, and not in the
  // order of their times.
  "the timers are called in the order they were made in": [
    {do: "set", clock: n(1000)},
    {do: "timer", id: 0, delay: n(100), time: null},
    {do: "timer", id: 1, delay: n(50), time: null},
    {do: "advance", by: n(100)},
    {do: "frame"},
  ],

  // A timer that is started again is due at its new time.
  "a timer that is started again": [
    {do: "set", clock: n(1000)},
    {do: "timer", id: 0, delay: n(100), time: null},
    {do: "restart", id: 0, delay: n(50), time: null},
    {do: "advance", by: n(60)},
    {do: "frame"},
  ],

  // A callback that stops another timer stops it before the other timer is
  // called.
  "a callback that stops another timer": [
    {do: "set", clock: n(1000)},
    {do: "timer", id: 0, delay: n(100), time: null, stops: 1},
    {do: "timer", id: 1, delay: n(100), time: null},
    {do: "advance", by: n(100)},
    {do: "frame"},
  ],

  // The flush with no timers in it does nothing but read the clock.
  "the flush with nothing to do": [
    {do: "set", clock: n(1000)},
    {do: "flush"},
  ],

  // A timer that is due within the frame is waited for with a frame, and
  // one that is due after it is waited for with a timeout.
  "the timer that is due within the frame": [
    {do: "set", clock: n(1000)},
    {do: "timer", id: 0, delay: n(24), time: null},
    {do: "frame"},
  ],

  // The timer that is due after the frame.
  "the timer that is due after the frame": [
    {do: "set", clock: n(1000)},
    {do: "timer", id: 0, delay: n(25), time: null},
    {do: "frame"},
  ],

  // The clock's skew is looked at now and again, and a clock that has
  // jumped is read against the time it jumped from.
  "the clock's skew": [
    {do: "set", clock: n(1000)},
    {do: "timer", id: 0, delay: n(10), time: null},
    {do: "frame"},
    {do: "advance", by: n(2000)},
    {do: "now"},
  ],
};

// The callback of a timer, which says what it was called with.
function callbackFor(id, stops, log) {
  return (elapsed) => {
    log.push("call " + id + " n:" + bits(elapsed));
    if (stops !== undefined) {
      const other = timers.get(stops);
      if (other !== undefined) other.stop();
    }
  };
}

// The timers of the run, which the callbacks may stop.
let timers = new Map();

// The run of the steps of a scenario, which gives back the log.
function run(steps) {
  const log = [];
  loop.clock = 0;
  loop.timeout = null;
  loop.interval = null;
  loop.frames = [];
  loop.log = log;
  timers = new Map();
  const d3 = fresh();

  for (const step of steps) {
    switch (step.do) {
      case "set": {
        log.push("set n:" + bits(decode(step.clock)));
        loop.clock = decode(step.clock);
        break;
      }
      case "advance": {
        log.push("advance n:" + bits(decode(step.by)));
        loop.clock += decode(step.by);
        // The timeout, if its time has come.
        if (loop.timeout !== null && loop.timeout.due <= loop.clock) {
          const timeout = loop.timeout;
          loop.timeout = null;
          timeout.callback();
        }
        // The interval, if a whole period of it has gone by.
        if (loop.interval !== null && loop.clock - loop.interval.last >= loop.interval.period) {
          const interval = loop.interval;
          interval.last = loop.clock;
          interval.callback();
        }
        break;
      }
      case "timer": {
        log.push("timer " + step.id);
        const timer = d3.timer(
          callbackFor(step.id, step.stops, log),
          decode(step.delay),
          decode(step.time),
        );
        timers.set(step.id, timer);
        break;
      }
      case "timeout": {
        log.push("timeout " + step.id);
        const timer = d3.timeout(
          callbackFor(step.id, step.stops, log),
          decode(step.delay),
          decode(step.time),
        );
        timers.set(step.id, timer);
        break;
      }
      case "interval": {
        log.push("interval " + step.id);
        const timer = d3.interval(
          callbackFor(step.id, step.stops, log),
          decode(step.delay),
          decode(step.time),
        );
        timers.set(step.id, timer);
        break;
      }
      case "restart": {
        log.push("restart " + step.id);
        timers
          .get(step.id)
          .restart(callbackFor(step.id, step.stops, log), decode(step.delay), decode(step.time));
        break;
      }
      case "stop": {
        log.push("stop " + step.id);
        timers.get(step.id).stop();
        break;
      }
      case "flush": {
        log.push("flush");
        d3.timerFlush();
        break;
      }
      case "frame": {
        log.push("frame");
        // The browser's own frames run every callback that was asked for
        // before the frame came.
        const frames = loop.frames;
        loop.frames = [];
        for (const callback of frames) callback();
        break;
      }
      case "now": {
        log.push("now n:" + bits(d3.now()));
        break;
      }
      default:
        throw new Error("the step " + step.do + " is not known");
    }
  }

  return log;
}

const out = {scenarios: {}};

for (const [name, steps] of Object.entries(scenarios)) {
  out.scenarios[name] = {steps, log: run(steps)};
}

// The scenarios the oracle makes for itself, to try the port with more
// steps than the ones written down by hand.
//
// The source is the oracle's own, so that the scenarios are the same every
// time the oracle is run.
function lcg(seed) {
  let state = seed >>> 0;
  return () => {
    state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
    return state / 4294967296;
  };
}

// A number of a step that may be nothing at all.
function maybe(value) {
  return value === null ? null : n(value);
}

// The scenarios of the oracle's own making.
function fuzz(count) {
  const random = lcg(20240607);
  const delays = [null, 0, 1, 10, 24, 25, 50, 100, -100, 1000];
  const times = [null, 0, 500, 1000, 2000, 5000];
  const out = [];
  for (let index = 0; index < count; index += 1) {
    const steps = [];
    const made = [];
    const length = 3 + Math.floor(random() * 8);
    for (let step = 0; step < length; step += 1) {
      const what = Math.floor(random() * 11);
      const id = Math.floor(random() * 3);
      if (what === 0) {
        steps.push({do: "set", clock: n(Math.floor(random() * 3000))});
      } else if (what === 1) {
        steps.push({do: "advance", by: n(Math.floor(random() * 300))});
      } else if (what < 5) {
        const kind = ["timer", "timeout", "interval"][what - 2];
        steps.push({
          do: kind,
          id,
          delay: maybe(delays[Math.floor(random() * delays.length)]),
          time: maybe(times[Math.floor(random() * times.length)]),
        });
        made.push(id);
      } else if (what === 5 && made.length > 0) {
        steps.push({do: "stop", id: made[Math.floor(random() * made.length)]});
      } else if (what === 6 && made.length > 0) {
        const step = {
          do: "restart",
          id: made[Math.floor(random() * made.length)],
          delay: maybe(delays[Math.floor(random() * delays.length)]),
          time: maybe(times[Math.floor(random() * times.length)]),
        };
        if (made.length > 1 && random() < 0.3) {
          step.stops = made[Math.floor(random() * made.length)];
        }
        steps.push(step);
      } else if (what === 7) {
        steps.push({do: "flush"});
      } else if (what === 8) {
        steps.push({do: "frame"});
      } else if (what === 9) {
        steps.push({do: "now"});
      } else {
        steps.push({do: "flush"});
      }
    }
    out.push({steps, log: run(steps)});
  }
  return out;
}

out.fuzz = fuzz(150);

writeFileSync(
  process.argv[2] || "tests/fixtures/timer.json",
  JSON.stringify(out, null, 1) + "\n");
