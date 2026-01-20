mod parser;
mod theme;

pub use theme::{Theme, ThemeName};

use std::path::PathBuf;
use std::collections::HashMap;
use crate::error::{Error, Result};
use crate::tui::Color;

#[derive(Debug, Clone)]
pub struct Config {
    pub theme: ThemeName,
    pub show_line_numbers: bool,
    pub diff_context_lines: u32,
    pub max_commits: usize,
    pub date_format: DateFormat,
    pub auto_refresh: u32,
    pub confirm_destructive: bool,
    pub editor: Option<String>,
    pub themes: Themes,
    pub keybindings: HashMap<String, Vec<String>>,
    pub git: GitConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateFormat {
    Relative,
    Iso,
    Local,
}

#[derive(Debug, Clone)]
pub struct Themes {
    pub dark: Theme,
    pub light: Theme,
}

#[derive(Debug, Clone)]
pub struct GitConfig {
    pub default_remote: String,
    pub default_branch: String,
    pub sign_commits: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: ThemeName::Dark,
            show_line_numbers: true,
            diff_context_lines: 3,
            max_commits: 1000,
            date_format: DateFormat::Relative,
            auto_refresh: 0,
            confirm_destructive: true,
            editor: None,
            themes: Themes::default(),
            keybindings: HashMap::new(),
            git: GitConfig::default(),
        }
    }
}

impl Default for Themes {
    fn default() -> Self {
        Self {
            dark: Theme::dark(),
            light: Theme::light(),
        }
    }
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            default_remote: "origin".to_string(),
            default_branch: "main".to_string(),
            sign_commits: false,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = Self::config_path();

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&path)?;
        Self::parse(&content)
    }

    pub fn config_path() -> PathBuf {
        #[cfg(windows)]
        {
            let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(appdata).join("g").join("config.toml")
        }

        #[cfg(not(windows))]
        {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".config").join("g").join("config.toml")
        }
    }

    pub fn parse(content: &str) -> Result<Self> {
        let toml = parser::parse(content)?;
        let mut config = Self::default();

        if let Some(value) = toml.get("theme") {
            if let parser::Value::String(s) = value {
                config.theme = match s.as_str() {
                    "dark" => ThemeName::Dark,
                    "light" => ThemeName::Light,
                    _ => ThemeName::Dark,
                };
            }
        }

        if let Some(parser::Value::Boolean(b)) = toml.get("show_line_numbers") {
            config.show_line_numbers = *b;
        }

        if let Some(parser::Value::Integer(n)) = toml.get("diff_context_lines") {
            config.diff_context_lines = *n as u32;
        }

        if let Some(parser::Value::Integer(n)) = toml.get("max_commits") {
            config.max_commits = *n as usize;
        }

        if let Some(parser::Value::String(s)) = toml.get("date_format") {
            config.date_format = match s.as_str() {
                "relative" => DateFormat::Relative,
                "iso" => DateFormat::Iso,
                "local" => DateFormat::Local,
                _ => DateFormat::Relative,
            };
        }

        if let Some(parser::Value::Integer(n)) = toml.get("auto_refresh") {
            config.auto_refresh = *n as u32;
        }

        if let Some(parser::Value::Boolean(b)) = toml.get("confirm_destructive") {
            config.confirm_destructive = *b;
        }

        if let Some(parser::Value::String(s)) = toml.get("editor") {
            config.editor = Some(s.clone());
        }

        Ok(config)
    }

    pub fn current_theme(&self) -> &Theme {
        match self.theme {
            ThemeName::Dark => &self.themes.dark,
            ThemeName::Light => &self.themes.light,
        }
    }

    pub fn toggle_theme(&mut self) {
        self.theme = match self.theme {
            ThemeName::Dark => ThemeName::Light,
            ThemeName::Light => ThemeName::Dark,
        };
    }

    pub fn editor(&self) -> String {
        self.editor.clone()
            .or_else(|| std::env::var("VISUAL").ok())
            .or_else(|| std::env::var("EDITOR").ok())
            .unwrap_or_else(|| "vi".to_string())
    }
}
