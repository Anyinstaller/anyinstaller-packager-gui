//! GUI-only settings, kept outside the AIP configuration.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::market::DEFAULT_GITHUB_ACCELERATOR;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GuiSettings {
    pub github_acceleration: GithubAccelerationSettings,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GithubAccelerationSettings {
    pub enabled: bool,
    pub prefix: String,
}

impl Default for GithubAccelerationSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            prefix: DEFAULT_GITHUB_ACCELERATOR.into(),
        }
    }
}

pub fn load() -> GuiSettings {
    let path = settings_path();
    let Ok(text) = std::fs::read_to_string(path) else {
        return GuiSettings::default();
    };
    toml::from_str(&text).unwrap_or_default()
}

pub fn save(settings: &GuiSettings) -> Result<PathBuf> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("无法创建设置目录：{}", parent.display()))?;
    }
    let text = toml::to_string_pretty(settings).context("无法序列化 GUI 设置")?;
    std::fs::write(&path, text.as_bytes())
        .with_context(|| format!("无法写入 GUI 设置：{}", path.display()))?;
    Ok(path)
}

pub fn settings_path() -> PathBuf {
    data_root().join("settings.toml")
}

pub fn data_root() -> PathBuf {
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        Path::new(&local).join("Anyinstaller").join("PackagerGUI")
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".anyinstaller-packager-gui")
    }
}
