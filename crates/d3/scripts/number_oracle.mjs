// The oracle: the ECMAScript's own writing of a number, run under Node,
// writing what it produces.
//
//   node scripts/number_oracle.mjs > tests/fixtures/number.json

import {writeFileSync} from "fs";

const values = [
  0, -0, 1, -1, 0.5, -0.5, 2, -2, 2.5, -2.5, 1.5, -1.5, 0.125, -0.125,
  0.375, -0.375, 1.005, -1.005, 9.995, -9.995, 0.000001, 0.0000001,
  1e21, -1e21, 1e20, -1e20, 123456789.987654321, -123456789.987654321,
  0.1, 0.2, 0.3, 1 / 3, 2 / 3, 1e-300, 1e300, 1.7976931348623157e308,
  4.9406564584124654e-324, 100, 1000, 1e6, 12345.6789, 0.000012345,
  9.99, 0.0999, 1e-7, 1e-6, 5e-324, 1.5e-10, 123456789012345680000,
  9.5, 9.95, 95, 0.95, 0.995, 99.5, 999.5, 0.0000005, 0.00000005,
  1.5, 15, 150, 1e-5, 1e-4, 999999999999999900000,
];

const out = {fixed: {}, exponential: {}, precision: {}};

for (const value of values) {
  for (let digits = 0; digits <= 20; digits += 1) {
    out.fixed[`${value}/${digits}`] = value.toFixed(digits);
  }
  for (let digits = 0; digits <= 20; digits += 1) {
    out.exponential[`${value}/${digits}`] = value.toExponential(digits);
  }
  for (let digits = 1; digits <= 21; digits += 1) {
    out.precision[`${value}/${digits}`] = value.toPrecision(digits);
  }
}

// The number in another base, which the JavaScript's own `toString`
// writes.
{
  const result = {};
  // The numbers that are written in another base are the whole ones
  // below ten to the nineteen, for the JavaScript's own writing of a
  // larger number in another base loses the last of it, and the
  // specification leaves that to the implementation.
  for (const value of values.filter((value) => Math.abs(value) < 1e19)) {
    for (const radix of [2, 8, 10, 16, 36]) {
      // The number is a whole one, for the formats that use this round
      // the number first.
      result[`${value}/${radix}`] = Math.round(value).toString(radix);
    }
  }
  out.radix = result;
}

writeFileSync(
  process.argv[2] || "tests/fixtures/number.json",
  JSON.stringify(out, null, 1) + "\n");
