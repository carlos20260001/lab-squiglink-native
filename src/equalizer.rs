//! A numerical port of the squig.link equalizer DSP.
//!
//! `web/equalizer.js` is the source of truth for every filter calculation in
//! this module: each function below corresponds to one in that file, in the
//! same operation order, so the native app reproduces the web tool's curves
//! and exports rather than approximating them. Everything is [`f64`] for the
//! same reason the original is — the biquad coefficient chain loses precision
//! in `f32` quickly enough to show in a plotted curve.
//!
//! The port is pinned against the JavaScript output by golden fixtures:
//! `equalizer_golden.json` holds responses and AutoEQ results captured from
//! the original, and the tests in this module compare against them. Regenerate
//! the fixture with `node scripts/equalizer_golden.mjs` after changing the
//! reference implementation, never by editing it by hand.

use once_cell::sync::Lazy;
use std::f64::consts::PI;

/// Sample rate the biquads and magnitude responses are calculated against.
/// Not every DSP software uses this rate, and filters near Nyquist move when it
/// changes; the original pins one common value for the same reason.
pub const DEFAULT_SAMPLE_RATE: f64 = 48_000.0;
/// AutoEQ keeps filters below this frequency in its first batch, leaving the
/// treble to be corrected once the rest of the curve is settled.
pub const TREBLE_START_FROM: f64 = 7_000.0;
/// Frequency bounds AutoEQ places filters within. Filters close to Nyquist are
/// avoided because their behaviour is implementation dependent.
pub const AUTO_EQ_RANGE: (f64, f64) = (20.0, 15_000.0);
/// Bounds the optimizer and the exported filters are clamped to.
pub const OPTIMIZE_Q_RANGE: (f64, f64) = (0.5, 2.0);
/// Bounds the optimizer and the exported filters are clamped to.
pub const OPTIMIZE_GAIN_RANGE: (f64, f64) = (-12.0, 12.0);
/// Per-pass coordinate-descent deltas, coarse to fine: the maximum frequency,
/// Q, and gain step, then the size each step is multiplied by.
const OPTIMIZE_DELTAS: [[f64; 6]; 3] = [
    [10.0, 10.0, 10.0, 5.0, 0.1, 0.5],
    [10.0, 10.0, 10.0, 2.0, 0.1, 0.2],
    [10.0, 10.0, 10.0, 1.0, 0.1, 0.1],
];

/// The three filter types the reference tool has. Low-pass and high-pass
/// filters are deliberately absent: the original has no LP/HP either, and
/// inventing one here would diverge from its exports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterType {
    /// Peaking bell, the type every AutoEQ candidate uses.
    Peaking,
    /// Low shelf.
    LowShelf,
    /// High shelf.
    HighShelf,
}

/// One parametric filter: a type, a centre or corner frequency in Hz, a Q, and
/// a gain in dB.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Filter {
    pub filter_type: FilterType,
    pub freq: f64,
    pub q: f64,
    pub gain: f64,
}

impl Filter {
    pub fn new(filter_type: FilterType, freq: f64, q: f64, gain: f64) -> Self {
        Self {
            filter_type,
            freq,
            q,
            gain,
        }
    }
}

/// One point on a frequency response: a frequency in Hz and a value in dB.
/// This is the shape the JavaScript original exchanges as `[hz, db]` pairs.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ResponsePoint {
    pub hz: f64,
    pub db: f64,
}

impl ResponsePoint {
    pub fn new(hz: f64, db: f64) -> Self {
        Self { hz, db }
    }
}

/// Piecewise-linear resampler, shared with the measurement loader rather than
/// being EQ-specific. `frequencies` must ascend, matching the original: the
/// cursor only moves forward across calls for successive frequencies.
///
/// Values outside the curve are clamped to its end values, and the curve must
/// not be empty.
pub fn interp(frequencies: &[f64], curve: &[ResponsePoint]) -> Vec<ResponsePoint> {
    let last = curve
        .last()
        .expect("interp requires a curve with at least one point");
    let mut cursor = 0usize;
    frequencies
        .iter()
        .map(|&frequency| {
            while cursor + 1 < curve.len() {
                let (f0, v0) = (curve[cursor].hz, curve[cursor].db);
                let (f1, v1) = (curve[cursor + 1].hz, curve[cursor + 1].db);
                if cursor == 0 && frequency < f0 {
                    return ResponsePoint::new(frequency, v0);
                } else if (f0..f1).contains(&frequency) {
                    let value = v0 + (v1 - v0) * (frequency - f0) / (f1 - f0);
                    return ResponsePoint::new(frequency, value);
                }
                cursor += 1;
            }
            ResponsePoint::new(frequency, last.db)
        })
        .collect()
}

/// RBJ low-shelf coefficients as `(a0, a1, a2, b0, b1, b2)` with `a0`
/// normalised to one, matching the order the JavaScript original returns.
pub fn lowshelf(freq: f64, q: f64, gain: f64, sample_rate: f64) -> [f64; 6] {
    let freq = (freq / sample_rate).clamp(1e-6, 1.0);
    let q = q.clamp(1e-4, 1000.0);
    let gain = gain.clamp(-40.0, 40.0);

    let w0 = 2.0 * PI * freq;
    let sin = w0.sin();
    let cos = w0.cos();
    let a = 10.0_f64.powf(gain / 40.0);
    let alpha = sin / (2.0 * q);
    let alphamod = 2.0 * a.sqrt() * alpha;

    let a0 = (a + 1.0) + (a - 1.0) * cos + alphamod;
    let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos);
    let a2 = (a + 1.0) + (a - 1.0) * cos - alphamod;
    let b0 = a * ((a + 1.0) - (a - 1.0) * cos + alphamod);
    let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos);
    let b2 = a * ((a + 1.0) - (a - 1.0) * cos - alphamod);

    [1.0, a1 / a0, a2 / a0, b0 / a0, b1 / a0, b2 / a0]
}

/// RBJ high-shelf coefficients, normalised like [`lowshelf`].
pub fn highshelf(freq: f64, q: f64, gain: f64, sample_rate: f64) -> [f64; 6] {
    let freq = (freq / sample_rate).clamp(1e-6, 1.0);
    let q = q.clamp(1e-4, 1000.0);
    let gain = gain.clamp(-40.0, 40.0);

    let w0 = 2.0 * PI * freq;
    let sin = w0.sin();
    let cos = w0.cos();
    let a = 10.0_f64.powf(gain / 40.0);
    let alpha = sin / (2.0 * q);
    let alphamod = 2.0 * a.sqrt() * alpha;

    let a0 = (a + 1.0) - (a - 1.0) * cos + alphamod;
    let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos);
    let a2 = (a + 1.0) - (a - 1.0) * cos - alphamod;
    let b0 = a * ((a + 1.0) + (a - 1.0) * cos + alphamod);
    let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos);
    let b2 = a * ((a + 1.0) + (a - 1.0) * cos - alphamod);

    [1.0, a1 / a0, a2 / a0, b0 / a0, b1 / a0, b2 / a0]
}

/// RBJ peaking coefficients, normalised like [`lowshelf`].
pub fn peaking(freq: f64, q: f64, gain: f64, sample_rate: f64) -> [f64; 6] {
    let freq = (freq / sample_rate).clamp(1e-6, 1.0);
    let q = q.clamp(1e-4, 1000.0);
    let gain = gain.clamp(-40.0, 40.0);

    let w0 = 2.0 * PI * freq;
    let sin = w0.sin();
    let cos = w0.cos();
    let a = 10.0_f64.powf(gain / 40.0);
    let alpha = sin / (2.0 * q);

    let a0 = 1.0 + alpha / a;
    let a1 = -2.0 * cos;
    let a2 = 1.0 - alpha / a;
    let b0 = 1.0 + alpha * a;
    let b1 = -2.0 * cos;
    let b2 = 1.0 - alpha * a;

    [1.0, a1 / a0, a2 / a0, b0 / a0, b1 / a0, b2 / a0]
}

/// Coefficients for every filter that has an effect. The original drops a
/// filter whose frequency, Q, or gain is zero, because such a filter
/// contributes nothing and would only cost calculation; a non-finite component
/// is dropped for the same reason, since it would poison the whole response.
fn filters_to_coeffs(filters: &[Filter], sample_rate: f64) -> Vec<[f64; 6]> {
    filters
        .iter()
        .filter(|filter| {
            let inert = |value: f64| value == 0.0 || !value.is_finite();
            !(inert(filter.freq) || inert(filter.q) || inert(filter.gain))
        })
        .map(|filter| match filter.filter_type {
            FilterType::LowShelf => lowshelf(filter.freq, filter.q, filter.gain, sample_rate),
            FilterType::HighShelf => highshelf(filter.freq, filter.q, filter.gain, sample_rate),
            FilterType::Peaking => peaking(filter.freq, filter.q, filter.gain, sample_rate),
        })
        .collect()
}

/// Magnitude response in dB at each frequency, summed over every biquad.
pub fn calc_gains(freqs: &[f64], coeffs: &[[f64; 6]], sample_rate: f64) -> Vec<f64> {
    let mut gains = vec![0.0; freqs.len()];
    for [a0, a1, a2, b0, b1, b2] in coeffs {
        for (index, &freq) in freqs.iter().enumerate() {
            let w = 2.0 * PI * freq / sample_rate;
            let phi = 4.0 * (w / 2.0).sin().powi(2);
            let numerator = (b0 + b1 + b2).powi(2)
                + (b0 * b2 * phi - (b1 * (b0 + b2) + 4.0 * b0 * b2)) * phi;
            let denominator = (a0 + a1 + a2).powi(2)
                + (a0 * a2 * phi - (a1 * (a0 + a2) + 4.0 * a0 * a2)) * phi;
            gains[index] += 10.0 * numerator.log10() - 10.0 * denominator.log10();
        }
    }
    gains
}

/// Preamp gain for an export: the negative of the largest boost the EQ asks
/// for, so the exported curve never boosts. Calculated between the base curve
/// and the EQ'd curve on the same grid.
pub fn calc_preamp(base: &[ResponsePoint], eq: &[ResponsePoint]) -> f64 {
    let mut max_gain = f64::NEG_INFINITY;
    for (base_point, eq_point) in base.iter().zip(eq) {
        max_gain = max_gain.max(eq_point.db - base_point.db);
    }
    -max_gain
}

/// Mean absolute difference between two curves, ignoring differences below
/// 0.1 dB so rounding noise does not dominate the optimizer's score.
pub fn calc_distance(a: &[ResponsePoint], b: &[ResponsePoint]) -> f64 {
    let mut distance = 0.0;
    for (left, right) in a.iter().zip(b) {
        let difference = (left.db - right.db).abs();
        if difference >= 0.1 {
            distance += difference;
        }
    }
    distance / a.len() as f64
}

/// Add the EQ's gain onto a curve, returning a new curve on the same grid.
pub fn apply(curve: &[ResponsePoint], filters: &[Filter], sample_rate: f64) -> Vec<ResponsePoint> {
    let freqs: Vec<f64> = curve.iter().map(|point| point.hz).collect();
    let coeffs = filters_to_coeffs(filters, sample_rate);
    let gains = calc_gains(&freqs, &coeffs, sample_rate);
    curve
        .iter()
        .zip(&gains)
        .map(|(point, &gain)| ResponsePoint::new(point.hz, point.db + gain))
        .collect()
}

/// Dense ~1/96-octave grid the raw filter response is sampled on before it is
/// averaged down onto the graphic-EQ bands.
static GRAPHIC_EQ_RAW_FREQUENCIES: Lazy<Vec<f64>> = Lazy::new(|| {
    let count = ((20_000.0_f64 / 20.0).ln() / 1.0072_f64.ln()).ceil() as usize;
    (0..count)
        .map(|index| 20.0 * 1.0072_f64.powi(index as i32))
        .collect()
});

/// Smoothed graphic-EQ grid frequencies, ascending and deduplicated. The
/// original floors the raw values and dedupes through a Set before sorting.
static GRAPHIC_EQ_FREQUENCIES: Lazy<Vec<f64>> = Lazy::new(|| {
    let count = ((20_000.0_f64 / 20.0).ln() / 1.0563_f64.ln()).ceil() as usize;
    let mut frequencies: Vec<f64> = (0..count)
        .map(|index| (20.0 * 1.0563_f64.powi(index as i32)).floor())
        .collect();
    frequencies.sort_by(|left, right| left.total_cmp(right));
    frequencies.dedup();
    frequencies
});

/// The filter set rendered as a Wavelet GraphicEQ curve: sampled on the dense
/// raw grid, averaged onto the 127 graphic bands, then shifted down by its own
/// maximum so the curve never boosts.
pub fn as_graphic_eq(filters: &[Filter], sample_rate: f64) -> Vec<ResponsePoint> {
    let raw_freqs = &*GRAPHIC_EQ_RAW_FREQUENCIES;
    let bands = &*GRAPHIC_EQ_FREQUENCIES;
    let coeffs = filters_to_coeffs(filters, sample_rate);
    let gains = calc_gains(raw_freqs, &coeffs, sample_rate);
    let mut cursor = 0usize;
    let mut result: Vec<ResponsePoint> = bands
        .iter()
        .enumerate()
        .map(|(index, &freq)| {
            let freq_to = if index + 1 < bands.len() {
                (freq * bands[index + 1]).sqrt()
            } else {
                20_000.0
            };
            let mut sum = 0.0;
            let mut count = 0usize;
            while cursor < raw_freqs.len() {
                if raw_freqs[cursor] < freq_to {
                    sum += gains[cursor];
                    count += 1;
                    cursor += 1;
                } else {
                    break;
                }
            }
            ResponsePoint::new(freq, sum / count as f64)
        })
        .collect();
    let max_gain = result
        .iter()
        .fold(f64::NEG_INFINITY, |max, point| if max > point.db { max } else { point.db });
    for point in &mut result {
        point.db -= max_gain;
    }
    result
}

/// Peaking-filter candidates for the regions where a curve deviates from its
/// target by more than `threshold` dB. Each candidate spans one run of
/// same-signed deviation, centred on the geometric mean of its bounds. Both
/// curves must share the same ascending frequency grid.
pub fn search_candidates(
    curve: &[ResponsePoint],
    target: &[ResponsePoint],
    threshold: f64,
) -> Vec<Filter> {
    let mut state = 0.0; // 1: peak, 0: matched, -1: dip
    let mut start_index: isize = -1;
    let mut candidates = Vec::new();
    let (min_freq, max_freq) = AUTO_EQ_RANGE;
    for index in 0..curve.len() {
        let freq = curve[index].hz;
        let delta = curve[index].db - target[index].db;
        let delta_abs = delta.abs();
        let next_state = if delta_abs < threshold {
            0.0
        } else {
            delta / delta_abs
        };
        if next_state == state {
            continue;
        }
        if start_index >= 0 {
            if state != 0.0 {
                let start = curve[start_index as usize].hz;
                let end = freq;
                let center = (start * end).sqrt();
                let gain = interp(&[center], &target[start_index as usize..index])[0].db
                    - interp(&[center], &curve[start_index as usize..index])[0].db;
                let q = center / (end - start);
                if (min_freq..=max_freq).contains(&center) {
                    candidates.push(Filter::new(FilterType::Peaking, center, q, gain));
                }
            }
            start_index = -1;
        } else {
            start_index = index as isize;
        }
        state = next_state;
    }
    candidates
}

/// Frequency granularity a value is rounded to, so exported filters land on
/// numbers some DSP devices accept.
fn freq_unit(freq: f64) -> f64 {
    if freq < 100.0 {
        1.0
    } else if freq < 1000.0 {
        10.0
    } else if freq < 10000.0 {
        100.0
    } else {
        1000.0
    }
}

/// Round filters to friendly, device-compatible values and clamp them into the
/// optimizer's ranges.
fn strip(filters: Vec<Filter>) -> Vec<Filter> {
    let (min_q, max_q) = OPTIMIZE_Q_RANGE;
    let (min_gain, max_gain) = OPTIMIZE_GAIN_RANGE;
    filters
        .into_iter()
        .map(|filter| Filter {
            filter_type: filter.filter_type,
            freq: (filter.freq - filter.freq % freq_unit(filter.freq)).floor(),
            q: ((filter.q * 10.0).floor() / 10.0).clamp(min_q, max_q),
            gain: ((filter.gain * 10.0).floor() / 10.0).clamp(min_gain, max_gain),
        })
        .collect()
}

/// One coordinate-descent pass over the filters, forward or in reverse, at
/// coarse-to-fine `iteration`. Each filter is re-placed by trying frequency,
/// Q, and gain steps around its current value and keeping the first
/// improvement. The reverse pass additionally sorts, merges adjacent filters
/// that are close enough to be one, and prunes filters the curve does not
/// need.
pub fn optimize(
    curve: &[ResponsePoint],
    target: &[ResponsePoint],
    filters: Vec<Filter>,
    iteration: usize,
    reverse: bool,
) -> Vec<Filter> {
    let mut filters = strip(filters);
    let (min_freq, max_freq) = AUTO_EQ_RANGE;
    let (min_q, max_q) = OPTIMIZE_Q_RANGE;
    let (min_gain, max_gain) = OPTIMIZE_GAIN_RANGE;
    let [max_df, max_dq, max_dg, step_df, step_dq, step_dg] = OPTIMIZE_DELTAS[iteration];
    let count = filters.len() as isize;
    let (begin, end, step) = if reverse {
        (count - 1, -1, -1)
    } else {
        (0, count, 1)
    };
    let mut index = begin;
    while index != end {
        let position = index as usize;
        let filter = filters[position];
        // The response with this filter removed, then with it alone on top.
        // (The original also builds the response with every filter applied
        // here and never reads it; that work is left out.)
        let others: Vec<Filter> = filters
            .iter()
            .enumerate()
            .filter(|(other_index, _)| *other_index != position)
            .map(|(_, other)| *other)
            .collect();
        let without = apply(curve, &others, DEFAULT_SAMPLE_RATE);
        let with = apply(&without, &[filter], DEFAULT_SAMPLE_RATE);
        let mut best_filter = filter;
        let mut best_distance = calc_distance(&with, target);
        let mut test_filter = |df: f64, dq: f64, dg: f64| -> bool {
            let freq = filter.freq + df * freq_unit(filter.freq) * step_df;
            let q = filter.q + dq * step_dq;
            let gain = filter.gain + dg * step_dg;
            if !(min_freq..=max_freq).contains(&freq)
                || !(min_q..=max_q).contains(&q)
                || !(min_gain..=max_gain).contains(&gain)
            {
                return false;
            }
            let candidate = Filter::new(filter.filter_type, freq, q, gain);
            let candidate_curve = apply(&without, &[candidate], DEFAULT_SAMPLE_RATE);
            let distance = calc_distance(&candidate_curve, target);
            if distance < best_distance {
                best_filter = candidate;
                best_distance = distance;
                return true;
            }
            false
        };
        for df in -(max_df as i64)..max_df as i64 {
            for dq in (-(max_dq as i64)..max_dq as i64 - 1).rev() {
                // Widen and deepen the filter, then narrow it: each inner loop
                // stops at the first step that fails to improve.
                for dg in 1..max_dg as i64 {
                    if !test_filter(df as f64, dq as f64, dg as f64) {
                        break;
                    }
                }
                for dg in (-(max_dg as i64)..0).rev() {
                    if !test_filter(df as f64, dq as f64, dg as f64) {
                        break;
                    }
                }
            }
        }
        filters[position] = best_filter;
        index += step;
    }
    if !reverse {
        return optimize(curve, target, filters, iteration, true);
    }

    filters.sort_by(|left, right| left.freq.total_cmp(&right.freq));
    // Merge filters that sit within one frequency unit of each other and share
    // a Q: they are one filter the search split in two.
    let mut merge_index = 0;
    while merge_index + 1 < filters.len() {
        let (first, second) = (filters[merge_index], filters[merge_index + 1]);
        if (first.freq - second.freq).abs() <= freq_unit(first.freq)
            && (first.q - second.q).abs() <= 0.1
        {
            filters[merge_index].gain += second.gain;
            filters.remove(merge_index + 1);
        } else {
            merge_index += 1;
        }
    }
    // Remove filters that contribute nothing, starting from the smallest.
    let mut best_distance = calc_distance(&apply(curve, &filters, DEFAULT_SAMPLE_RATE), target);
    let mut prune_index = 0;
    while prune_index < filters.len() {
        if filters[prune_index].gain.abs() <= 0.1 {
            filters.remove(prune_index);
            continue;
        }
        let remaining: Vec<Filter> = filters
            .iter()
            .enumerate()
            .filter(|(other_index, _)| *other_index != prune_index)
            .map(|(_, other)| *other)
            .collect();
        let distance = calc_distance(&apply(curve, &remaining, DEFAULT_SAMPLE_RATE), target);
        if distance < best_distance {
            filters.remove(prune_index);
            best_distance = distance;
        } else {
            prune_index += 1;
        }
    }
    filters
}

/// Fit at most `max_filters` peaking filters to a curve so it approaches its
/// target. Two batches: the first corrects everything below the treble start
/// at a 1 dB threshold, the second spends the remaining budget on whatever is
/// left at a 0.5 dB threshold, and both are then optimized together.
pub fn autoeq(
    curve: &[ResponsePoint],
    target: &[ResponsePoint],
    max_filters: usize,
) -> Vec<Filter> {
    let first_batch_size = ((max_filters as f64 / 2.0).floor() - 1.0).max(1.0) as usize;
    let mut first_candidates = search_candidates(curve, target, 1.0)
        .into_iter()
        // The treble is left to the second batch.
        .filter(|candidate| candidate.freq <= TREBLE_START_FROM)
        .collect::<Vec<_>>();
    first_candidates.sort_by(|left, right| left.q.total_cmp(&right.q));
    let mut first_filters: Vec<Filter> = first_candidates
        .into_iter()
        .take(first_batch_size)
        .collect();
    first_filters.sort_by(|left, right| left.freq.total_cmp(&right.freq));
    for iteration in 0..OPTIMIZE_DELTAS.len() {
        first_filters = optimize(curve, target, first_filters, iteration, false);
    }

    let corrected = apply(curve, &first_filters, DEFAULT_SAMPLE_RATE);
    let second_batch_size = max_filters.saturating_sub(first_filters.len());
    let mut second_candidates = search_candidates(&corrected, target, 0.5);
    second_candidates.sort_by(|left, right| left.q.total_cmp(&right.q));
    let mut second_filters: Vec<Filter> = second_candidates
        .into_iter()
        .take(second_batch_size)
        .collect();
    second_filters.sort_by(|left, right| left.freq.total_cmp(&right.freq));
    for iteration in 0..OPTIMIZE_DELTAS.len() {
        second_filters = optimize(&corrected, target, second_filters, iteration, false);
    }

    let mut all_filters = first_filters;
    all_filters.extend(second_filters);
    for iteration in 0..OPTIMIZE_DELTAS.len() {
        all_filters = optimize(curve, target, all_filters, iteration, false);
    }
    strip(all_filters)
}

// ---------------------------------------------------------------------------
// Equalizer APO parametric export
// ---------------------------------------------------------------------------

/// An imported Equalizer APO configuration: the filter bands in file order,
/// and which of them the file switches on.
pub struct ImportedFilters {
    pub filters: Vec<Filter>,
    pub enabled: Vec<bool>,
}

/// Render a filter set as an Equalizer APO parametric EQ configuration,
/// against `preamp`, the negative peak of the EQ'd curve so it never boosts.
///
/// Lines are CRLF separated, as Equalizer APO expects. `Preamp` is written at
/// full precision. Shelf filters use APO's shorter `LSC` and `HSC` names, and
/// bands whose frequency, Q, and gain are all zero are omitted, matching the
/// reference tool's exports.
pub fn export_parametric(filters: &[Filter], preamp: f64) -> String {
    // Negative zero would export as "-0", which Equalizer APO accepts but no
    // other tool round-trips cleanly.
    let preamp = if preamp == 0.0 { 0.0 } else { preamp };
    let mut output = format!("Preamp: {preamp} dB\r\n");
    let mut number = 1;
    for filter in filters {
        if filter.freq == 0.0 && filter.q == 0.0 && filter.gain == 0.0 {
            continue;
        }
        let filter_type = match filter.filter_type {
            FilterType::Peaking => "PK",
            FilterType::LowShelf => "LSC",
            FilterType::HighShelf => "HSC",
        };
        use std::fmt::Write;
        let _ = write!(
            output,
            "Filter {number}: ON {filter_type} Fc {} Hz Gain {} dB Q {}\r\n",
            filter.freq, filter.gain, filter.q
        );
        number += 1;
    }
    output
}

/// Parse an Equalizer APO parametric EQ configuration. Only the three filter
/// types the reference tool has are accepted; a bandwidth (`BW`) band or a
/// filter type outside `PK`, `LSC`, and `HSC` is refused rather than guessed
/// at, because a silent conversion would not round-trip.
pub fn import_parametric(text: &str) -> anyhow::Result<ImportedFilters> {
    let mut filters = Vec::new();
    let mut enabled = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        let number = index + 1;
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let lowered = line.to_ascii_lowercase();
        if lowered.starts_with("preamp") {
            continue;
        }
        // Everything after "Filter <n>:", so a missing or renumbered prefix
        // does not matter.
        let body = match line.split_once(':') {
            Some((_, body)) => body.trim(),
            None => anyhow::bail!("line {number}: not a filter band: {line}"),
        };
        let mut tokens = body.split_whitespace();
        let state = tokens
            .next()
            .ok_or_else(|| anyhow::anyhow!("line {number}: a filter band needs a state"))?;
        let on = match state.to_ascii_lowercase().as_str() {
            "on" => true,
            "off" => false,
            other => anyhow::bail!("line {number}: unknown filter state {other:?}"),
        };
        let filter_type = match tokens
            .next()
            .ok_or_else(|| anyhow::anyhow!("line {number}: a filter band needs a type"))?
            .to_ascii_uppercase()
            .as_str()
        {
            "PK" => FilterType::Peaking,
            "LSC" | "LSQ" => FilterType::LowShelf,
            "HSC" | "HSQ" => FilterType::HighShelf,
            other => anyhow::bail!(
                "line {number}: filter type {other} is not one of PK, LSC, or HSC"
            ),
        };
        // Fc, Gain, and Q arrive as key/value pairs with their units between
        // them: "Fc 500 Hz Gain -3.5 dB Q 1.2". A key takes the token after it
        // as its value, and the units are skipped on their own.
        let (mut freq, mut q, mut gain) = (0.0, 0.0, 0.0);
        let mut tokens = tokens.peekable();
        while let Some(key) = tokens.next() {
            let mut value = || {
                tokens
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("line {number}: {key} has no value"))
            };
            match key.to_ascii_uppercase().as_str() {
                "FC" => freq = parse_number(value()?, number)?,
                "Q" => q = parse_number(value()?, number)?,
                "GAIN" => gain = parse_number(value()?, number)?,
                "HZ" | "DB" => {}
                "BW" => anyhow::bail!(
                    "line {number}: bandwidth filters are not supported, only Q"
                ),
                other => anyhow::bail!("line {number}: unknown filter field {other}"),
            }
        }
        filters.push(Filter::new(filter_type, freq, q, gain));
        enabled.push(on);
    }
    Ok(ImportedFilters { filters, enabled })
}

fn parse_number(token: &str, line: usize) -> anyhow::Result<f64> {
    token
        .parse::<f64>()
        .map_err(|_| anyhow::anyhow!("line {line}: {token:?} is not a number"))
}

#[cfg(test)]
mod tests {
    use super::{
        apply, as_graphic_eq, autoeq, calc_distance, calc_gains, calc_preamp, export_parametric,
        highshelf, import_parametric, interp, lowshelf, peaking, Filter, FilterType, ResponsePoint,
        DEFAULT_SAMPLE_RATE,
    };
    use serde::Deserialize;

    /// Golden responses captured from `web/equalizer.js` by
    /// `scripts/equalizer_golden.mjs`. The whole point of this port is that it
    /// reproduces the reference's numbers, so the comparisons below are the
    /// only authority on whether a change to this file is correct.
    #[derive(Deserialize)]
    struct GoldenFixture {
        #[serde(rename = "sampleRate")]
        sample_rate: f64,
        curve: Vec<[f64; 2]>,
        target: Vec<[f64; 2]>,
        filters: Vec<GoldenFilter>,
        lowshelf: [f64; 6],
        highshelf: [f64; 6],
        peaking: [f64; 6],
        gains: Vec<f64>,
        apply: Vec<[f64; 2]>,
        interp: GoldenInterp,
        #[serde(rename = "calcPreamp")]
        calc_preamp: f64,
        #[serde(rename = "calcDistance")]
        calc_distance: f64,
        #[serde(rename = "asGraphicEq")]
        as_graphic_eq: Vec<[f64; 2]>,
        autoeq: GoldenAutoEq,
        #[serde(rename = "autoeqSmall")]
        autoeq_small: GoldenAutoEq,
    }

    #[derive(Deserialize)]
    struct GoldenFilter {
        #[serde(rename = "type")]
        filter_type: String,
        freq: f64,
        q: f64,
        gain: f64,
    }

    #[derive(Deserialize)]
    struct GoldenInterp {
        frequencies: Vec<f64>,
        curve: Vec<[f64; 2]>,
        result: Vec<[f64; 2]>,
    }

    #[derive(Deserialize)]
    struct GoldenAutoEq {
        #[serde(rename = "maxFilters")]
        max_filters: usize,
        filters: Vec<GoldenFilter>,
    }

    impl GoldenFilter {
        fn to_filter(&self) -> Filter {
            let filter_type = match self.filter_type.as_str() {
                "LSQ" => FilterType::LowShelf,
                "HSQ" => FilterType::HighShelf,
                "PK" => FilterType::Peaking,
                other => panic!("fixture names an unknown filter type: {other}"),
            };
            Filter::new(filter_type, self.freq, self.q, self.gain)
        }
    }

    fn fixture() -> GoldenFixture {
        let json = include_str!("equalizer_golden.json");
        serde_json::from_str(json).expect("the golden fixture is valid JSON")
    }

    fn to_curve(pairs: &[[f64; 2]]) -> Vec<ResponsePoint> {
        pairs
            .iter()
            .map(|[hz, db]| ResponsePoint::new(*hz, *db))
            .collect()
    }

    fn to_filters(golden: &[GoldenFilter]) -> Vec<Filter> {
        golden.iter().map(GoldenFilter::to_filter).collect()
    }

    /// Distance beyond which two doubles are not the same response. biquad
    /// coefficients, magnitudes, and optimizer scores are all compared with
    /// it: it is far tighter than anything a plotted curve could show, and far
    /// looser than the last-ulp differences between the JavaScript and Rust
    /// transcendental functions.
    const TOLERANCE: f64 = 1e-9;

    fn assert_close(actual: f64, expected: f64, context: &str) {
        assert!(
            (actual - expected).abs() <= TOLERANCE,
            "{context}: port produced {actual}, reference produced {expected}"
        );
    }

    fn assert_curves_match(actual: &[ResponsePoint], expected: &[[f64; 2]], context: &str) {
        assert_eq!(
            actual.len(),
            expected.len(),
            "{context}: port produced {} points, reference produced {}",
            actual.len(),
            expected.len()
        );
        for (index, (point, [hz, db])) in actual.iter().zip(expected).enumerate() {
            assert_close(point.hz, *hz, &format!("{context} point {index} frequency"));
            assert_close(point.db, *db, &format!("{context} point {index} value"));
        }
    }

    fn assert_filters_match(actual: &[Filter], expected: &[GoldenFilter], context: &str) {
        assert_eq!(
            actual.len(),
            expected.len(),
            "{context}: port produced {} filters, reference produced {}",
            actual.len(),
            expected.len()
        );
        for (index, (filter, golden)) in actual.iter().zip(expected).enumerate() {
            assert_eq!(
                filter.filter_type,
                golden.to_filter().filter_type,
                "{context} filter {index} type"
            );
            assert_close(
                filter.freq,
                golden.freq,
                &format!("{context} filter {index} frequency"),
            );
            assert_close(filter.q, golden.q, &format!("{context} filter {index} Q"));
            assert_close(
                filter.gain,
                golden.gain,
                &format!("{context} filter {index} gain")
            );
        }
    }

    #[test]
    fn biquad_coefficients_match_the_reference() {
        let golden = fixture();
        let lowshelf = lowshelf(105.0, 0.71, 6.0, golden.sample_rate);
        let highshelf = highshelf(9000.0, 0.71, -4.0, golden.sample_rate);
        let peaking = peaking(500.0, 1.2, -3.5, golden.sample_rate);
        for (name, actual, expected) in [
            ("lowshelf", lowshelf, golden.lowshelf),
            ("highshelf", highshelf, golden.highshelf),
            ("peaking", peaking, golden.peaking),
        ] {
            for (index, (coefficient, reference)) in
                actual.iter().zip(expected).enumerate()
            {
                assert_close(*coefficient, reference, &format!("{name} coefficient {index}"));
            }
            // a0 is normalised to one on return, as in the original.
            assert_close(actual[0], 1.0, &format!("{name} a0"));
        }
    }

    #[test]
    fn summed_magnitudes_match_the_reference() {
        let golden = fixture();
        let curve = to_curve(&golden.curve);
        let filters = to_filters(&golden.filters);
        let freqs: Vec<f64> = curve.iter().map(|point| point.hz).collect();
        let coeffs: Vec<[f64; 6]> = filters
            .iter()
            .map(|filter| match filter.filter_type {
                FilterType::LowShelf => lowshelf(filter.freq, filter.q, filter.gain, golden.sample_rate),
                FilterType::HighShelf => {
                    highshelf(filter.freq, filter.q, filter.gain, golden.sample_rate)
                }
                FilterType::Peaking => peaking(filter.freq, filter.q, filter.gain, golden.sample_rate),
            })
            .collect();
        let gains = calc_gains(&freqs, &coeffs, golden.sample_rate);
        assert_eq!(gains.len(), golden.gains.len());
        for (index, (gain, reference)) in gains.iter().zip(&golden.gains).enumerate() {
            assert_close(*gain, *reference, &format!("gain at point {index}"));
        }
    }

    #[test]
    fn applying_filters_matches_the_reference_response() {
        let golden = fixture();
        let curve = to_curve(&golden.curve);
        let filters = to_filters(&golden.filters);
        let equalized = apply(&curve, &filters, golden.sample_rate);
        assert_curves_match(&equalized, &golden.apply, "apply");
        // An empty filter set is the identity: it returns its input, not the
        // curve before the EQ.
        let unchanged = apply(&equalized, &[], golden.sample_rate);
        assert_curves_match(&unchanged, &golden.apply, "apply with no filters");
    }

    #[test]
    fn interpolation_resamples_and_clamps_like_the_reference() {
        let golden = fixture();
        let curve = to_curve(&golden.interp.curve);
        let result = interp(&golden.interp.frequencies, &curve);
        assert_curves_match(&result, &golden.interp.result, "interp");
    }

    #[test]
    fn preamp_and_distance_match_the_reference() {
        let golden = fixture();
        let curve = to_curve(&golden.curve);
        let target = to_curve(&golden.target);
        let filters = to_filters(&golden.filters);
        let equalized = apply(&curve, &filters, golden.sample_rate);
        assert_close(
            calc_preamp(&curve, &equalized),
            golden.calc_preamp,
            "calc_preamp",
        );
        assert_close(
            calc_distance(&curve, &target),
            golden.calc_distance,
            "calc_distance",
        );
    }

    #[test]
    fn graphic_eq_bands_match_the_reference_and_never_boost() {
        let golden = fixture();
        let filters = to_filters(&golden.filters);
        let bands = as_graphic_eq(&filters, golden.sample_rate);
        assert_curves_match(&bands, &golden.as_graphic_eq, "as_graphic_eq");
        // Precompensation is folded in, so nothing may sit above zero.
        for band in &bands {
            assert!(
                band.db <= 0.0,
                "graphic EQ band at {} Hz boosts to {}",
                band.hz,
                band.db
            );
        }
        assert_close(bands[0].db, 0.0, "the loudest band is normalised to zero");
    }

    #[test]
    fn autoeq_fits_the_reference_filter_set() {
        let golden = fixture();
        let curve = to_curve(&golden.curve);
        let target = to_curve(&golden.target);
        for case in [&golden.autoeq, &golden.autoeq_small] {
            let filters = autoeq(&curve, &target, case.max_filters);
            assert_filters_match(
                &filters,
                &case.filters,
                &format!("autoeq with {} filters", case.max_filters),
            );
        }
    }

    #[test]
    fn filters_with_no_effect_are_dropped_from_the_response() {
        let curve = vec![
            ResponsePoint::new(100.0, 0.0),
            ResponsePoint::new(1000.0, 0.0),
        ];
        let flat = apply(
            &curve,
            &[Filter::new(FilterType::Peaking, 500.0, 1.0, 0.0)],
            DEFAULT_SAMPLE_RATE,
        );
        assert_curves_match(&flat, &[[100.0, 0.0], [1000.0, 0.0]], "inert filter");
    }

    #[test]
    fn a_distant_shelf_leaves_a_band_flat() {
        // The grid the curve sits on, and the flat response an empty filter
        // set must return.
        let points: Vec<[f64; 2]> = (0..40)
            .map(|index| [20.0 * (1000.0_f64).powf(index as f64 / 39.0), 0.0])
            .collect();
        let curve = to_curve(&points);
        let flat = apply(&curve, &[], DEFAULT_SAMPLE_RATE);
        assert_curves_match(&flat, &points, "empty filter set");
        // A low shelf at 100 Hz must not have reached 20 kHz.
        let shelved = apply(
            &curve,
            &[Filter::new(FilterType::LowShelf, 100.0, 0.71, -9.0)],
            DEFAULT_SAMPLE_RATE,
        );
        let top = shelved.last().unwrap();
        assert!(
            top.db.abs() < 0.01,
            "the low shelf reached the treble: {top:?}"
        );
        // A peaking filter centred at 20 kHz must move it.
        let peaked = apply(
            &curve,
            &[Filter::new(FilterType::Peaking, 20_000.0, 1.0, 6.0)],
            DEFAULT_SAMPLE_RATE,
        );
        let top = peaked.last().unwrap();
        assert!(top.db > 5.0, "the peaking filter missed the treble: {top:?}");
    }

    /// The Equalizer APO configuration captured from the live site, the only
    /// authority on the export format.
    const CAPTURED_APO_SAMPLE: &str = include_str!("../docs/design-reference/export-apo-sample.txt");

    #[test]
    fn captured_sample_imports_and_exports_back_identically() {
        let imported = import_parametric(CAPTURED_APO_SAMPLE).unwrap();
        assert_eq!(imported.enabled, [true]);
        assert_eq!(
            imported.filters,
            [Filter::new(FilterType::Peaking, 500.0, 1.2, -3.5)]
        );
        // The capture was saved with LF endings; the export writes CRLF, so
        // the comparison is on the text with endings normalised.
        let exported = export_parametric(&imported.filters, 0.0002);
        assert_eq!(exported.replace("\r\n", "\n"), CAPTURED_APO_SAMPLE);
    }

    #[test]
    fn parametric_export_writes_crlf_apo_names_and_drops_empty_bands() {
        let filters = [
            Filter::new(FilterType::Peaking, 500.0, 1.2, -3.5),
            Filter::new(FilterType::LowShelf, 105.0, 0.71, 6.0),
            Filter::new(FilterType::HighShelf, 9000.0, 0.71, -4.0),
            Filter::new(FilterType::Peaking, 0.0, 0.0, 0.0),
        ];
        let exported = export_parametric(&filters, -5.989272033886169);
        assert_eq!(
            exported,
            "Preamp: -5.989272033886169 dB\r\n\
             Filter 1: ON PK Fc 500 Hz Gain -3.5 dB Q 1.2\r\n\
             Filter 2: ON LSC Fc 105 Hz Gain 6 dB Q 0.71\r\n\
             Filter 3: ON HSC Fc 9000 Hz Gain -4 dB Q 0.71\r\n"
        );
    }

    #[test]
    fn parametric_round_trip_preserves_the_filter_set() {
        let filters = [
            Filter::new(FilterType::Peaking, 42.0, 0.9, 6.6),
            Filter::new(FilterType::LowShelf, 105.0, 0.71, 6.0),
            Filter::new(FilterType::HighShelf, 9000.0, 0.71, -4.0),
        ];
        let exported = export_parametric(&filters, -2.0);
        let imported = import_parametric(&exported).unwrap();
        assert!(imported.enabled.iter().all(|&on| on));
        assert_eq!(imported.filters, filters);
        // And again: importing a file exported from the reference's own names
        // gives the same filters as one exported from this port.
        let reference_names = exported
            .replace("LSC", "LSQ")
            .replace("HSC", "HSQ");
        let imported = import_parametric(&reference_names).unwrap();
        assert_eq!(imported.filters, filters);
    }

    #[test]
    fn parametric_import_refuses_what_it_cannot_represent() {
        assert!(import_parametric("Filter 1: ON LPQ Fc 500 Hz Gain -3.5 dB").is_err());
        assert!(import_parametric("Filter 1: ON PK Fc 500 Hz Gain -3.5 dB BW 2").is_err());
        assert!(import_parametric("Filter 1: MAYBE PK Fc 500 Hz Gain -3.5 dB").is_err());
        assert!(import_parametric("Filter 1: ON PK Fc five Hz Gain -3.5 dB").is_err());
        assert!(import_parametric("this is not a filter band").is_err());
        // A preamp line and blank and commented lines are skipped, not errors.
        let imported =
            import_parametric("# exported by squig.link\r\n\r\nPreamp: -3 dB\r\n").unwrap();
        assert!(imported.filters.is_empty());
        // An off band is imported switched off.
        let imported =
            import_parametric("Filter 1: OFF PK Fc 500 Hz Gain -3.5 dB Q 1.2\r\n").unwrap();
        assert_eq!(imported.filters.len(), 1);
        assert_eq!(imported.enabled, [false]);
    }
}
