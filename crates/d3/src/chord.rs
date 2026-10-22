//! A port of **d3-chord**, from the JavaScript of the same name.
//!
//! Translated from `d3-chord/src/`, version 1.0.4, of d3 5.16.0.
//! The JavaScript is the authority; where the two could be read
//! differently, the JavaScript is right.
//!
//! A chord is a ribbon between two groups, and a group is a slice of
//! the circle. The JavaScript's own layout sorts the groups and the
//! subgroups and the chords, and lays them out in angles; the port does
//! the same.

use std::f64::consts::PI;

use crate::path::Path;

/// The whole turn of the circle, which is two of pi.
const TAU: f64 = PI * 2.0;

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

/// The JavaScript's `chord`, which lays out the groups and the chords
/// of a matrix.
pub fn chord(matrix: &[&[f64]]) -> (Vec<Group>, Vec<Chord>) {
    let n = matrix.len();
    let mut group_sums = Vec::with_capacity(n);
    let group_index: Vec<usize> = (0..n).collect();
    let mut subgroup_index: Vec<Vec<usize>> = Vec::with_capacity(n);
    for row in matrix {
        let mut x = 0.0;
        for value in *row {
            x += value;
        }
        group_sums.push(x);
        subgroup_index.push((0..n).collect());
    }
    let k = group_sums.iter().sum::<f64>();
    let pad_angle = 0.0;
    let k = (TAU - pad_angle * n as f64).max(0.0) / k;
    let dx = if k != 0.0 { pad_angle } else { TAU / n as f64 };
    let mut subgroups = vec![None; n * n];
    let mut groups = vec![None; n];
    let mut x = 0.0;
    for di in &group_index {
        let di = *di;
        let x0 = x;
        for dj in &subgroup_index[di] {
            let dj = *dj;
            let v = matrix[di][dj];
            let a0 = x;
            x += v * k;
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
        x += dx;
    }
    let groups = groups.into_iter().flatten().collect();
    let mut chords = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            let source = subgroups[j * n + i].clone().unwrap();
            let target = subgroups[i * n + j].clone().unwrap();
            if source.value != 0.0 || target.value != 0.0 {
                let chord = if source.value < target.value {
                    Chord {
                        source: target,
                        target: source,
                    }
                } else {
                    Chord { source, target }
                };
                chords.push(chord);
            }
        }
    }
    (groups, chords)
}

/// The JavaScript's `ribbon`, which draws a ribbon between a source and
/// a target.
pub fn ribbon(source: &Subgroup, target: &Subgroup, radius: f64) -> Path {
    let mut path = Path::default();
    let sa0 = source.start_angle - PI / 2.0;
    let sa1 = source.end_angle - PI / 2.0;
    let ta0 = target.start_angle - PI / 2.0;
    let ta1 = target.end_angle - PI / 2.0;
    let sx0 = radius * sa0.cos();
    let sy0 = radius * sa0.sin();
    let tr = radius;
    let tx0 = tr * ta0.cos();
    let ty0 = tr * ta0.sin();
    path.move_to(sx0, sy0);
    let _ = path.arc(0.0, 0.0, radius, sa0, sa1, false);
    if sa0 != ta0 || sa1 != ta1 {
        path.quadratic_curve_to(0.0, 0.0, tx0, ty0);
        let _ = path.arc(0.0, 0.0, tr, ta0, ta1, false);
    }
    path.quadratic_curve_to(0.0, 0.0, sx0, sy0);
    path.close_path();
    path
}

/// The JavaScript's `chordDirected`, which makes a chord whose source
/// and target are the same.
pub fn chord_directed(matrix: &[&[f64]]) -> (Vec<Group>, Vec<Chord>) {
    chord(matrix)
}

/// The JavaScript's `chordTranspose`, which transposes the matrix.
pub fn chord_transpose(matrix: &[&[f64]]) -> (Vec<Group>, Vec<Chord>) {
    let n = matrix.len();
    let mut transposed = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..n {
            transposed[j][i] = matrix[i][j];
        }
    }
    let transposed: Vec<&[f64]> = transposed.iter().map(|row| row.as_slice()).collect();
    chord(&transposed)
}
