//! Rig-compensation delta targets.
//!
//! No coupler or microphone measures a headphone on its own. A rig adds its
//! own response to everything it measures, and a *clone* of that rig — a 711
//! build that is not quite a 711, a substitute capsule standing in for a
//! B&K 5128 — adds a slightly different one. Published targets were measured on
//! the original hardware, so comparing a clone-rig measurement against a target
//! naively charges the rig's error to the headphone.
//!
//! A delta target isolates that error: the clone rig's own calibration minus
//! the original rig's. Subtracting it from a measurement taken on the clone
//! puts the headphone back in the original rig's frame, which is the frame
//! every target in the library is expressed in.
//!
//! Two consequences shape how this is used. A delta target is never *required*
//! to read a target — it is an optional correction, and the graph treats it as
//! one rather than as a target in its own right. And because a delta target
//! only means something alongside the calibrations it came from, it is defined
//! by those two curves rather than stored as a bare difference, so the claim it
//! makes is always traceable back to the measurements behind it.

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use serde::Deserialize;

use crate::data::FRPoint;
use crate::processing;

/// The manifest that names every delta target and the calibrations behind it.
const MANIFEST: &str = "delta_targets.json";

/// Why a delta target could not be built or applied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeltaError {
    /// The manifest named a calibration curve that is not in the directory.
    MissingCalibration(String),
    /// A calibration curve held no usable points.
    EmptyCalibration(String),
    /// The shared frequency grid a delta target is sampled onto is not a
    /// strictly increasing run of positive frequencies.
    InvalidGrid,
    /// A curve had to be sampled outside anything the delta target covers.
    Unsampleable,
    /// The delta directory, or its manifest, could not be read.
    Io(String),
    /// The manifest was not valid JSON in the expected shape.
    Manifest(String),
}

impl fmt::Display for DeltaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCalibration(name) => {
                write!(formatter, "no calibration curve named {name:?}")
            }
            Self::EmptyCalibration(name) => write!(formatter, "{name:?} holds no points"),
            Self::InvalidGrid => {
                write!(
                    formatter,
                    "the frequency grid must be finite, positive, and strictly increasing"
                )
            }
            Self::Unsampleable => write!(formatter, "unable to sample the delta target"),
            Self::Io(detail) => write!(formatter, "{detail}"),
            Self::Manifest(detail) => write!(formatter, "delta target manifest: {detail}"),
        }
    }
}

impl std::error::Error for DeltaError {}

/// One entry of the manifest: a delta target named by the two calibrations
/// that produce it.
#[derive(Clone, Debug, Deserialize)]
struct DeltaSpec {
    /// The name shown in the library and the manager.
    name: String,
    /// File name of the clone rig's calibration, resolved in the delta directory.
    clone: String,
    /// File name of the original rig's calibration, resolved there too.
    original: String,
    /// Whether the calibrations behind this target are placeholders rather
    /// than measurements. Carried through to the interface so a synthetic
    /// correction can never be mistaken for a real rig.
    #[serde(default)]
    synthetic: bool,
    /// Free text explaining where the calibrations came from.
    #[serde(default)]
    note: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct DeltaManifest {
    deltas: Vec<DeltaSpec>,
}

/// A rig-compensation curve: what a clone rig adds that the original does not.
#[derive(Clone, Debug)]
pub struct DeltaTarget {
    /// The name shown in the library and the manager.
    pub name: String,
    /// The clone rig this corrects for.
    pub clone_rig: String,
    /// The original rig whose frame the correction restores.
    pub original_rig: String,
    /// Whether the calibrations behind this target are placeholders.
    pub synthetic: bool,
    /// Where the calibrations came from.
    pub note: Option<String>,
    /// The compensation itself, on the shared grid: clone minus original.
    pub points: Vec<FRPoint>,
}

impl DeltaTarget {
    /// Whether this target is a placeholder rather than a measured correction.
    pub fn is_placeholder(&self) -> bool {
        self.synthetic
    }

    /// The curve's value at a frequency.
    pub fn db_at(&self, hz: f32) -> Option<f32> {
        processing::interpolate_db(&self.points, hz)
    }
}

/// The difference between two rigs' calibrations, sampled onto the shared grid.
///
/// Both curves are read at the same frequencies before being subtracted, so
/// the result lives on `grid` regardless of how densely each calibration was
/// measured.
pub fn compensation(
    clone: &[FRPoint],
    original: &[FRPoint],
    grid: &[f32],
) -> Result<Vec<FRPoint>, DeltaError> {
    validate_grid(grid)?;
    if clone.is_empty() {
        return Err(DeltaError::EmptyCalibration("clone rig".to_owned()));
    }
    if original.is_empty() {
        return Err(DeltaError::EmptyCalibration("original rig".to_owned()));
    }
    grid.iter()
        .map(|&hz| {
            let clone_db = processing::interpolate_db(clone, hz).ok_or(DeltaError::Unsampleable)?;
            let original_db =
                processing::interpolate_db(original, hz).ok_or(DeltaError::Unsampleable)?;
            Ok(FRPoint {
                hz,
                db: clone_db - original_db,
            })
        })
        .collect()
}

/// Remove a rig's error from a measurement, in place.
///
/// A rig that reads `x` dB where the original reads `y` dB adds `x - y` to
/// everything it measures, so the correction is the same subtraction. Frequencies
/// the delta target does not reach are left untouched rather than clamped to
/// its edge, because at those frequencies nothing was measured to correct.
pub fn apply(compensation: &[FRPoint], curve: &mut [FRPoint]) -> Result<(), DeltaError> {
    if compensation.is_empty() {
        return Err(DeltaError::Unsampleable);
    }
    for point in curve {
        let Some(delta) = processing::interpolate_db(compensation, point.hz) else {
            return Err(DeltaError::Unsampleable);
        };
        point.db -= delta;
    }
    Ok(())
}

/// Read every delta target described by the manifest in `dir`.
///
/// A directory with no manifest is not an error: delta targets are optional,
/// and most installs have none. Calibration curves are read once and shared
/// between the delta targets that reference them.
pub fn load_delta_targets(dir: &Path, grid: &[f32]) -> Result<Vec<DeltaTarget>, DeltaError> {
    let manifest_path = dir.join(MANIFEST);
    if !manifest_path.is_file() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(&manifest_path)
        .map_err(|error| DeltaError::Io(format!("{}: {error}", manifest_path.display())))?;
    let manifest: DeltaManifest =
        serde_json::from_str(&raw).map_err(|error| DeltaError::Manifest(error.to_string()))?;

    let mut calibrations = BTreeMap::<String, Vec<FRPoint>>::new();
    for spec in &manifest.deltas {
        for name in [&spec.clone, &spec.original] {
            if calibrations.contains_key(name) {
                continue;
            }
            let path = dir.join(name);
            if !path.is_file() {
                return Err(DeltaError::MissingCalibration(name.clone()));
            }
            let content = std::fs::read_to_string(&path)
                .map_err(|error| DeltaError::Io(format!("{}: {error}", path.display())))?;
            let points = parse_calibration(&content);
            if points.is_empty() {
                return Err(DeltaError::EmptyCalibration(name.clone()));
            }
            calibrations.insert(name.clone(), points);
        }
    }

    let mut targets = Vec::with_capacity(manifest.deltas.len());
    for spec in manifest.deltas {
        let clone = &calibrations[&spec.clone];
        let original = &calibrations[&spec.original];
        let points = compensation(clone, original, grid)?;
        targets.push(DeltaTarget {
            name: spec.name,
            clone_rig: spec.clone,
            original_rig: spec.original,
            synthetic: spec.synthetic,
            note: spec.note,
            points,
        });
    }
    Ok(targets)
}

/// A calibration curve's points: whitespace-separated frequency and dB, with
/// any further columns ignored, matching how the database reads a curve.
fn parse_calibration(content: &str) -> Vec<FRPoint> {
    content
        .lines()
        .filter_map(crate::data::parse_point)
        .collect()
}

fn validate_grid(grid: &[f32]) -> Result<(), DeltaError> {
    if grid.is_empty()
        || !grid.iter().all(|hz| hz.is_finite() && *hz > 0.0)
        || grid.windows(2).any(|pair| pair[1] <= pair[0])
    {
        return Err(DeltaError::InvalidGrid);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(db: f32, points: usize) -> Vec<FRPoint> {
        (0..points)
            .map(|i| {
                let hz = 20.0 * 10.0f32.powf(i as f32 * 3.0 / (points - 1) as f32);
                FRPoint { hz, db }
            })
            .collect()
    }

    /// A stand-in for the app's shared grid, coarse enough to keep the tests
    /// readable.
    fn grid() -> Vec<f32> {
        (0..33)
            .map(|i| 20.0 * 10.0f32.powf(i as f32 * 3.0 / 32.0))
            .collect()
    }

    #[test]
    fn a_compensation_is_the_clone_minus_the_original() {
        let clone = flat(2.0, 16);
        let original = flat(0.5, 16);
        let points = compensation(&clone, &original, &grid()).unwrap();
        // The rig reads 1.5 dB high everywhere, so that is what it adds to a
        // headphone measured on it, and that is what has to come back off.
        assert!(points.iter().all(|point| (point.db - 1.5).abs() < 1e-4));
    }

    #[test]
    fn a_compensation_is_zero_when_both_rigs_agree() {
        let rig = flat(-3.25, 16);
        let points = compensation(&rig, &rig, &grid()).unwrap();
        assert!(points.iter().all(|point| point.db.abs() < 1e-4));
    }

    #[test]
    fn applying_a_compensation_restores_the_original_reading() {
        let clone = flat(2.0, 16);
        let original = flat(0.5, 16);
        let delta = compensation(&clone, &original, &grid()).unwrap();
        // A headphone that is flat at 80 dB reads 82 dB on the clone.
        let mut measured = flat(82.0, 32);
        apply(&delta, &mut measured).unwrap();
        assert!(measured.iter().all(|point| (point.db - 80.5).abs() < 1e-3));
    }

    #[test]
    fn both_calibrations_are_read_at_the_same_frequencies() {
        // The clone is sampled sparsely and the original densely; the result is
        // still sampled wherever the grid says, because each curve is read at
        // the grid's frequencies rather than the other way round.
        let sparse = vec![
            FRPoint { hz: 20.0, db: 1.0 },
            FRPoint {
                hz: 1000.0,
                db: 1.0,
            },
            FRPoint {
                hz: 20000.0,
                db: 1.0,
            },
        ];
        let original = flat(0.0, 16);
        let points = compensation(&sparse, &original, &grid()).unwrap();
        assert_eq!(points.len(), grid().len());
        assert!(points.iter().all(|point| (point.db - 1.0).abs() < 1e-4));
    }

    #[test]
    fn a_calibration_that_stops_early_is_held_flat_rather_than_extrapolated() {
        // The clone was only calibrated to 2 kHz. Above that it must contribute
        // nothing beyond its last measured value, not a continued slope.
        let truncated = flat(2.0, 8);
        let original = flat(0.0, 8);
        let wide = grid();
        let points = compensation(&truncated, &original, &wide).unwrap();
        let above = wide
            .iter()
            .position(|hz| *hz >= 2000.0)
            .expect("the grid reaches past the calibration");
        assert!(points[above..]
            .iter()
            .all(|point| (point.db - 2.0).abs() < 1e-4));
    }

    #[test]
    fn a_grid_that_does_not_increase_is_rejected() {
        let clone = flat(1.0, 8);
        let original = flat(0.0, 8);
        assert_eq!(
            compensation(&clone, &original, &[1000.0, 100.0]).unwrap_err(),
            DeltaError::InvalidGrid
        );
        assert_eq!(
            compensation(&clone, &original, &[]).unwrap_err(),
            DeltaError::InvalidGrid
        );
        assert_eq!(
            compensation(&clone, &original, &[0.0, 100.0]).unwrap_err(),
            DeltaError::InvalidGrid
        );
    }

    #[test]
    fn a_calibration_with_no_points_is_rejected_by_name() {
        let original = flat(0.0, 8);
        assert_eq!(
            compensation(&[], &original, &grid()).unwrap_err(),
            DeltaError::EmptyCalibration("clone rig".to_owned())
        );
        assert_eq!(
            compensation(&original, &[], &grid()).unwrap_err(),
            DeltaError::EmptyCalibration("original rig".to_owned())
        );
    }

    #[test]
    fn a_directory_without_a_manifest_has_no_delta_targets() {
        let dir = std::env::temp_dir().join("squiglab-delta-absent");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // Delta targets are optional, so an empty directory is not a failure.
        assert!(load_delta_targets(&dir, &grid()).unwrap().is_empty());
    }

    #[test]
    fn a_manifest_builds_its_delta_targets_from_the_calibrations() {
        let dir = std::env::temp_dir().join("squiglab-delta-manifest");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("clone.txt"), "20\t1.0\n1000\t1.0\n20000\t1.0\n").unwrap();
        std::fs::write(
            dir.join("original.txt"),
            "20\t0.25\n1000\t0.25\n20000\t0.25\n",
        )
        .unwrap();
        std::fs::write(
            dir.join(MANIFEST),
            r#"{"deltas":[{"name":"Δ Clone","clone":"clone.txt","original":"original.txt",
                "synthetic":true,"note":"placeholder"}]}"#,
        )
        .unwrap();

        let targets = load_delta_targets(&dir, &grid()).unwrap();
        assert_eq!(targets.len(), 1);
        let delta = &targets[0];
        assert_eq!(delta.name, "Δ Clone");
        assert!(delta.is_placeholder());
        assert_eq!(delta.note.as_deref(), Some("placeholder"));
        assert!(delta
            .points
            .iter()
            .all(|point| (point.db - 0.75).abs() < 1e-4));
        // The correction reads back at any frequency the grid reaches.
        assert!((delta.db_at(100.0).unwrap() - 0.75).abs() < 1e-4);
    }

    #[test]
    fn a_manifest_naming_a_missing_calibration_is_an_error() {
        let dir = std::env::temp_dir().join("squiglab-delta-missing");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(MANIFEST),
            r#"{"deltas":[{"name":"Δ Clone","clone":"absent.txt","original":"original.txt"}]}"#,
        )
        .unwrap();
        assert_eq!(
            load_delta_targets(&dir, &grid()).unwrap_err(),
            DeltaError::MissingCalibration("absent.txt".to_owned())
        );
    }

    #[test]
    fn the_repository_tracks_no_calibration_curves() {
        // The delta directory holds the user's own measurement data, and it is
        // ignored so a stray `git add -A` cannot publish it. This pins the other
        // half of that bargain: nothing calibration-shaped may be *tracked*, so a
        // new rig in here is a leak rather than a feature. Without it the only
        // thing standing between a new rig's measurements and the repository is
        // someone remembering to check .gitignore.
        //
        // It asks git rather than looking at the directory, because the
        // calibrations are legitimately still present on disk — an installed
        // copy uses them — and ignoring them is what the working tree is for.
        // The core crate sits one level below the repository root, which is
        // where the data directories live.
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("the core crate sits inside the repository");
        let dir = root.join("data_deltas");
        let _ = &dir;
        let Ok(output) = std::process::Command::new("git")
            .args(["ls-files", "-z", "--", dir.to_str().unwrap()])
            .current_dir(root)
            .output()
        else {
            // No git in the test environment, so there is nothing to check.
            return;
        };
        if !output.status.success() {
            return;
        }
        // Paths come back as git printed them — relative to the repository root
        // — so the name is taken rather than a prefix that may not have matched.
        let tracked: Vec<String> = std::str::from_utf8(&output.stdout)
            .unwrap()
            .split('\0')
            .filter(|path| !path.is_empty())
            .filter_map(|path| {
                Path::new(path)
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .collect();
        // Only documentation and the manifest template. A calibration curve or a
        // manifest in here is data that should never have been committed.
        for name in &tracked {
            assert!(
                name == "README.md" || name == "delta_targets.example.json",
                "{name} is committed in data_deltas/, which holds the user's own rig data"
            );
        }
        assert!(
            tracked
                .iter()
                .any(|name| name == "delta_targets.example.json"),
            "the delta directory should still ship a manifest to start from"
        );
    }

    #[test]
    fn applying_an_empty_compensation_is_refused() {
        let mut curve = flat(80.0, 8);
        assert_eq!(apply(&[], &mut curve), Err(DeltaError::Unsampleable));
    }
}
