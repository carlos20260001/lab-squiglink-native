# How a d3 package is ported into this crate

You are porting one package of the d3 JavaScript library to Rust, faithfully.
The repository is `/home/carlos/github/lab-squiglink-native`.

## The files you may change

Only these four, and no others:

- `crates/d3/src/<package>.rs` — the port. It may already exist as a stub;
  fill it in. If the module is not declared in `crates/d3/src/lib.rs`, say
  so in your report rather than editing `lib.rs` yourself — several people
  work at once and that file is shared.
- `crates/d3/scripts/<package>_oracle.mjs` — the oracle.
- `crates/d3/tests/<package>.rs` — the test.
- `crates/d3/tests/fixtures/<package>.json` — the fixture.

**Do not touch any other file.** In particular do not edit
`crates/d3/src/lib.rs` (it is shared), do not edit `crates/d3/src/js.rs`
(it is shared), and do not run `cargo fmt` over the workspace — it would
touch other people's files. You may run `rustfmt` on your own files only.
**Do not run `git commit`** — the work is committed by the one in charge.

If you find a bug in a shared module — `crates/d3/src/js.rs`,
`crates/d3/src/number.rs`, `crates/d3/src/time/` — do not fix it in
silence: report it, with the change you would make, and the one in charge
will decide. If your port cannot be correct without the fix, make it,
and say so plainly in your report.

## The rules of this crate

1. **The JavaScript is the authority.** Where a reading of the JavaScript
   and a reading of your Rust could differ, the JavaScript is right. The
   JavaScript is vendored at `crates/d3/reference/<package>/src/` — read
   it. Note the version: d3 5.16.0 pins a particular release of each
   package, and the vendored copy is that release. Use the vendored copy
   as the authority, not the version you remember.

2. **The crate has ZERO dependencies.** No crates at all, not even
   dev-dependencies beyond what is already there (`serde_json` is a
   dev-dependency and is available to you in tests). Where the
   JavaScript uses a regular expression, write the matching by hand.
   Where it uses `Date`, `Map`, `Set` or an `Intl`, use the crate's own
   `crates/d3/src/time/` and `crates/d3/src/collection.rs`, which are
   already ported.

3. **The JavaScript's own names are kept**, in snake_case for Rust. The
   words and their order are the JavaScript's.

4. **Comments explain why, and are written as prose**, in this house
   style:

   ```rust
   /// The JavaScript's `dispatch`, which calls a function when a thing
   /// happens.
   ///
   /// The JavaScript's own is a closure with a type on it ✓, and the
   /// port's is a trait with the same three methods ✓.
   ```

   The `✓` at the end of a sentence is the house style: it marks a
   statement that has been checked against the JavaScript. Use it
   sparingly, only where you have actually checked.

5. **Every public item is documented** (the crate has
   `missing_docs = "warn"`).

6. **The port keeps the JavaScript's quirks** — the order of the keys, the
   treatment of a missing value, the errors it throws — and says in a
   comment where the port had to differ and why.

7. **The whole package is ported, not a part of it.** The crate's promise
   is that every package of d3 is here whole, so a package's whole
   surface goes in, not the two functions that are the most used.

## How the port is checked

- The JavaScript is run under Node, and what it produces is written down
  as a fixture; the test then requires the Rust to give the same
  answers. Look at `crates/d3/scripts/array_oracle.mjs`,
  `crates/d3/tests/array.rs` and `crates/d3/tests/fixtures/array.json`
  and follow that pattern.
- The d3 packages are installed at `/tmp/opencode/d3src/node_modules/`.
  If a package is not there, `npm install d3-<name>@<version>` into that
  directory — and check that the version you install is the one d3 5.16.0
  pins, which the vendored reference under `crates/d3/reference/` is.
  Some packages (d3-collection, d3-time, d3-time-format) are written for
  a bundler and name their neighbours without the `.js` extension — for
  those use the package's own bundle at
  `.../d3-<name>/dist/d3-<name>.js` instead. Check which kind yours is
  by looking at its `src/index.js`.
- The fixture is committed (the oracle needs Node; the test does not),
  and the test reads it from `crates/d3/tests/fixtures/`.
- **Write the numbers in the fixture as their bits** (the hex of the
  double), not as decimal text: a number written in text and read back
  may not be the same number. The other fixtures do this — see the
  `bits` helper in `crates/d3/tests/array.rs`.
- **A number that is not a number is compared by its bits too.** The
  JavaScript's own NaN has its own bits, and the port's must have them:
  a port that is to be faithful gives the same bits.
- Cover the package properly: every export, and a spread of values,
  including the edges (zero, one, negative, more than one, the
  boundaries).
- Make sure the test would *fail* if the port were wrong: break the
  port on purpose for a moment and see that the test catches it.
- If you can, fuzz the port against the JavaScript over a few thousand
  random inputs and say so in your report.

## When you are done

```
cd /home/carlos/github/lab-squiglink-native
cargo test --offline -p d3 --test <package>
```

and make it pass. Then run
`cargo clippy --offline -p d3 --all-targets 2>&1 | grep -E "<package>" | head -20`
and fix anything it reports about **your** files.

## Report back, concisely

- the public API you wrote,
- the test count and that they pass,
- anything in the JavaScript you could not port faithfully and why,
- **any bug you found in a shared module**, with the change you made or
  would make.
