//! Data structures and loading for CrinGraph measurement files

use serde::Deserialize;
use serde_json::Value;
use std::path::Path;
use walkdir::WalkDir;

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
            let points: Vec<FRPoint> = content
                .lines()
                .filter_map(|line| {
                    let mut columns = line.split_whitespace();
                    let hz = columns.next()?.parse::<f32>().ok()?;
                    let db = columns.next()?.parse::<f32>().ok()?;
                    (hz.is_finite() && hz > 0.0 && db.is_finite()).then_some(FRPoint { hz, db })
                })
                .collect();

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
    use super::{parse_phone_book, split_model_variant};
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
}
