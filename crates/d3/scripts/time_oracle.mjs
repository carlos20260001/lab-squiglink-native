// The oracle: d3-time's own JavaScript, run under Node, writing what it
// produces.
//
//   node scripts/time_oracle.mjs > tests/fixtures/time.json

import {writeFileSync} from "fs";
import {createRequire} from "module";

const require = createRequire(import.meta.url);
const root = "/tmp/opencode/d3src/node_modules";

function d3(pkg) {
  return require(root + "/" + pkg + "/src/index.js");
}

const d3time = d3("d3-time");

// The dates the intervals are given.
const dates = [
  new Date(Date.UTC(2020, 0, 1)),
  new Date(Date.UTC(2020, 5, 15, 12, 30, 45, 123)),
  new Date(Date.UTC(2021, 11, 31, 23, 59, 59, 999)),
  new Date(Date.UTC(1999, 6, 4, 6, 5, 4)),
];

// The intervals, and the names they are known by.
const intervals = {
  timeMillisecond: d3time.timeMillisecond,
  timeSecond: d3time.timeSecond,
  timeMinute: d3time.timeMinute,
  timeHour: d3time.timeHour,
  timeDay: d3time.timeDay,
  timeWeek: d3time.timeWeek,
  timeSunday: d3time.timeSunday,
  timeMonday: d3time.timeMonday,
  timeMonth: d3time.timeMonth,
  timeYear: d3time.timeYear,
  utcMillisecond: d3time.utcMillisecond,
  utcSecond: d3time.utcSecond,
  utcMinute: d3time.utcMinute,
  utcHour: d3time.utcHour,
  utcDay: d3time.utcDay,
  utcWeek: d3time.utcWeek,
  utcSunday: d3time.utcSunday,
  utcMonday: d3time.utcMonday,
  utcMonth: d3time.utcMonth,
  utcYear: d3time.utcYear,
};

const out = {floor: {}, offset: {}, range: {}, count: {}, local: {}};

// The dates the intervals are given.
out.dates = dates.map((date) => date.getTime());

for (const [name, interval] of Object.entries(intervals)) {
  out.floor[name] = dates.map((date) => interval.floor(date).getTime());
  out.offset[name] = dates.map((date) => interval.offset(date, 3).getTime());
  // The range is a second long, for the interval of a millisecond would
  // otherwise be a million dates long.
  out.range[name] = interval
    .range(dates[0], new Date(dates[0].getTime() + 1000), 1)
    .map((date) => date.getTime());
  out.count[name] = interval.count(dates[0], dates[2]);
}

// The parts of a date, which the calendar has.
out.local["fields"] = dates.map((date) => [
  date.getFullYear(),
  date.getMonth(),
  date.getDate(),
  date.getDay(),
  date.getHours(),
  date.getMinutes(),
  date.getSeconds(),
  date.getMilliseconds(),
]);

// The offset of the timezone, which the system has.
out.local["offset"] = dates.map((date) => date.getTimezoneOffset());

writeFileSync(
  process.argv[2] || "tests/fixtures/time.json",
  JSON.stringify(out, null, 1) + "\n");
