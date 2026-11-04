use std::{collections::BTreeMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use super::export::Adv;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Size,
    Quality,
    Lossless,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Codec {
    H264,
    Hevc,
    Av1,
}

impl Codec {
    pub fn id(self) -> &'static str {
        match self {
            Codec::H264 => "h264",
            Codec::Hevc => "hevc",
            Codec::Av1 => "av1",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Codec::H264 => "H.264",
            Codec::Hevc => "HEVC",
            Codec::Av1 => "AV1",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SegmentsPref {
    Ask,
    Merge,
    Separate,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Preset {
    pub name: String,
    pub adv: Adv,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub output_dir: String,
    pub name_template: String,
    pub default_mode: Mode,
    pub default_limit: f64,
    pub custom_limit: f64,
    pub default_codec: Codec,
    pub segments: SegmentsPref,
    pub copy_clipboard: bool,
    pub reveal: bool,
    pub hwdec: String,
    pub resume: bool,
    pub auto_hide_ms: u64,
    pub adv_open: bool,
    pub show_command: bool,
    pub volume: f64,
    pub adv: Adv,
    pub presets: Vec<Preset>,
    pub positions: BTreeMap<String, f64>,
    pub counters: BTreeMap<String, u32>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            output_dir: dirs::video_dir()
                .unwrap_or_else(std::env::temp_dir)
                .join("Clips")
                .to_string_lossy()
                .into_owned(),
            name_template: "{name}_clip{n}".into(),
            default_mode: Mode::Size,
            default_limit: 20.0,
            custom_limit: 25.0,
            default_codec: Codec::H264,
            segments: SegmentsPref::Ask,
            copy_clipboard: true,
            reveal: false,
            hwdec: "auto-safe".into(),
            resume: true,
            auto_hide_ms: 2000,
            adv_open: false,
            show_command: true,
            volume: 100.0,
            adv: Adv::default(),
            presets: Vec::new(),
            positions: BTreeMap::new(),
            counters: BTreeMap::new(),
        }
    }
}

fn path() -> PathBuf {
    dirs::config_dir().unwrap_or_else(std::env::temp_dir).join("Clipdat").join("settings.json")
}

pub fn cache_dir() -> PathBuf {
    dirs::data_local_dir().unwrap_or_else(std::env::temp_dir).join("Clipdat")
}

impl Settings {
    pub fn load() -> Settings {
        std::fs::read_to_string(path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Atomic write (tmp + rename) so a crash never leaves a half-written file.
    pub fn save(&self) {
        let p = path();
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        if let Ok(json) = serde_json::to_vec_pretty(self) {
            let tmp = p.with_extension("json.tmp");
            if std::fs::write(&tmp, json).is_ok() {
                let _ = std::fs::rename(tmp, p);
            }
        }
    }
}
