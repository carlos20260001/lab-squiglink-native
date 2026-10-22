// The oracle: d3-time-format's own JavaScript, run under Node, writing
// what it produces.
//
//   node scripts/time_format_oracle.mjs > tests/fixtures/time_format.json
//
// The package's own bundle is used, for the sources name their
// neighbours without the `.js` extension and are meant for a bundler.

import {writeFileSync} from "fs";
import {createRequire} from "module";

const require = createRequire(import.meta.url);
const root = "/tmp/opencode/d3src/node_modules";

const timeFormat = require(root + "/d3-time-format/dist/d3-time-format.js");

// The dates the formats and the parses are given.
//
// The dates are in Greenwich, for the formats that read the local time
// read them in the timezone of the machine that runs this.
const dates = [
  new Date(Date.UTC(1970, 0, 1)),
  new Date(Date.UTC(2020, 0, 1)),
  new Date(Date.UTC(2020, 1, 29, 12)),
  new Date(Date.UTC(2020, 2, 7, 23, 30)),
  new Date(Date.UTC(2020, 5, 15, 12, 30, 45, 123)),
  new Date(Date.UTC(2021, 0, 1, 0, 59, 59, 999)),
  new Date(Date.UTC(2021, 11, 31, 23, 59, 59, 999)),
  new Date(Date.UTC(1999, 6, 4, 6, 5, 4)),
  new Date(-1),
];

// The dates of the old times.
//
// The JavaScript's own timezone keeps the seconds of the offsets that
// were not of whole minutes, and the port's own keeps only the minutes
// of them, and so the local reading of these dates is a little apart
// between the two. The dates are given to the formats that read the
// time in Greenwich alone.
const oldDates = [];

// A year under a hundred, which the JavaScript's own constructor reads
// as a year of the calendar and not as a year of nineteen hundred.
{
  const year = new Date(Date.UTC(2000, 0, 1));
  year.setUTCFullYear(50);
  oldDates.push(year);
}

// The specifications the formats are made for.
//
// The last one holds every directive, and the ones the JavaScript does
// not know, which it writes as they are.
const specifiers = [
  "%a", "%A", "%b", "%B", "%c", "%d", "%e", "%f", "%g", "%G", "%H", "%I",
  "%j", "%L", "%m", "%M", "%p", "%q", "%Q", "%s", "%S", "%u", "%U", "%V",
  "%w", "%W", "%x", "%X", "%y", "%Y", "%Z", "%%",
  "%-d", "%_d", "%0d", "%-m", "%-e", "%-H", "%-I", "%-M", "%-S", "%-j",
  "%-L", "%-y", "%-Y", "%-B", "%-p",
  "%Y-%m-%dT%H:%M:%S.%LZ", "%A, %B %-d, %Y", "%x %X", "%c",
  "a %A b %B c %c d %d e %e f %f g %g G %G h %H i %I j %j k %L l %m m %M n %p o %p p %q Q %Q r %s S %S t %u u %U v %V w %w W %W x %x X %X y %y Y %Y z %Z",
];

// The specifications the parses are made for, and the texts they are
// given.
//
// The texts of the local parse are given a year of their own, for the
// local reading of a date of the year of nineteen hundred is one of
// the old ones, and the local reading of those is the one the two
// timezone databases do not quite agree upon.
const parses = {
  "%Y-%m-%d": ["2020-01-02", "2020-1-2", "20200102", "2020-01-02 extra", "not a date", ""],
  "%Y-%m-%dT%H:%M:%S.%L": [
    "2020-01-02T03:04:05.006",
    "2020-01-02T03:04:05.6",
    "2020-01-02T03:04:05",
  ],
  "%Y-%m-%dT%H:%M:%S.%L%Z": [
    "2020-01-02T03:04:05.006Z",
    "2020-01-02T03:04:05.006+05:30",
    "2020-01-02T03:04:05.006-08:00",
    "2020-01-02T03:04:05.006+0530",
    "2020-01-02T03:04:05.006Z extra",
  ],
  "%d/%m/%Y": ["02/01/2020", "2/1/2020"],
  "%m/%d/%Y": ["01/02/2020", "13/01/2020"],
  "%b %d, %Y": ["Jan 02, 2020", "January 02, 2020", "Jan 2, 2020"],
  "%B %d, %Y": ["January 02, 2020", "Jan 02, 2020"],
  "%a %b %d %Y": ["Thu Jan 02 2020", "Thursday Jan 02 2020"],
  "%A %B %d %Y": ["Thursday January 02 2020", "Thu Jan 02 2020"],
  "%Y %H:%M:%S": ["2020 03:04:05", "2020 3:04:05", "2020 03:04"],
  "%Y %I:%M %p": ["2020 03:04 PM", "2020 03:04 am", "2020 12:00 AM"],
  "%Y %j": ["2020 002", "2020 32", "2020 366"],
  "%Y %U": ["2020 00", "2020 01", "2020 52"],
  "%Y %W": ["2020 00", "2020 01", "2020 52"],
  "%Y %V": ["2020 01", "2021 53", "2020 00"],
  "%G-W%V-%u": ["2020-W01-4", "2021-W53-5"],
  "%G-%V-%u": ["2020-01-4", "2021-53-5"],
  "%Y %q": ["2020 1", "2020 4", "2020 0"],
  "%Y %Q": ["2020 1577836800000"],
  "%Y %s": ["2020 1577836800"],
  "%Q": ["1577836800000", "99999999999999999999"],
  "%s": ["1577836800", "99999999999999999999"],
  "%Y %Z": ["2020 Z", "2020 +05:30", "2020 -0800", "2020 +05", "2020 Z+05:30"],
  "%Y %p": ["2020 AM", "2020 PM", "2020 am", "2020 pm"],
  "%Y %c": ["2020 1/2/2020, 3:04:05 AM", "2020 01/02/2020, 03:04:05 AM"],
  "%Y %x": ["2020 1/2/2020", "2020 01/02/2020"],
  "%Y %X": ["2020 3:04:05 AM", "2020 03:04:05 AM"],
  "%Y %%": ["2020 %", "2020 %%"],
  "%Y %y": ["2020 20", "2020 68", "2020 69", "2020 99"],
  "%Y %Y": ["2020 2020", "2020 1969", "2020 2038"],
  "%Y %d": ["2020 00", "2020 02", "2020 32", "2020 99"],
  "%Y %m": ["2020 00", "2020 01", "2020 13"],
  "%Y %L": ["2020 000", "2020 6", "2020 006"],
  "%Y %f": ["2020 006000", "2020 6000", "2020 006"],
};

// The texts of the parse in Greenwich alone, which have no year of
// their own and so are of the year of nineteen hundred, and the texts
// of the years under a hundred.
const utcOnly = {
  "%H:%M:%S": ["03:04:05", "3:04:05", "03:04"],
  "%I:%M %p": ["03:04 PM", "03:04 am", "12:00 AM"],
  "%Z": ["Z", "+05:30", "-0800", "+05", "Z+05:30"],
  "%p": ["AM", "PM", "am", "pm"],
  "%X": ["3:04:05 AM", "03:04:05 AM"],
  "%%": ["%", "%%"],
  "%Y %Y": ["0050", "50", "999"],
  "%Y": ["2020", "50", "0050", "999"],
  "%d": ["00", "02", "32", "99"],
  "%m": ["00", "01", "13"],
  "%L": ["000", "6", "006"],
  "%f": ["006000", "6000", "006"],
};

// The locale of one's own, which the JavaScript's own takes as an
// object.
const definition = {
  dateTime: "%x %X",
  date: "%d.%m.%Y",
  time: "%H:%M",
  periods: ["am", "pm"],
  days: [
    "dimanche", "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi",
  ],
  shortDays: ["di", "lu", "ma", "me", "je", "ve", "sa"],
  months: [
    "janvier", "février", "mars", "avril", "mai", "juin", "juillet",
    "août", "septembre", "octobre", "novembre", "décembre",
  ],
  shortMonths: [
    "jan", "fév", "mar", "avr", "mai", "jui", "jul", "aoû", "sep", "oct", "nov", "déc",
  ],
};

// The texts the locale's own parses are given.
const localeParses = {
  "%Y %d.%m.%Y": ["2020 02.01.2020", "2020 2.1.2020"],
  "%Y %H:%M": ["2020 03:04", "2020 3:04"],
  "%Y %d.%m.%Y %H:%M": ["2020 02.01.2020 03:04"],
  "%Y %a %d %b %Y": ["2020 je 02 jan 2020", "2020 jeudi 02 janvier 2020"],
};

const out = {format: {}, utcFormat: {}, parse: {}, utcParse: {}};

// The dates the formats and the parses are given.
out.dates = dates.map((date) => date.getTime());
out.oldDates = oldDates.map((date) => date.getTime());

// The formats.
for (const specifier of specifiers) {
  const format = timeFormat.timeFormat(specifier);
  out.format[specifier] = dates.map((date) => format(date));
  const utc = timeFormat.utcFormat(specifier);
  out.utcFormat[specifier] = [...dates, ...oldDates].map((date) => utc(date));
}

// The parses.
//
// A parse that does not fit gives null, and a date that is not a date
// at all gives a number that is not a number; the fixture writes the
// first as null and the second as a word.
const parsed = (parse, string) => {
  const date = parse(string);
  if (date === null) return null;
  if (isNaN(date.getTime())) return "Invalid";
  return date.getTime();
};

const texts = (specifier) => parses[specifier] || [];

for (const specifier of Object.keys(parses)) {
  const parse = timeFormat.timeParse(specifier);
  out.parse[specifier] = {};
  for (const string of parses[specifier]) {
    out.parse[specifier][string] = parsed(parse, string);
  }
  const utc = timeFormat.utcParse(specifier);
  out.utcParse[specifier] = {};
  for (const string of texts(specifier)) {
    out.utcParse[specifier][string] = parsed(utc, string);
  }
}

// The parses that are only in Greenwich, which have no year of their
// own or a year of the old times.
for (const specifier of Object.keys(utcOnly)) {
  const utc = timeFormat.utcParse(specifier);
  out.utcParse[specifier] = out.utcParse[specifier] || {};
  for (const string of utcOnly[specifier]) {
    out.utcParse[specifier][string] = parsed(utc, string);
  }
}

// The locale of one's own.
{
  const locale = timeFormat.timeFormatLocale(definition);
  const specifier = "%A %d %B %Y %X %x %c %p";
  out.locale = {
    format: dates.map((date) => locale.format(specifier)(date)),
    utcFormat: [...dates, ...oldDates].map((date) => locale.utcFormat(specifier)(date)),
  };
  out.locale.parse = {};
  out.locale.utcParse = {};
  for (const [specifier, list] of Object.entries(localeParses)) {
    out.locale.parse[specifier] = {};
    for (const string of list) {
      out.locale.parse[specifier][string] = parsed(locale.parse(specifier), string);
    }
    out.locale.utcParse[specifier] = {};
    for (const string of list) {
      out.locale.utcParse[specifier][string] = parsed(locale.utcParse(specifier), string);
    }
  }
}

// A date that is not a date at all.
//
// The JavaScript's own getters of such a date give `NaN` for every part
// of it, and the format writes that.
{
  const invalid = new Date(NaN);
  out.invalid = {format: {}, utcFormat: {}};
  for (const specifier of specifiers) {
    out.invalid.format[specifier] = timeFormat.timeFormat(specifier)(invalid);
    out.invalid.utcFormat[specifier] = timeFormat.utcFormat(specifier)(invalid);
  }
}

// The ISO form.
//
// The texts are the ones the JavaScript's own specification gives; the
// JavaScript's own fallback, which reads the texts that are not in that
// form, is not the specification's and is not written down here.
const isoStrings = [
  "2020-01-02T03:04:05.006Z",
  "2020-01-02",
  "2020-01",
  "2020",
  "2020-01-02T03:04",
  "2020-01-02T03:04:05",
  "2020-01-02T03:04:05.123",
  "2020-01-02T03:04:05.123456789",
  "2020-01-02T03:04:05Z",
  "2020-01-02t03:04:05z",
  "2020-01-02T03:04:05+05:30",
  "2020-01-02T03:04:05-05:30",
  "2020-01-02T03:04:05-08:00",
  "2020-01-02T03:04:05+0530",
  "2020-01-02T03:04:05.0060",
  "2020-01-02T03:04:05,006",
  "2020-01-02T03:04:05.006 Z",
  "2020-01-02T03:04:60",
  "20200102",
  "2020-02-30",
  "2020-13-01",
  "2020-01-32",
  "+002020-01-02",
  "-000001-01-02",
  "2020-01-02T24:00:00Z",
  "2020-01-02T03:04:05+05",
  "hello",
  "",
];

out.iso = {
  format: [...dates, ...oldDates].map((date) => {
    try {
      return timeFormat.isoFormat(date);
    } catch (error) {
      return "Invalid";
    }
  }),
  parse: {},
};
for (const string of isoStrings) {
  const date = timeFormat.isoParse(string);
  out.iso.parse[string] =
    date === null ? null : isNaN(date.getTime()) ? "Invalid" : date.getTime();
}

// The default locale, which the JavaScript's own changes.
//
// The change is made last, for it changes the locale the module's own
// formats and parses use.
{
  const locale = timeFormat.timeFormatDefaultLocale(definition);
  const format = timeFormat.timeFormat("%A %d %B %Y");
  const parse = timeFormat.timeParse("%Y %d.%m.%Y");
  out.default = {
    format: dates.map((date) => format(date)),
    utcFormat: [...dates, ...oldDates].map((date) =>
      timeFormat.utcFormat("%A %d %B %Y")(date)),
    parse: {},
    utcParse: {},
  };
  for (const string of localeParses["%Y %d.%m.%Y"]) {
    out.default.parse[string] = parsed(parse, string);
    out.default.utcParse[string] = parsed(timeFormat.utcParse("%Y %d.%m.%Y"), string);
  }
}

writeFileSync(
  process.argv[2] || "tests/fixtures/time_format.json",
  JSON.stringify(out, null, 1) + "\n");
