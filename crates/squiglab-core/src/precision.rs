//! The precision guard.
//!
//! The optimisation work in this repository changes *how* the numbers are
//! arrived at, never what they are. The curves the program draws, the
//! smoothing that shapes them, and the normalisation that moves them are
//! pinned here to their exact bits, so that a change which alters any of
//! them by so much as the last unit in the last place fails the build.
//!
//! The values are the ones the program produced before the optimisation
//! work began. They are not written to be pretty: they are written to be
//! exactly what the program computed, so that the comparison is between
//! bits and not between tolerances.
//!
//! The equalizer's own response is pinned separately, against the
//! reference's own JavaScript, in `equalizer.rs`.

#[cfg(test)]
mod tests {
    use crate::data::{CurveInfo, FRPoint};
    use crate::processing::{smooth, SmoothingParams};
    use crate::session::{EqBank, EqRow, MeasurementGroup, MeasurementVariant, Session};

    /// The bits of a curve, as they are stored.
    type CurveBits = Vec<(u32, u32)>;

    /// The shape of a drawn curve, as the guard records it: the curve it
    /// belongs to, whether it is a target, and the bits of its first, middle
    /// and last point.
    type CurveShape = (usize, bool, u32, u32, u32, u32);

    fn bits(points: &[FRPoint]) -> CurveBits {
        points
            .iter()
            .map(|point| (point.hz.to_bits(), point.db.to_bits()))
            .collect()
    }

    /// A digest of a curve's bits, so that every one of them is held to account
    /// without the file carrying four thousand numbers.
    ///
    /// This is FNV-1a, over the bits and not over the numbers: two curves that
    /// differ in the last place of one point have different digests, and two
    /// that do not differ have the same one.
    fn digest(bits: &CurveBits) -> u64 {
        const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
        const PRIME: u64 = 0x0000_0100_0000_01b3;
        let mut hash = OFFSET;
        for (hz, db) in bits {
            for byte in hz.to_le_bytes().iter().chain(db.to_le_bytes().iter()) {
                hash ^= u64::from(*byte);
                hash = hash.wrapping_mul(PRIME);
            }
        }
        hash
    }

    /// A frequency grid like the one the measurements use: a tenth of an octave
    /// from twenty hertz to twenty kilohertz.
    fn grid(count: usize) -> Vec<f32> {
        (0..count)
            .map(|index| 20.0 * 1000.0_f32.powf(index as f32 / (count - 1) as f32))
            .collect()
    }

    /// A curve with a shape worth smoothing: a rise, a peak and a fall.
    fn shaped(frequencies: &[f32]) -> Vec<FRPoint> {
        frequencies
            .iter()
            .enumerate()
            .map(|(index, &hz)| {
                let position = index as f32 / frequencies.len() as f32;
                let peak = 20.0 * (-((position - 0.62) / 0.09).powi(2)).exp();
                let tilt = 6.0 * (position - 0.5);
                FRPoint {
                    hz,
                    db: 58.0 + tilt + peak,
                }
            })
            .collect()
    }

    /// The smoothing is the same to the last bit whether the matrix is made
    /// afresh or taken from a cache.
    ///
    /// The matrix is built from the grid alone, so making it twice from the
    /// same grid must give the same answer. This is the property the caching
    /// relies on, and it is checked here rather than assumed.
    #[test]
    fn the_smoothing_is_the_same_however_the_matrix_is_made() {
        let frequencies = grid(480);
        let curve = shaped(&frequencies);

        let first = SmoothingParams::new(&frequencies, 6).unwrap();
        let second = SmoothingParams::new(&frequencies, 6).unwrap();

        let one = smooth(&curve, &first).unwrap();
        let two = smooth(&curve, &second).unwrap();
        assert_eq!(bits(&one), bits(&two));

        // And the matrix itself, bit for bit, since that is what is cached.
        assert_eq!(first.matrix_bits(), second.matrix_bits());
    }

    /// The matrix is a property of the grid and the level, and of nothing else.
    ///
    /// Two curves on the same grid are smoothed by the same matrix, and the
    /// matrix does not depend on the curve it is used on. This is the property
    /// the caching relies on: the matrix can be made once for a grid and used
    /// for every curve on it, and the answer is the same as making it afresh
    /// for each.
    #[test]
    fn the_matrix_depends_on_the_grid_and_the_level_and_on_nothing_else() {
        let frequencies = grid(480);
        let other = grid(240);

        // The same grid and the same level: the same matrix, bit for bit.
        let first = SmoothingParams::new(&frequencies, 6).unwrap();
        let second = SmoothingParams::new(&frequencies, 6).unwrap();
        assert_eq!(first.matrix_bits(), second.matrix_bits());

        // A different level: a different matrix.
        let other_level = SmoothingParams::new(&frequencies, 3).unwrap();
        assert_ne!(first.matrix_bits(), other_level.matrix_bits());

        // A different grid: a different matrix.
        let other_grid = SmoothingParams::new(&other, 6).unwrap();
        assert_ne!(first.matrix_bits(), other_grid.matrix_bits());

        // And the curve that is smoothed makes no difference to the matrix
        // itself, only to the result.
        let curve = shaped(&frequencies);
        let mut changed = curve.clone();
        changed[240].db += 3.0;
        let one = smooth(&curve, &first).unwrap();
        let two = smooth(&changed, &first).unwrap();
        assert_eq!(one.len(), two.len());
        assert_eq!(one[0].hz, two[0].hz);
    }

    /// The display curves the program draws are pinned to their bits.
    ///
    /// This is the whole of what the optimisation work may not change: the
    /// points of every curve on the graph, the curve that an equalizer makes
    /// beside its parent, and the target. The values are those the program
    /// produced before the work began.
    #[test]
    fn the_display_curves_are_the_same_to_the_bit() {
        let frequencies = grid(480);
        let mut session = Session::default();

        // A measurement with a left and a right channel on the same grid, and a
        // target beside it.
        session.curves.push(CurveInfo {
            model: "QBK BKF".to_owned(),
            variant: "L".to_owned(),
            points: shaped(&frequencies),
        });
        session.curves.push(CurveInfo {
            model: "QBK BKF".to_owned(),
            variant: "R".to_owned(),
            points: shaped(&frequencies)
                .iter()
                .map(|point| FRPoint {
                    hz: point.hz,
                    db: point.db - 1.5,
                })
                .collect(),
        });
        session.curves.push(CurveInfo {
            model: "Harman IE 2019 v2".to_owned(),
            variant: "L".to_owned(),
            points: shaped(&frequencies)
                .iter()
                .map(|point| FRPoint {
                    hz: point.hz,
                    db: point.db - 4.0,
                })
                .collect(),
        });
        session.groups.push(MeasurementGroup {
            brand: "QBK".to_owned(),
            brand_suffix: None,
            name: "BKF".to_owned(),
            variants: vec![MeasurementVariant {
                name: "L".to_owned(),
                curve_indices: vec![0],
            }],
            target: false,
            review_score: None,
            review_link: None,
            shop_link: None,
            price: None,
        });
        session.groups.push(MeasurementGroup {
            brand: "QBK".to_owned(),
            brand_suffix: None,
            name: "BKF".to_owned(),
            variants: vec![MeasurementVariant {
                name: "R".to_owned(),
                curve_indices: vec![1],
            }],
            target: false,
            review_score: None,
            review_link: None,
            shop_link: None,
            price: None,
        });
        session.groups.push(MeasurementGroup {
            brand: "Harman".to_owned(),
            brand_suffix: None,
            name: "IE 2019 v2".to_owned(),
            variants: vec![MeasurementVariant {
                name: "L".to_owned(),
                curve_indices: vec![2],
            }],
            target: true,
            review_score: None,
            review_link: None,
            shop_link: None,
            price: None,
        });
        session.active.insert(0);
        session.active.insert(1);
        session.active_targets.insert(2);
        session.smooth_level = 6;

        // A pair of filters on the measurement, so the equalized curve is there
        // too.
        session.eq_filters.insert(
            0,
            vec![
                EqRow {
                    filter: crate::equalizer::Filter::new(
                        crate::equalizer::FilterType::Peaking,
                        1_000.0,
                        1.0,
                        -3.0,
                    ),
                    enabled: true,
                    bank: EqBank::Both,
                },
                EqRow {
                    filter: crate::equalizer::Filter::new(
                        crate::equalizer::FilterType::LowShelf,
                        200.0,
                        0.7,
                        2.0,
                    ),
                    enabled: true,
                    bank: EqBank::Both,
                },
            ],
        );

        session.rebuild_display_curves();

        // The curves that are drawn, and their points, bit for bit.
        let drawn: Vec<(usize, bool, CurveBits)> = session
            .display_curves
            .iter()
            .map(|curve| (curve.group_index, curve.target, bits(&curve.points)))
            .collect();

        // The reference's own shape of a curve, for the eye: the first and the
        // last point of each, so that a change in the numbers is visible in the
        // failure message and not only in a wall of bits.
        let summary: Vec<CurveShape> = session
            .display_curves
            .iter()
            .map(|curve| {
                let first = curve
                    .points
                    .first()
                    .map(|point| point.db.to_bits())
                    .unwrap_or(0);
                let last = curve
                    .points
                    .last()
                    .map(|point| point.db.to_bits())
                    .unwrap_or(0);
                let middle = curve
                    .points
                    .get(curve.points.len() / 2)
                    .map(|point| point.db.to_bits())
                    .unwrap_or(0);
                (
                    curve.group_index,
                    curve.target,
                    first,
                    middle,
                    last,
                    curve.points.len() as u32,
                )
            })
            .collect();

        assert_eq!(
            summary,
            vec![
                (0, false, 1_111_736_237, 1_112_472_351, 1_112_781_769, 480),
                (0, false, 1_111_212_429, 1_112_885_755, 1_112_782_016, 480),
                (1, false, 1_111_212_429, 1_112_885_755, 1_112_782_016, 480),
                (2, true, 1_111_212_429, 1_112_885_755, 1_112_782_016, 480),
            ],
            "the display curves have changed; the optimisation must not change them"
        );
        assert_eq!(drawn.len(), 4, "the number of drawn curves has changed");

        // And every point of every curve, to its exact bit. The three
        // points above are for the eye; this is the check. The digest is
        // taken over the bits themselves, so a change to any one of them
        // changes it.
        let digests: Vec<u64> = session
            .display_curves
            .iter()
            .map(|curve| digest(&bits(&curve.points)))
            .collect();
        assert_eq!(
            digests,
            vec![
                4_780_704_968_488_885_110,
                4_344_029_678_929_859_288,
                15_130_162_126_814_105_159,
                5_354_860_558_369_409_449,
            ],
            "a point of a drawn curve has changed; the optimisation must not change it"
        );
    }
}
