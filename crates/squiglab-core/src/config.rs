//! The application's configuration, its preferences, and the links that carry
//! them.
//!
//! These are rules of the program rather than of a picture: where the
//! configuration lives, what a valid set of preferences is, and how a view is
//! turned into a link that another installation can open.

use crate::session::Preferences;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The smoothing levels the program offers.
pub const MAX_SMOOTH_LEVEL: usize = 12;

/// The oldest configuration directories, kept so a view written by an earlier
/// name is still found.
pub const LEGACY_CONFIG_DIRECTORIES: [&str; 2] = ["cringraph", "CrinGraph-native"];

/// Share links issued before the rename. Still decoded so links people already
/// shared keep working.

#[derive(Clone, Debug, Deserialize, Default, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct AppConfig {
    pub measurements: MeasurementConfig,
    pub defaults: DefaultConfig,
    /// Optional provenance line drawn in the corner of the graph, e.g. the
    /// rig the measurements were taken on. The web tool labels every graph this
    /// way; here it is opt-in because the native app has no rig metadata of its
    /// own to fall back on.
    pub rig_description: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Default, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct MeasurementConfig {
    pub database_dir: Option<PathBuf>,
    pub headphone_database_dir: Option<PathBuf>,
    /// Where rig-compensation delta targets live. Kept out of `database_dir`
    /// on purpose: the calibration curves are not headphones, and the database
    /// loader would list them as phantom measurements.
    pub delta_dir: Option<PathBuf>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DefaultConfig {
    pub smoothing_octaves: usize,
}

impl Default for DefaultConfig {
    fn default() -> Self {
        Self {
            smoothing_octaves: 5,
        }
    }
}

pub fn config_root() -> anyhow::Result<PathBuf> {
    if let Some(override_directory) = std::env::var_os("SQUIGLAB_CONFIG_DIR") {
        if !override_directory.is_empty() {
            return Ok(PathBuf::from(override_directory));
        }
    }
    dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("the operating system did not provide a config directory"))
}
pub fn app_config_path() -> anyhow::Result<PathBuf> {
    let config_dir = config_root()?;
    Ok(config_dir.join(CONFIG_DIRECTORY_NAME).join("config.toml"))
}
pub fn preferences_path() -> anyhow::Result<PathBuf> {
    let config_dir = config_root()?;
    Ok(config_dir
        .join(CONFIG_DIRECTORY_NAME)
        .join("preferences.json"))
}
pub fn legacy_preferences_paths() -> anyhow::Result<Vec<PathBuf>> {
    let config_dir = config_root()?;
    Ok(LEGACY_CONFIG_DIRECTORIES
        .iter()
        .map(|directory| config_dir.join(directory).join("preferences.json"))
        .collect())
}
pub fn read_app_config_from(path: &Path) -> anyhow::Result<AppConfig> {
    match std::fs::read_to_string(path) {
        Ok(contents) => Ok(toml::from_str(&contents)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(AppConfig::default()),
        Err(error) => Err(error.into()),
    }
}
pub fn initialize_app_config_from(path: &Path) -> anyhow::Result<AppConfig> {
    match std::fs::read_to_string(path) {
        Ok(contents) => Ok(toml::from_str(&contents)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let config = default_app_config()?;
            let directory = path
                .parent()
                .ok_or_else(|| anyhow::anyhow!("configuration path has no parent directory"))?;
            std::fs::create_dir_all(directory)?;
            let contents = toml::to_string_pretty(&config)?;
            let mut file = match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
            {
                Ok(file) => file,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    return read_app_config_from(path);
                }
                Err(error) => return Err(error.into()),
            };
            use std::io::Write;
            file.write_all(contents.as_bytes())?;
            file.sync_all()?;
            Ok(config)
        }
        Err(error) => Err(error.into()),
    }
}
pub fn default_app_config() -> anyhow::Result<AppConfig> {
    let working_directory = std::env::current_dir()?;
    let headphone_database_dir = working_directory.join("data_hp");
    let delta_dir = working_directory.join("data_deltas");
    Ok(AppConfig {
        measurements: MeasurementConfig {
            database_dir: Some(working_directory.join("data")),
            headphone_database_dir: headphone_database_dir
                .is_dir()
                .then_some(headphone_database_dir),
            delta_dir: delta_dir.is_dir().then_some(delta_dir),
        },
        defaults: DefaultConfig::default(),
        rig_description: None,
    })
}
pub fn validate_app_config(config: &AppConfig) -> anyhow::Result<()> {
    if config.defaults.smoothing_octaves > MAX_SMOOTH_LEVEL {
        anyhow::bail!("configured smoothing_octaves must be between 0 and {MAX_SMOOTH_LEVEL}");
    }
    Ok(())
}
pub fn resolve_measurement_dir(
    configured: Option<&Path>,
    fallback: &Path,
    config_path: &Path,
) -> PathBuf {
    let Some(path) = configured else {
        return fallback.to_path_buf();
    };
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        config_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(path)
    }
}
pub fn read_preferences() -> anyhow::Result<Option<Preferences>> {
    if let Some(preferences) = read_preferences_from(&preferences_path()?)? {
        return Ok(Some(preferences));
    }
    for path in legacy_preferences_paths()? {
        if let Some(preferences) = read_preferences_from(&path)? {
            return Ok(Some(preferences));
        }
    }
    Ok(None)
}
pub fn read_preferences_from(path: &Path) -> anyhow::Result<Option<Preferences>> {
    match std::fs::read(path) {
        Ok(contents) => Ok(Some(serde_json::from_slice(&contents)?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}
pub fn write_preferences_to(path: &Path, preferences: &Preferences) -> anyhow::Result<()> {
    let directory = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("preferences path has no parent directory"))?;
    std::fs::create_dir_all(directory)?;
    let temporary_path = path.with_extension("json.tmp");
    let serialized = serde_json::to_vec_pretty(preferences)?;
    let mut file = std::fs::File::create(&temporary_path)?;
    use std::io::Write;
    file.write_all(&serialized)?;
    file.sync_all()?;
    std::fs::rename(temporary_path, path)?;
    Ok(())
}
pub fn encode_share_uri(preferences: &Preferences) -> anyhow::Result<String> {
    let serialized = serde_json::to_vec(preferences)?;
    let mut uri = String::with_capacity(SHARE_URI_PREFIX.len() + serialized.len() * 2);
    uri.push_str(SHARE_URI_PREFIX);
    for byte in serialized {
        use std::fmt::Write;
        write!(uri, "{byte:02x}")?;
    }
    if uri.len() > MAX_SHARE_URI_LENGTH {
        anyhow::bail!("shared state exceeds the maximum URI length");
    }
    Ok(uri)
}
pub fn decode_share_uri(uri: &str) -> anyhow::Result<Preferences> {
    let payload = uri
        .strip_prefix(SHARE_URI_PREFIX)
        .or_else(|| uri.strip_prefix(LEGACY_SHARE_URI_PREFIX))
        .ok_or_else(|| anyhow::anyhow!("unsupported share URI"))?;
    if payload.is_empty() || payload.len() % 2 != 0 || uri.len() > MAX_SHARE_URI_LENGTH {
        anyhow::bail!("share URI payload has an invalid length");
    }
    let bytes = payload
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let high = (pair[0] as char)
                .to_digit(16)
                .ok_or_else(|| anyhow::anyhow!("share URI contains a non-hex character"))?;
            let low = (pair[1] as char)
                .to_digit(16)
                .ok_or_else(|| anyhow::anyhow!("share URI contains a non-hex character"))?;
            Ok(((high << 4) | low) as u8)
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(serde_json::from_slice(&bytes)?)
}
pub const CONFIG_DIRECTORY_NAME: &str = "squiglab";
pub const SHARE_URI_PREFIX: &str = "squiglab://state/v1/";
/// Share links issued before the rename. Still decoded so links people already
/// shared keep working.
pub const LEGACY_SHARE_URI_PREFIX: &str = "cringraph://state/v1/";
pub const MAX_SHARE_URI_LENGTH: usize = 65_536;
