//! A port of **d3-chord**, from the JavaScript of the same name.
//!
//! Translated from `d3-chord/src/`, version 1.0.4, of d3 5.16.0.
//! The JavaScript is the authority; where the two could be read
//! differently, the JavaScript is right.
//!
//! A chord is a ribbon between two groups, and a group is a slice of
//! the circle. The layout sorts the groups, the subgroups and the
//! chords, and lays them out in angles; the ribbon draws the band
//! between a source and a target.
//!
//! The JavaScript builds its layout as a function with three setters,
//! `padAngle`, `sortGroups`, `sortSubgroups` and `sortChords`, and the
//! port builds it as [`Chord`]. The JavaScript's `chord` is a factory;
//! the port's is [`Chord::new`].

use std::cmp::Ordering;
use std::f64::consts::PI;

use crate::arith;
use crate::ieee754;
use crate::js;
use crate::path::Path;
use crate::shape::Context;

/// The whole turn of the circle, which is two of pi.
const TAU: f64 = PI * 2.0;

/// A comparison of two numbers, as the JavaScript's `Array#sort` takes
/// one.
///
/// The JavaScript's comparator returns a number, not an ordering: a
/// negative number puts the first before the second, a positive one
/// puts it after, and zero keeps the order it found. A number that is
/// not a number is treated as zero, which is what the JavaScript's
/// `Array#sort` does.
pub type Compare = Box<dyn Fn(f64, f64) -> f64>;

/// A group, which is a slice of the circle.
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    /// The index of the group.
    pub index: usize,
    /// The angle at which the group begins.
    pub start_angle: f64,
    /// The angle at which the group ends.
    pub end_angle: f64,
    /// The value of the group.
    pub value: f64,
}

/// A chord, which is a ribbon between two groups.
#[derive(Clone, Debug, PartialEq)]
pub struct Chord {
    /// The source of the chord.
    pub source: Subgroup,
    /// The target of the chord.
    pub target: Subgroup,
}

/// A subgroup, which is a slice of a group.
#[derive(Clone, Debug, PartialEq)]
pub struct Subgroup {
    /// The index of the group.
    pub index: usize,
    /// The index of the subgroup.
    pub subindex: usize,
    /// The angle at which the subgroup begins.
    pub start_angle: f64,
    /// The angle at which the subgroup ends.
    pub end_angle: f64,
    /// The value of the subgroup.
    pub value: f64,
}

/// The groups and the chords of a matrix.
///
/// The JavaScript returns the chords as an array with the groups on it,
/// `chords.groups`, and the port returns the two together.
#[derive(Clone, Debug, PartialEq)]
pub struct Chords {
    /// The groups, one for each row of the matrix.
    pub groups: Vec<Group>,
    /// The chords, one for each non-empty pair of subgroups.
    pub chords: Vec<Chord>,
}

/// The JavaScript's `chord`, which lays out the groups and the chords
/// of a matrix.
///
/// The JavaScript's `chord()` returns a function with setters on it;
/// the port's [`ChordLayout::new`] returns a value with the same setters, and
/// [`ChordLayout::apply`] calls it.
pub struct ChordLayout {
    pad_angle: f64,
    sort_groups: Option<Compare>,
    sort_subgroups: Option<Compare>,
    sort_chords: Option<Compare>,
}

impl Default for ChordLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl ChordLayout {
    /// The JavaScript's `chord()`: a layout with no padding and no
    /// sorting.
    pub fn new() -> Self {
        Self {
            pad_angle: 0.0,
            sort_groups: None,
            sort_subgroups: None,
            sort_chords: None,
        }
    }

    /// The JavaScript's `chord.padAngle`, which sets the angle between
    /// the groups.
    ///
    /// The JavaScript takes the greater of zero and the angle it is
    /// given, and so a negative angle becomes zero. A number that is
    /// not a number stays one, as it does in the JavaScript's own
    /// `Math.max`.
    pub fn pad_angle(mut self, value: f64) -> Self {
        self.pad_angle = max(0.0, value);
        self
    }

    /// The JavaScript's `chord.sortGroups`, which sorts the groups by
    /// the sums of their rows.
    pub fn sort_groups<F>(mut self, compare: F) -> Self
    where
        F: Fn(f64, f64) -> f64 + 'static,
    {
        self.sort_groups = Some(Box::new(compare));
        self
    }

    /// The JavaScript's `chord.sortSubgroups`, which sorts the
    /// subgroups within each row by the values of that row.
    pub fn sort_subgroups<F>(mut self, compare: F) -> Self
    where
        F: Fn(f64, f64) -> f64 + 'static,
    {
        self.sort_subgroups = Some(Box::new(compare));
        self
    }

    /// The JavaScript's `chord.sortChords`, which sorts the chords by
    /// the sum of their source and target values.
    ///
    /// The JavaScript keeps the comparator it is given on the wrapper it
    /// sorts with, and its getter returns the original; the port keeps
    /// the original too.
    pub fn sort_chords<F>(mut self, compare: F) -> Self
    where
        F: Fn(f64, f64) -> f64 + 'static,
    {
        self.sort_chords = Some(Box::new(compare));
        self
    }

    /// The JavaScript's `chord.sortChords(null)`, which removes the
    /// sorting of the chords.
    pub fn clear_sort_chords(mut self) -> Self {
        self.sort_chords = None;
        self
    }

    /// The JavaScript's `chord.padAngle()` getter.
    pub fn get_pad_angle(&self) -> f64 {
        self.pad_angle
    }

    /// The JavaScript's `chord.sortGroups()` getter.
    pub fn get_sort_groups(&self) -> Option<&Compare> {
        self.sort_groups.as_ref()
    }

    /// The JavaScript's `chord.sortSubgroups()` getter.
    pub fn get_sort_subgroups(&self) -> Option<&Compare> {
        self.sort_subgroups.as_ref()
    }

    /// The JavaScript's `chord.sortChords()` getter, which gives the
    /// comparator as it was given rather than the wrapper.
    pub fn get_sort_chords(&self) -> Option<&Compare> {
        self.sort_chords.as_ref()
    }

    /// The JavaScript's `chord(matrix)`.
    pub fn apply(&self, matrix: &[&[f64]]) -> Chords {
        let n = matrix.len();

        // The JavaScript's own loops go by the place of a thing in the
        // matrix, and the place is what the sorting and the layout are
        // about.
        let mut group_sums = Vec::with_capacity(n);
        let mut group_index: Vec<usize> = (0..n).collect();
        let mut subgroup_index: Vec<Vec<usize>> = Vec::with_capacity(n);
        let mut k = 0.0;
        for row in matrix {
            let mut x = 0.0;
            for value in *row {
                x = arith::add(x, *value);
            }
            group_sums.push(x);
            subgroup_index.push((0..n).collect());
            k = arith::add(k, x);
        }

        // The JavaScript sorts the groups by their sums.
        if let Some(compare) = &self.sort_groups {
            group_index.sort_by(|&a, &b| compare_number(compare, group_sums[a], group_sums[b]));
        }

        // The JavaScript sorts each row's subgroups by that row's values.
        if let Some(compare) = &self.sort_subgroups {
            for i in 0..n {
                let row = matrix[i];
                subgroup_index[i].sort_by(|&a, &b| compare_number(compare, row[a], row[b]));
            }
        }

        // The JavaScript's scaling factor and its padding step.
        let k = arith::divide(
            max(
                0.0,
                arith::subtract(TAU, arith::multiply(self.pad_angle, n as f64)),
            ),
            k,
        );
        let dx = if js::truthy(k) {
            self.pad_angle
        } else {
            arith::divide(TAU, n as f64)
        };

        // The JavaScript's object literal has the group index and the
        // subgroup index in it, and the port keeps them in the struct.
        let mut subgroups: Vec<Option<Subgroup>> = vec![None; n.saturating_mul(n)];
        let mut groups: Vec<Option<Group>> = vec![None; n];
        let mut x = 0.0;
        for &di in &group_index {
            let x0 = x;
            for &dj in &subgroup_index[di] {
                let v = matrix[di][dj];
                let a0 = x;
                x = arith::add(x, arith::multiply(v, k));
                subgroups[dj * n + di] = Some(Subgroup {
                    index: di,
                    subindex: dj,
                    start_angle: a0,
                    end_angle: x,
                    value: v,
                });
            }
            groups[di] = Some(Group {
                index: di,
                start_angle: x0,
                end_angle: x,
                value: group_sums[di],
            });
            x = arith::add(x, dx);
        }

        // The JavaScript makes a chord for each non-empty pair. The
        // JavaScript's `||` is false for zero and for a number that is
        // not a number, and the port keeps that.
        let mut chords = Vec::new();
        for i in 0..n {
            for j in i..n {
                let source = subgroups[j * n + i].clone().expect("the lower half");
                let target = subgroups[i * n + j].clone().expect("the upper half");
                if js::truthy(source.value) || js::truthy(target.value) {
                    chords.push(if source.value < target.value {
                        Chord {
                            source: target,
                            target: source,
                        }
                    } else {
                        Chord { source, target }
                    });
                }
            }
        }

        // The JavaScript's `sortChords` is given the sum of the two
        // values, which is the JavaScript's own `+`.
        if let Some(compare) = &self.sort_chords {
            chords.sort_by(|a, b| {
                compare_number(
                    compare,
                    arith::add(a.source.value, a.target.value),
                    arith::add(b.source.value, b.target.value),
                )
            });
        }

        Chords {
            groups: groups.into_iter().flatten().collect(),
            chords,
        }
    }
}

/// The JavaScript's `chordDefault`, which is a layout with no padding
/// and no sorting.
///
/// The JavaScript writes this as `chord()(matrix)`, and the port keeps
/// the two calls apart so that the layout can be given its setters.
pub fn chord(matrix: &[&[f64]]) -> Chords {
    ChordLayout::new().apply(matrix)
}

/// Compare two numbers as the JavaScript's `Array#sort` does.
///
/// A comparator that returns a number that is not a number is treated as
/// zero, and the order it found is kept.
fn compare_number(compare: &Compare, a: f64, b: f64) -> Ordering {
    let result = compare(a, b);
    if result.is_nan() || result == 0.0 {
        Ordering::Equal
    } else if result < 0.0 {
        Ordering::Less
    } else {
        Ordering::Greater
    }
}

/// The JavaScript's `Math.max`, which gives a number that is not a
/// number when either of its arguments is one.
fn max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a > b {
        a
    } else {
        b
    }
}

/// Something the ribbon can read a source and a target from.
///
/// The JavaScript's `ribbon` takes a datum and reads `d.source` and
/// `d.target` from it by default. The port's datum gives them as the
/// [`Subgroup`]s the layout made.
pub trait RibbonDatum {
    /// The JavaScript's `d.source`.
    fn source(&self) -> Subgroup;
    /// The JavaScript's `d.target`.
    fn target(&self) -> Subgroup;
}

impl RibbonDatum for Chord {
    fn source(&self) -> Subgroup {
        self.source.clone()
    }

    fn target(&self) -> Subgroup {
        self.target.clone()
    }
}

/// A function that reads a subgroup out of a datum.
pub type RibbonEnd<D> = Box<dyn Fn(&D) -> Subgroup>;

/// A function that reads a number out of a subgroup.
pub type RibbonNumber = Box<dyn Fn(&Subgroup) -> f64>;

/// The JavaScript's `ribbon`, which draws the band between a source and
/// a target.
///
/// The JavaScript's ribbon is a generator with accessors on it. The
/// port's is a value with the same accessors, and [`Ribbon::apply`]
/// calls it.
pub struct Ribbon<D> {
    source: RibbonEnd<D>,
    target: RibbonEnd<D>,
    radius: RibbonNumber,
    start_angle: RibbonNumber,
    end_angle: RibbonNumber,
}

impl<D: RibbonDatum> Default for Ribbon<D> {
    fn default() -> Self {
        Self {
            source: Box::new(|d| d.source()),
            target: Box::new(|d| d.target()),
            // The JavaScript's default radius reads `d.radius`, and a
            // subgroup has no radius of its own. The JavaScript gives a
            // number that is not a number for it, and the port does too.
            radius: Box::new(|_| f64::NAN),
            start_angle: Box::new(|s| s.start_angle),
            end_angle: Box::new(|s| s.end_angle),
        }
    }
}

impl<D> Ribbon<D> {
    /// The JavaScript's `ribbon.source`, which reads the source from the
    /// datum.
    pub fn source<F>(mut self, f: F) -> Self
    where
        F: Fn(&D) -> Subgroup + 'static,
    {
        self.source = Box::new(f);
        self
    }

    /// The JavaScript's `ribbon.target`, which reads the target from the
    /// datum.
    pub fn target<F>(mut self, f: F) -> Self
    where
        F: Fn(&D) -> Subgroup + 'static,
    {
        self.target = Box::new(f);
        self
    }

    /// The JavaScript's `ribbon.radius`, which reads the radius from the
    /// source and from the target.
    pub fn radius<F>(mut self, f: F) -> Self
    where
        F: Fn(&Subgroup) -> f64 + 'static,
    {
        self.radius = Box::new(f);
        self
    }

    /// The JavaScript's `ribbon.startAngle`, which reads the angle from
    /// the source and from the target.
    pub fn start_angle<F>(mut self, f: F) -> Self
    where
        F: Fn(&Subgroup) -> f64 + 'static,
    {
        self.start_angle = Box::new(f);
        self
    }

    /// The JavaScript's `ribbon.endAngle`, which reads the angle from
    /// the source and from the target.
    pub fn end_angle<F>(mut self, f: F) -> Self
    where
        F: Fn(&Subgroup) -> f64 + 'static,
    {
        self.end_angle = Box::new(f);
        self
    }

    /// The JavaScript's `ribbon(datum)`, which gives the path of the
    /// ribbon.
    pub fn apply(&self, datum: &D) -> String {
        let mut path = Path::default();
        self.apply_to(datum, &mut path);
        path.text().to_owned()
    }

    /// The JavaScript's `ribbon.context`, which draws into a context the
    /// caller holds rather than returning the path.
    pub fn apply_to(&self, datum: &D, context: &mut dyn Context) {
        let s = (self.source)(datum);
        let t = (self.target)(datum);
        let sr = (self.radius)(&s);
        let sa0 = arith::subtract((self.start_angle)(&s), PI / 2.0);
        let sa1 = arith::subtract((self.end_angle)(&s), PI / 2.0);
        let sx0 = arith::multiply(sr, ieee754::cos(sa0));
        let sy0 = arith::multiply(sr, ieee754::sin(sa0));
        let tr = (self.radius)(&t);
        let ta0 = arith::subtract((self.start_angle)(&t), PI / 2.0);
        let ta1 = arith::subtract((self.end_angle)(&t), PI / 2.0);

        context.move_to(sx0, sy0);
        let _ = context.arc(0.0, 0.0, sr, sa0, sa1, false);
        if sa0 != ta0 || sa1 != ta1 {
            context.quadratic_curve_to(
                0.0,
                0.0,
                arith::multiply(tr, ieee754::cos(ta0)),
                arith::multiply(tr, ieee754::sin(ta0)),
            );
            let _ = context.arc(0.0, 0.0, tr, ta0, ta1, false);
        }
        context.quadratic_curve_to(0.0, 0.0, sx0, sy0);
        context.close_path();
    }
}

/// The JavaScript's `ribbon()`, with the default accessors, drawn with a
/// radius of one.
///
/// The JavaScript's own default radius is `d.radius`, and the port's
/// [`Ribbon`] keeps that default. This helper is for the common case in
/// which the two subgroups are given and the radius is known.
pub fn ribbon(source: &Subgroup, target: &Subgroup, radius: f64) -> Path {
    let mut path = Path::default();
    ribbon_to(source, target, radius, &mut path);
    path
}

/// The JavaScript's `ribbon` drawn into a context.
pub fn ribbon_to(source: &Subgroup, target: &Subgroup, radius: f64, context: &mut dyn Context) {
    let sa0 = arith::subtract(source.start_angle, PI / 2.0);
    let sa1 = arith::subtract(source.end_angle, PI / 2.0);
    let sx0 = arith::multiply(radius, ieee754::cos(sa0));
    let sy0 = arith::multiply(radius, ieee754::sin(sa0));
    let ta0 = arith::subtract(target.start_angle, PI / 2.0);
    let ta1 = arith::subtract(target.end_angle, PI / 2.0);

    context.move_to(sx0, sy0);
    let _ = context.arc(0.0, 0.0, radius, sa0, sa1, false);
    if sa0 != ta0 || sa1 != ta1 {
        context.quadratic_curve_to(
            0.0,
            0.0,
            arith::multiply(radius, ieee754::cos(ta0)),
            arith::multiply(radius, ieee754::sin(ta0)),
        );
        let _ = context.arc(0.0, 0.0, radius, ta0, ta1, false);
    }
    context.quadratic_curve_to(0.0, 0.0, sx0, sy0);
    context.close_path();
}
