//! Configuration module
//!
//! Handles reading and writing application configuration.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Application language
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum Language {
    #[default]
    Japanese,
    English,
    ChineseSimplified,
    ChineseTraditional,
}

/// Key binding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBindings {
    /// Key to open settings window
    pub open_settings: String,
    /// Key to toggle overlay visibility
    pub toggle_overlay: String,
    /// Normal attack key
    pub normal_attack: String,
    /// Chain/link attack key
    pub chain_attack: String,
    /// Operator 1 skill key
    pub operator1_skill: String,
    /// Operator 2 skill key
    pub operator2_skill: String,
    /// Operator 3 skill key
    pub operator3_skill: String,
    /// Operator 4 skill key
    pub operator4_skill: String,
    /// Heavy attack key
    pub heavy_attack: String,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            open_settings: "Home".to_string(),
            toggle_overlay: "F1".to_string(),
            normal_attack: "MouseLeft".to_string(),
            chain_attack: "E".to_string(),
            operator1_skill: "1".to_string(),
            operator2_skill: "2".to_string(),
            operator3_skill: "3".to_string(),
            operator4_skill: "4".to_string(),
            heavy_attack: "MouseLeft".to_string(),
        }
    }
}

/// Overlay window settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlaySettings {
    /// Opacity (0.0 - 1.0, higher = more opaque)
    pub opacity: f32,
    /// X position
    pub x: i32,
    /// Y position
    pub y: i32,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
}

impl Default for OverlaySettings {
    fn default() -> Self {
        Self {
            opacity: 0.8,
            x: 100,
            y: 100,
            width: 400,
            height: 100,
        }
    }
}

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Language setting
    pub language: Language,
    /// Key bindings
    pub key_bindings: KeyBindings,
    /// Overlay settings
    pub overlay: OverlaySettings,
    /// Last loaded combo file path
    pub last_combo_file: Option<String>,
}

impl Config {
    /// Create a new default configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path).map_err(|e| ConfigError::IoError(e.to_string()))?;
        toml::from_str(&content).map_err(|e| ConfigError::ParseError(e.to_string()))
    }

    /// Save configuration to file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let content =
            toml::to_string_pretty(self).map_err(|e| ConfigError::SerializeError(e.to_string()))?;

        // Ensure parent directory exists
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent).map_err(|e| ConfigError::IoError(e.to_string()))?;
        }

        fs::write(path, content).map_err(|e| ConfigError::IoError(e.to_string()))
    }

    /// Get default config file path
    pub fn default_path() -> PathBuf {
        // Get executable directory
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                return exe_dir.join("config").join("General.toml");
            }
        }
        PathBuf::from("config/General.toml")
    }

    /// Load from default path or create default config
    pub fn load_or_default() -> Self {
        let path = Self::default_path();
        match Self::load(&path) {
            Ok(config) => config,
            Err(_) => {
                let config = Self::default();
                // Try to save default config
                let _ = config.save(&path);
                config
            }
        }
    }
}

/// Configuration errors
#[derive(Debug, Clone)]
pub enum ConfigError {
    IoError(String),
    ParseError(String),
    SerializeError(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::IoError(msg) => write!(f, "IO error: {}", msg),
            ConfigError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ConfigError::SerializeError(msg) => write!(f, "Serialize error: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(matches!(config.language, Language::Japanese));
        assert_eq!(config.key_bindings.open_settings, "Home");
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(
            parsed.key_bindings.open_settings,
            config.key_bindings.open_settings
        );
    }
}
