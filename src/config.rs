use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Editor configuration.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(default)]
pub struct EditorConfig {
    /// Target line width for wrapping (64, 72, or 80).
    pub line_width: u16,
    /// Whether auto-save is enabled.
    pub auto_save: bool,
    /// Delay in milliseconds before auto-saving after a change.
    pub auto_save_delay_ms: u64,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            line_width: 72,
            auto_save: true,
            auto_save_delay_ms: 500,
        }
    }
}

/// Focus mode configuration.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(default)]
pub struct FocusConfig {
    /// Focus mode: "off", "sentence", "paragraph", or "typewriter".
    pub mode: String,
    /// Opacity for dimmed (unfocused) text, 10-60.
    pub opacity: u8,
}

impl Default for FocusConfig {
    fn default() -> Self {
        Self {
            mode: "off".to_string(),
            opacity: 30,
        }
    }
}

/// Theme configuration.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(default)]
pub struct ThemeConfig {
    /// Theme mode: "system", "light", or "dark".
    pub mode: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            mode: "system".to_string(),
        }
    }
}

/// File browser configuration.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(default)]
pub struct BrowserConfig {
    /// Whether to show hidden files/directories.
    pub show_hidden: bool,
    /// Layout ratio [browser, editor]. e.g. [1, 3] = browser 1/4, editor 3/4.
    pub ratio: [u32; 2],
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            show_hidden: false,
            ratio: [1, 3],
        }
    }
}

/// Top-level application configuration.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    pub editor: EditorConfig,
    pub focus: FocusConfig,
    pub theme: ThemeConfig,
    pub browser: BrowserConfig,
}

impl Config {
    /// Parse a TOML string into a Config, filling in defaults for missing fields.
    pub fn from_toml_str(s: &str) -> anyhow::Result<Self> {
        let config: Config = toml::from_str(s)?;
        Ok(config)
    }

    /// Return the default config file path: `~/.config/deepwrite/config.toml`.
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("deepwrite").join("config.toml"))
    }

    /// Load config from `~/.config/deepwrite/config.toml`.
    /// Falls back to defaults if the file does not exist or cannot be read.
    /// On first run, creates a template config with all options commented out.
    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };

        match fs::read_to_string(&path) {
            Ok(contents) => Self::from_toml_str(&contents).unwrap_or_default(),
            Err(_) => {
                // First run — write a template config so the user knows what's available.
                let _ = Self::write_template(&path);
                Self::default()
            }
        }
    }

    /// Write a commented-out template config file.
    fn write_template(path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let template = r#"# Deepwrite Configuration
# https://github.com/tomdhyang-byte/deepwrite-tui

[editor]
# line_width = 72        # 64 | 72 | 80
# auto_save = true
# auto_save_delay_ms = 500

[focus]
# mode = "off"           # off | sentence | paragraph | typewriter
# opacity = 30           # 10-60, dimming intensity

[theme]
# mode = "system"        # system | light | dark

[browser]
# show_hidden = false
# ratio = [1, 3]         # [browser, editor] panel ratio
"#;

        fs::write(path, template)?;
        Ok(())
    }
}
