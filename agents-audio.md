# Agent: Audio and EQ correctness audit

## Scope

Fix correctness defects in the native audio/EQ path of `lab-squiglink-native`.

Primary files:
- `crates/squiglab-core/src/audio.rs`
- `crates/squiglab-core/src/equalizer.rs`
- Related tests and call sites under `crates/`

Use the JavaScript reference for EQ curve/filter semantics where applicable. Keep the native implementation idiomatic Rust and preserve existing public APIs where practical. Do not broaden this task into UI parity, redesign, or system-wide audio routing.

## Workflow

1. Inspect the current working tree and read the relevant implementation, tests, and callers before editing. Repository line numbers and branches can change; verify every finding against the checked-out code.
2. Reproduce each bug with a focused test before implementing the fix when practical.
3. Fix root causes, not symptoms. Avoid adding dependencies unless the existing workspace cannot reasonably support the requirement.
4. Run formatting, targeted tests, and the full workspace test suite. Report commands and their real results; never claim a test passed unless it was executed successfully.
5. Review the final diff for accidental API changes, duplicated logic, and unrelated edits.

## P1 — Shared biquads must not share state between stereo channels

**Location:** `crates/squiglab-core/src/audio.rs`, `Generator::render()` and `Chain`; state is stored in `crates/squiglab-core/src/equalizer.rs`, `Biquad`.

`Biquad` is stateful: `process()` updates its previous input/output samples (`x` and `y`). `Generator::render()` currently sends the left sample and then the right sample through the *same* instance of every `chain.shared` filter. The right channel therefore inherits the left channel's history. Coefficients may be shared, but delay/state memory must not be shared across channels.

### Required change

- Give every shared filter an independent left and right state instance, using identical coefficients.
- Keep left-only filters confined to the left path and right-only filters confined to the right path.
- Keep filter ordering consistent with the intended chain.
- When rebuilding/changing a chain or sample rate, ensure each channel's coefficients and state remain consistent; do not leave stale filters or stale-rate coefficients.
- Prefer a clear representation such as paired channel filters or separate left/right shared banks. Update constructors, conversion helpers, and all callers consistently.

### Required regression tests

- With a non-empty shared EQ chain and an identical mono source, left and right output samples must match within an explicitly justified floating-point tolerance.
- With different left/right input sequences (for example, an impulse on the left only), each channel's output must match an independently processed reference and must not leak into the other channel through filter history.
- Test more than one filter in a chain so state independence is verified across the whole cascade.

## P1 — Music playback must preserve stereo and use frame-based resampling

**Location:** `crates/squiglab-core/src/audio.rs`, `FilePlayback::next_sample()` and `Generator::render()` / `Generator::next_sample()`.

`FilePlayback.samples` is documented as interleaved by channel, but `next_sample()` averages values over the channel count and returns one scalar. `Generator` then duplicates this scalar into left and right outputs. Stereo image and channel-specific content are lost.

Also audit the playhead and interpolation indexing: the playhead currently advances by a rate ratio while indexing the interleaved scalar sample vector, but interpolation steps by `channels`. For multi-channel input, indexing must be coherent in *frames*, not accidentally slide across adjacent interleaved channel samples.

### Required change

- Change the internal playback path to produce a stereo frame (left/right) rather than one averaged scalar.
- Track playhead position in frames. Compute frame index and interpolation fraction from that position; interpolate each channel against the corresponding channel in the next frame.
- For mono input, duplicate the sample to both output channels.
- For stereo input, preserve left/right separation exactly apart from resampling and subsequent EQ.
- For files with more than two channels, either implement and document an explicit channel-layout-aware downmix or retain a deliberate, documented policy. Never average arbitrary adjacent interleaved scalar samples as a substitute for downmixing.
- Update the internal API and all tests/callers together. Keep a scalar helper only if it has an unambiguous mono contract.

### Required regression tests

- A stereo test file with distinct left and right content must preserve that separation after playback/resampling.
- A mono file must still render equally on both channels.
- At a different input/output sample rate, both channels must retain the expected pitch and duration within a stated tolerance.
- Add a test with an impulse or unique sample pattern per channel to catch indexing that crosses channel boundaries.

## P2 — Correct end-of-file and looping boundaries

**Location:** `crates/squiglab-core/src/audio.rs`, `FilePlayback::next_sample()` (or its replacement).

The current end check tests `position >= last` before reading a sample. This can return silence instead of emitting the final valid frame. With looping enabled, a one-frame file can repeatedly reset to position zero and recurse without making progress.

### Required change

- Define file length, last valid frame, and playhead in frame units.
- Emit every valid frame, including the final frame, exactly once for non-looping playback.
- Implement looping without recursion. Wrapping must always make progress and must behave correctly for files shorter than one resampling step.
- Handle empty data, zero channels, invalid/inconsistent channel counts, non-finite or non-positive sample rates, and malformed trailing samples without panics, infinite recursion, or out-of-bounds reads. Follow the existing project error-handling conventions; do not silently hide invalid data if callers can report an error.
- Decide and test the expected output after a non-looping file ends (for example, silence after the final frame).

### Required regression tests

- Empty file, one-frame file, two-frame file, and ordinary file; each with looping both disabled and enabled where meaningful.
- Assert the last frame is not skipped.
- Assert looping a one-frame file returns promptly and repeats the expected audio.
- Exercise fractional resampling steps near the end of a file.

## P2 — Validate the rendered DSP chain, not only EQ curves

**Locations:** `crates/squiglab-core/src/equalizer.rs`, `audio.rs`, and current DSP/reference-fixture tests.

Reference-derived curve fixtures are useful for checking numerical parity, but a correct plotted response alone does not prove that rendered audio has that response. Tests must cover the filter coefficients, stateful processing, channel paths, and the combined chain.

### Required change

- Preserve the JavaScript reference's filter semantics and coefficient conventions for peaking (`PK`), low-shelf (`LSQ`), and high-shelf (`HSQ`) filters.
- Keep curve computation and audio filtering mathematically consistent. If a difference is found, identify whether the curve, coefficients, or processing is wrong before changing expected fixtures.
- Add deterministic impulse/sine response tests for a single filter and multiple cascaded filters.
- Cover zero gain, reasonable parameter extremes, frequency/sample-rate boundaries, and invalid/non-finite inputs according to the existing API's validation policy.
- Do not loosen tolerances just to make a failing test pass. Document the numerical reason for each nontrivial tolerance.
- Keep reference-generated golden fixtures and add randomized/deterministic parameter coverage where useful.

## P2 — Sample-rate changes must not leave stale filter coefficients

**Location:** `crates/squiglab-core/src/audio.rs`, `Generator::set_sample_rate()` and `set_chain()`; inspect all code that creates filter banks.

Filter coefficients depend on sample rate. Confirm that when the output device reports a different rate, the active EQ chain is rebuilt or otherwise recalculated for that rate. Do not assume updating oscillator/resampler timing also updates the existing `Biquad` coefficients.

### Acceptance criteria

- A chain built for the active sample rate is used for every rendered block.
- Changing sample rate cannot retain coefficients calculated for the old rate.
- Tests verify the updated chain matches a newly constructed reference chain at the new rate.

## Validation commands

Run from the repository root, adjusting only if workspace scripts specify a different standard:

```sh
cargo fmt --all -- --check
cargo test -p squiglab-core
cargo test --workspace
```

If the package name differs, inspect the workspace manifest and run the equivalent package test. Run Linux/PipeWire integration tests when available and supported by the environment; distinguish skipped/unavailable integration tests from passed tests. For audio loopback testing, document the device, sample rate, test signal, capture method, and measured result.

## Definition of done

- [ ] Shared filter state is independent per channel.
- [ ] Stereo files preserve L/R content; mono still duplicates cleanly.
- [ ] Resampling uses frame-correct indexing and retains pitch.
- [ ] Final-frame, empty-file, and looping boundary cases are safe and tested.
- [ ] Rendered audio is tested against expected EQ behaviour, not just plotted curves.
- [ ] Active filter coefficients match the active output sample rate.
- [ ] Formatting, targeted tests, and workspace tests have been run and results reported accurately.
- [ ] Only audio/EQ-related files changed unless a directly required caller or fixture also needs updating.
