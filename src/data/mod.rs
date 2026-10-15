//! Data structures and loading for CrinGraph measurement files

use serde::Deserialize;
use serde_json::Value;
use std::path::Path;
use walkdir::WalkDir;

/// Parse one line of a curve file as a frequency and dB pair.
///
/// Columns may be separated by whitespace or by commas. The database ships
/// tab-separated files, but an export that comes back as CSV — or a
/// comma-separated dump from another tool — has to load the same, or the app
/// would refuse its own exports.
pub fn parse_point(line: &str) -> Option<FRPoint> {
    let mut columns = line.split(|c: char| c.is_whitespace() || c == ',');
    let hz = columns.next()?.parse::<f32>().ok()?;
    let db = columns.next()?.parse::<f32>().ok()?;
    (hz.is_finite() && hz > 0.0 && db.is_finite()).then_some(FRPoint { hz, db })
}

/// A single frequency response point
#[derive(Clone, Debug)]
pub struct FRPoint {
    pub hz: f32,
    pub db: f32,
}

/// A curve from a single measurement file
#[derive(Clone, Debug)]
pub struct CurveInfo {
    pub model: String,
    pub variant: String,
    pub points: Vec<FRPoint>,
}

/// Phone info from phone_book.json
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhoneInfo {
    pub brand: String,
    pub disp_brand: String,
    pub phone: String,
    pub full_name: String,
    pub file_name: String,
    pub disp_name: String,
    pub file_names: Vec<String>,
    pub vars: Option<serde_json::Value>,
    pub collab: Option<String>,
    pub suffix: Option<String>,
    pub prefix: Option<String>,
    pub disp_names: Option<Vec<String>>,
    pub is_target: bool,
}

/// Brand info from phone_book.json
#[derive(Clone, Debug, Deserialize)]
pub struct BrandInfo {
    pub name: String,
    pub phones: Option<serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhoneBook {
    pub brands: Vec<PhoneBrand>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhoneBrand {
    pub name: String,
    pub suffix: Option<String>,
    pub models: Vec<PhoneModel>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhoneModel {
    pub name: String,
    pub variants: Vec<PhoneVariant>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhoneVariant {
    pub name: String,
    pub file_stem: String,
}

/// Read the web app's flexible brand/model/variant JSON into native selector data.
pub fn load_phone_book(path: &Path) -> anyhow::Result<PhoneBook> {
    let json: Value = serde_json::from_slice(&std::fs::read(path)?)?;
    parse_phone_book(&json)
}

fn parse_phone_book(json: &Value) -> anyhow::Result<PhoneBook> {
    let brands = json
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("phone book must be an array of brands"))?;
    let mut output = Vec::with_capacity(brands.len());
    for brand in brands {
        let name = brand
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("phone book brand is missing a name"))?;
        let models = brand
            .get("phones")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow::anyhow!("phone book brand {name} is missing its phones list"))?
            .iter()
            .map(|phone| parse_phone_model(phone, name))
            .collect::<anyhow::Result<Vec<_>>>()?;
        output.push(PhoneBrand {
            name: name.to_owned(),
            suffix: brand
                .get("suffix")
                .and_then(Value::as_str)
                .map(str::to_owned),
            models,
        });
    }
    Ok(PhoneBook { brands: output })
}

fn parse_phone_model(phone: &Value, brand_name: &str) -> anyhow::Result<PhoneModel> {
    let (names, files, suffixes, prefix) = if let Some(name) = phone.as_str() {
        (
            vec![name.to_owned()],
            vec![name.to_owned()],
            Vec::new(),
            None,
        )
    } else {
        let object = phone
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("invalid model entry for brand {brand_name}"))?;
        let names = string_or_array(object.get("name"))
            .ok_or_else(|| anyhow::anyhow!("model in {brand_name} is missing its name"))?;
        let files = string_or_array(object.get("file")).unwrap_or_else(|| names.clone());
        let suffixes = string_or_array(object.get("suffix")).unwrap_or_default();
        let prefix = object
            .get("prefix")
            .and_then(Value::as_str)
            .map(str::to_owned);
        (names, files, suffixes, prefix)
    };
    if files.is_empty() {
        anyhow::bail!("model in {brand_name} has no measurement files");
    }
    let model_name = names.first().cloned().unwrap_or_else(|| files[0].clone());
    let variants = files
        .iter()
        .enumerate()
        .map(|(index, file_stem)| {
            let name = suffixes
                .get(index)
                .map(|suffix| {
                    if suffix.is_empty() {
                        model_name.clone()
                    } else {
                        format!("{model_name} {suffix}")
                    }
                })
                .or_else(|| {
                    prefix.as_ref().map(|prefix| {
                        let display_suffix = file_stem
                            .get(prefix.len()..)
                            .filter(|_| file_stem.starts_with(prefix))
                            .unwrap_or(file_stem);
                        if display_suffix.is_empty() {
                            model_name.clone()
                        } else {
                            format!("{model_name} {display_suffix}")
                        }
                    })
                })
                .or_else(|| {
                    names
                        .get(index)
                        .filter(|_| names.len() == files.len())
                        .cloned()
                })
                .or_else(|| (files.len() > 1).then(|| file_stem.clone()))
                .unwrap_or_else(|| model_name.clone());
            PhoneVariant {
                name,
                file_stem: file_stem.clone(),
            }
        })
        .collect();
    Ok(PhoneModel {
        name: model_name,
        variants,
    })
}

fn string_or_array(value: Option<&Value>) -> Option<Vec<String>> {
    match value? {
        Value::String(value) => Some(vec![value.clone()]),
        Value::Array(values) => Some(
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
        ),
        _ => None,
    }
}

/// Load measurement data from data/ directory
pub fn load_data(data_dir: &Path) -> anyhow::Result<(Vec<CurveInfo>, Vec<f32>)> {
    let mut curves = Vec::new();

    // Walk the data directory for .txt files
    for entry in WalkDir::new(data_dir) {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) == Some("txt") {
            let content = std::fs::read_to_string(path)?;
            let points: Vec<FRPoint> = content.lines().filter_map(parse_point).collect();

            if !points.is_empty() {
                let stem = path
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .unwrap_or("unknown");
                let (model, variant) = split_model_variant(stem);
                curves.push(CurveInfo {
                    model,
                    variant,
                    points,
                });
            }
        }
    }

    // Sort curves by model name
    curves.sort_by(|a, b| a.model.cmp(&b.model).then(a.variant.cmp(&b.variant)));

    // Build shared frequency grid: 490 points from 20Hz to 20kHz, log-spaced
    let f_values: Vec<f32> = (0..490)
        .map(|i| {
            let ratio = 10.0f32.powf(i as f32 * 4.0 / (490 - 1) as f32 - 2.0);
            20.0 * ratio
        })
        .collect();

    Ok((curves, f_values))
}

/// Fewest points a measurement may have to be accepted. A REW or AudioTool
/// export below this is a fragment rather than a measurement, and drawing it
/// against a full sweep would show a curve that was never measured. The
/// reference tool refuses the same files.
pub const MIN_UPLOAD_POINTS: usize = 128;

/// What an uploaded file turned out to be.
#[derive(Clone, Debug)]
pub struct UploadedCurve {
    /// The curve, on the frequency grid it was resampled onto.
    pub curve: CurveInfo,
    /// Whether the file was uploaded as a target rather than a measurement.
    pub target: bool,
}

/// Why an uploaded file was not accepted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UploadError {
    /// The file had no readable frequency and level columns.
    NoColumns,
    /// The file parsed, but has fewer than [`MIN_UPLOAD_POINTS`] points.
    TooFewPoints(usize),
    /// The file's points were not a rising sweep.
    NotMonotonic,
}

impl std::fmt::Display for UploadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoColumns => write!(
                formatter,
                "no frequency and level columns were found; expected a two-column \
                 REW or AudioTool export"
            ),
            Self::TooFewPoints(count) => write!(
                formatter,
                "only {count} points were read, and a measurement needs at least \
                 {MIN_UPLOAD_POINTS}"
            ),
            Self::NotMonotonic => {
                write!(formatter, "the frequencies do not rise through the file")
            }
        }
    }
}

impl std::error::Error for UploadError {}

/// Read an uploaded REW or AudioTool measurement, or target.
///
/// Both formats are whitespace- or tab-separated frequency and level columns
/// with optional headers, so one parser covers them. The points are resampled
/// onto `grid` so an uploaded curve can be compared with the database's, and
/// a file whose name ends in " R" has a null left channel spliced in front of
/// its right channel, so the two line up on the same frequencies.
///
/// `file_name` is the measurement's name in the UI, exactly as the reference
/// tool uses it: the file's name, with " Target" appended for a target upload.
pub fn parse_upload(
    file_name: &str,
    contents: &str,
    grid: &[f32],
    target: bool,
) -> Result<UploadedCurve, UploadError> {
    let mut points: Vec<FRPoint> = Vec::new();
    for line in contents.lines() {
        // REW writes tab-separated columns, AudioTool writes comma-separated
        // ones, and either may pad with spaces. Treating every separator as
        // whitespace covers both.
        let line = line.replace(',', " ");
        let mut columns = line.split_whitespace();
        let (Some(frequency), Some(level)) = (columns.next(), columns.next()) else {
            // A header line, a blank line, or a comment: skip it and keep
            // reading, which is what both formats need.
            continue;
        };
        let (Ok(frequency), Ok(level)) = (frequency.parse::<f32>(), level.parse::<f32>()) else {
            continue;
        };
        if !frequency.is_finite() || !level.is_finite() || frequency <= 0.0 {
            continue;
        }
        // A descending sweep is the other channel's file, not this one's.
        if points.last().is_some_and(|last| frequency <= last.hz) {
            return Err(UploadError::NotMonotonic);
        }
        points.push(FRPoint {
            hz: frequency,
            db: level,
        });
    }
    if points.len() < MIN_UPLOAD_POINTS {
        return Err(UploadError::TooFewPoints(points.len()));
    }

    let is_right_channel = file_name.trim_end().ends_with(" R");
    let model = if target {
        format!("{file_name} Target")
    } else {
        file_name.to_owned()
    };
    let resampled = resample(&points, grid)?;
    Ok(UploadedCurve {
        curve: CurveInfo {
            model,
            // The variant carries the channel, as it does in the database. A
            // right-only file is prefixed with a null left channel so both
            // channels sit on the same frequencies.
            variant: if is_right_channel {
                "R".to_owned()
            } else {
                String::new()
            },
            points: resampled,
        },
        target,
    })
}

/// Resample a curve onto a shared grid, piecewise-linearly in log frequency,
/// which is how the reference tool's `interp` places a measurement on the same
/// frequencies as the database's.
///
/// A right-channel-only measurement is returned as its right channel here; the
/// caller keeps the left channel null, so the two cannot disagree on
/// frequency.
fn resample(points: &[FRPoint], grid: &[f32]) -> Result<Vec<FRPoint>, UploadError> {
    if points.is_empty() {
        return Err(UploadError::TooFewPoints(0));
    }
    let output = crate::equalizer::interp(
        &grid
            .iter()
            .map(|frequency| f64::from(*frequency))
            .collect::<Vec<_>>(),
        &points
            .iter()
            .map(|point| {
                crate::equalizer::ResponsePoint::new(f64::from(point.hz), f64::from(point.db))
            })
            .collect::<Vec<_>>(),
    );
    Ok(output
        .into_iter()
        .map(|point| FRPoint {
            hz: point.hz as f32,
            db: point.db as f32,
        })
        .collect())
}

fn split_model_variant(stem: &str) -> (String, String) {
    let mut split = stem.rsplitn(2, char::is_whitespace);
    let last = split.next().unwrap_or(stem);
    let model = split.next().unwrap_or("");
    let is_variant = last == "L"
        || last == "R"
        || last == "Avg"
        || (last.len() > 1
            && matches!(last.as_bytes()[0], b'L' | b'R')
            && last[1..].bytes().all(|digit| digit.is_ascii_digit()));
    if is_variant && !model.is_empty() {
        (model.to_owned(), last.to_owned())
    } else {
        (stem.to_owned(), String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        parse_phone_book, parse_upload, split_model_variant, UploadError, MIN_UPLOAD_POINTS,
    };
    use serde_json::json;
    use std::path::Path;

    #[test]
    fn model_names_keep_spaces_and_sample_numbers_are_variants() {
        assert_eq!(
            split_model_variant("Combo Variant 1 L"),
            ("Combo Variant 1".into(), "L".into())
        );
        assert_eq!(
            split_model_variant("DD H4 R5"),
            ("DD H4".into(), "R5".into())
        );
        assert_eq!(
            split_model_variant("Harman Target"),
            ("Harman Target".into(), String::new())
        );
    }

    #[test]
    fn phone_book_parses_brand_suffixes_and_variant_arrays() {
        let book = parse_phone_book(&json!([
            {
                "name": "Example",
                "suffix": "Audio",
                "phones": [
                    "Single",
                    {"name": "Reference", "file": ["Ref1", "Ref2"], "suffix": ["A", "B"]}
                ]
            }
        ]))
        .unwrap();
        assert_eq!(book.brands[0].suffix.as_deref(), Some("Audio"));
        assert_eq!(book.brands[0].models[0].variants[0].file_stem, "Single");
        assert_eq!(book.brands[0].models[1].variants[1].name, "Reference B");
        assert_eq!(book.brands[0].models[1].variants[1].file_stem, "Ref2");
    }

    #[test]
    fn project_phone_books_load_against_their_measurement_files() {
        let book = super::load_phone_book(Path::new("data/phone_book.json")).unwrap();
        assert!(book.brands.iter().any(|brand| brand.name == "GZR"));
        assert!(book.brands[0].models[0].variants.len() == 4);
        let headphone_book = super::load_phone_book(Path::new("data_hp/phone_book.json")).unwrap();
        assert!(headphone_book
            .brands
            .iter()
            .any(|brand| brand.name == "Groot"));
    }

    /// A shared measurement grid, 20 Hz to 20 kHz, matching the reference's
    /// 490-point resolution.
    fn grid() -> Vec<f32> {
        (0..490)
            .map(|index| 20.0 * 1000.0_f32.powf(index as f32 / 489.0))
            .collect()
    }

    /// A REW export: a header line, then a rising sweep.
    fn rew_export(points: usize) -> String {
        let mut text = String::from("Freq	Level\n");
        for index in 0..points {
            let frequency = 20.0 + index as f64 * 40.0;
            text.push_str(&format!(
                "{frequency:.3}\t{:.2}\n",
                70.0 - (index % 7) as f64
            ));
        }
        text
    }

    #[test]
    fn an_uploaded_measurement_lands_on_the_shared_grid() {
        let uploaded = parse_upload("My Headphones", &rew_export(500), &grid(), false).unwrap();
        assert!(!uploaded.target);
        assert_eq!(uploaded.curve.model, "My Headphones");
        // The file's own sweep is resampled onto the database's frequencies, so
        // an uploaded curve can be compared with a stored one.
        assert_eq!(uploaded.curve.points.len(), 490);
        assert!((uploaded.curve.points[0].hz - 20.0).abs() < 0.01);
        assert!((uploaded.curve.points[489].hz - 20_000.0).abs() < 1.0);
        // The levels survived: the sweep steps through seven values.
        let levels: Vec<f32> = uploaded.curve.points.iter().map(|point| point.db).collect();
        assert!(levels.iter().all(|level| (63.0..=70.0).contains(level)));
    }

    #[test]
    fn fewer_than_128_points_is_refused() {
        // The reference tool rejects these, and so does this one: a fragment
        // is not a measurement.
        let error =
            parse_upload("Short", &rew_export(MIN_UPLOAD_POINTS - 1), &grid(), false).unwrap_err();
        assert_eq!(error, UploadError::TooFewPoints(MIN_UPLOAD_POINTS - 1));
        assert!(error.to_string().contains("at least 128"));
        // Exactly the minimum is accepted.
        assert!(parse_upload(
            "Just enough",
            &rew_export(MIN_UPLOAD_POINTS),
            &grid(),
            false
        )
        .is_ok());
    }

    #[test]
    fn a_file_with_no_columns_is_refused() {
        let header_only = "This is a note about the file, not a measurement\n".repeat(200);
        assert_eq!(
            parse_upload("Notes", &header_only, &grid(), false).unwrap_err(),
            UploadError::TooFewPoints(0)
        );
    }

    #[test]
    fn a_target_upload_is_named_and_flagged_as_a_target() {
        let uploaded = parse_upload("Harman", &rew_export(300), &grid(), true).unwrap();
        assert!(uploaded.target);
        // " Target" is appended, which is how the database names its targets
        // and so how a deduplication pass recognises them.
        assert_eq!(uploaded.curve.model, "Harman Target");
    }

    #[test]
    fn a_right_channel_file_is_marked_as_the_right_channel() {
        let right = parse_upload("My Headphones R", &rew_export(300), &grid(), false).unwrap();
        assert_eq!(right.curve.variant, "R");
        // The name keeps its suffix, and the "R" is what the model splitter
        // would have taken off had this come through the database loader.
        assert_eq!(right.curve.model, "My Headphones R");
        // A file that is not right-only is a single unlabelled curve.
        let both = parse_upload("My Headphones", &rew_export(300), &grid(), false).unwrap();
        assert_eq!(both.curve.variant, "");
    }

    #[test]
    fn a_descending_sweep_is_refused() {
        let mut text = String::new();
        for index in (0..300).rev() {
            text.push_str(&format!("{:.3}\t60.0\n", 20.0 + index as f64 * 40.0));
        }
        assert_eq!(
            parse_upload("Descending", &text, &grid(), false).unwrap_err(),
            UploadError::NotMonotonic
        );
    }

    #[test]
    fn an_audio_tool_export_parses_like_a_rew_export() {
        // AudioTool writes comma-separated values with its own header.
        let mut text = String::from("Frequency, SPL\n");
        for index in 0..300 {
            text.push_str(&format!("{:.2}, {:.2}\n", 20.0 + index as f64 * 60.0, 65.0));
        }
        let uploaded = parse_upload("Headphones", &text, &grid(), false).unwrap();
        assert_eq!(uploaded.curve.points.len(), 490);
        assert!(uploaded.curve.points.iter().all(|point| point.db == 65.0));
    }

    #[test]
    fn the_repository_tracks_nothing_under_data_real() {
        // `data/real/` is the user's own database: the target curves and
        // measurements they actually use, which are neither the sample data the
        // repository ships nor anything it has any claim to. It is ignored, and
        // this pins the other half, because an ignore rule only protects a file
        // that has not already been added.
        //
        // It asks git rather than listing the directory, because the curves are
        // legitimately present on disk — they are what the app loads — and
        // ignoring them is what a working tree is for.
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let Ok(output) = std::process::Command::new("git")
            .args(["ls-files", "-z", "--", "data/real/"])
            .current_dir(root)
            .output()
        else {
            // No git in the test environment, so there is nothing to check.
            return;
        };
        if !output.status.success() {
            return;
        }
        let tracked: Vec<String> = std::str::from_utf8(&output.stdout)
            .unwrap()
            .split('\0')
            .filter(|path| !path.is_empty())
            .map(|path| {
                Path::new(path)
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default()
            })
            .collect();
        assert!(
            tracked.is_empty(),
            "data/real/ is the user's own database, but these are committed: {tracked:?}"
        );
    }
}
