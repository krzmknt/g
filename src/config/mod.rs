mod layout;
mod parser;
mod theme;

pub use layout::{Column, LayoutConfig, PanelHeight};
pub use theme::{Theme, HIGHLIGHT_COLORS};

use crate::error::Result;
use crate::tui::Color;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultDiffMode {
    Inline,
    Split,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultCommitsMode {
    Compact,
    Detailed,
    Graph,
}

#[derive(Debug, Clone)]
pub struct ViewDefaults {
    pub diff_mode: DefaultDiffMode,
    pub commits_mode: DefaultCommitsMode,
    pub branches_show_remote: bool,
    pub files_show_ignored: bool,
}

impl Default for ViewDefaults {
    fn default() -> Self {
        Self {
            diff_mode: DefaultDiffMode::Split,
            commits_mode: DefaultCommitsMode::Graph,
            branches_show_remote: true,
            files_show_ignored: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub theme: Theme,
    pub show_line_numbers: bool,
    pub diff_context_lines: u32,
    pub max_commits: usize,
    pub date_format: DateFormat,
    /// Git data refresh interval in seconds (status, branches). 0 = disabled.
    pub auto_refresh: u32,
    /// Auto fetch interval in seconds (fetch from all remotes). 0 = disabled.
    pub auto_fetch_interval: u32,
    /// GitHub API refresh interval in seconds (PRs, Issues, Actions, Releases). 0 = disabled.
    pub github_refresh_interval: u64,
    pub confirm_destructive: bool,
    pub editor: Option<String>,
    pub keybindings: HashMap<String, Vec<String>>,
    pub git: GitConfig,
    pub layout: LayoutConfig,
    pub view_defaults: ViewDefaults,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateFormat {
    Relative,
    Iso,
    Local,
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
            theme: Theme::default(),
            show_line_numbers: true,
            diff_context_lines: 3,
            max_commits: 1000,
            date_format: DateFormat::Relative,
            auto_refresh: 60,       // Default: refresh git data every 60 seconds
            auto_fetch_interval: 0, // Default: disabled (set to e.g. 300 for 5 min)
            github_refresh_interval: 60, // Default: refresh every 60 seconds
            confirm_destructive: true,
            editor: None,
            keybindings: HashMap::new(),
            git: GitConfig::default(),
            layout: LayoutConfig::default(),
            view_defaults: ViewDefaults::default(),
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
            PathBuf::from(home)
                .join(".config")
                .join("g")
                .join("config.toml")
        }
    }

    pub fn parse(content: &str) -> Result<Self> {
        let toml = parser::parse(content)?;
        let mut config = Self::default();

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

        if let Some(parser::Value::Integer(n)) = toml.get("auto_fetch_interval") {
            config.auto_fetch_interval = *n as u32;
        }

        if let Some(parser::Value::Integer(n)) = toml.get("github_refresh_interval") {
            config.github_refresh_interval = *n as u64;
        }

        if let Some(parser::Value::Boolean(b)) = toml.get("confirm_destructive") {
            config.confirm_destructive = *b;
        }

        if let Some(parser::Value::String(s)) = toml.get("editor") {
            config.editor = Some(s.clone());
        }

        // Parse layout config
        config.layout = LayoutConfig::from_toml(&toml);

        // Parse theme
        if let Some(parser::Value::Table(theme_table)) = toml.get("theme") {
            if let Some(parser::Value::String(s)) = theme_table.get("selection_color") {
                if let Some(color) = Color::from_hex(s) {
                    config.theme.selection = color;
                    config.theme.border_focused = color;
                }
            }
        }

        // Parse view defaults
        if let Some(parser::Value::Table(views)) = toml.get("views") {
            if let Some(parser::Value::String(s)) = views.get("diff_mode") {
                config.view_defaults.diff_mode = match s.as_str() {
                    "inline" => DefaultDiffMode::Inline,
                    "split" | "side_by_side" => DefaultDiffMode::Split,
                    _ => DefaultDiffMode::Split,
                };
            }
            if let Some(parser::Value::String(s)) = views.get("commits_mode") {
                config.view_defaults.commits_mode = match s.as_str() {
                    "compact" => DefaultCommitsMode::Compact,
                    "detailed" => DefaultCommitsMode::Detailed,
                    "graph" => DefaultCommitsMode::Graph,
                    _ => DefaultCommitsMode::Compact,
                };
            }
            if let Some(parser::Value::Boolean(b)) = views.get("branches_show_remote") {
                config.view_defaults.branches_show_remote = *b;
            }
            if let Some(parser::Value::Boolean(b)) = views.get("files_show_ignored") {
                config.view_defaults.files_show_ignored = *b;
            }
        }

        Ok(config)
    }

    pub fn save_highlight_color(&self) {
        let config_path = Self::config_path();

        if let Some(parent) = config_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let hex = match self.theme.selection.to_hex() {
            Some(h) => h,
            None => return,
        };

        let mut content = String::new();
        let mut found_theme_section = false;
        let mut replaced_selection = false;

        if let Ok(existing) = std::fs::read_to_string(&config_path) {
            let mut in_theme_section = false;
            for line in existing.lines() {
                if line.trim() == "[theme]" {
                    in_theme_section = true;
                    found_theme_section = true;
                    content.push_str(line);
                    content.push('\n');
                    continue;
                }
                if in_theme_section && line.trim_start().starts_with("selection_color") {
                    content.push_str(&format!("selection_color = \"{}\"\n", hex));
                    replaced_selection = true;
                    continue;
                }
                if in_theme_section && line.starts_with('[') {
                    if !replaced_selection {
                        content.push_str(&format!("selection_color = \"{}\"\n", hex));
                        replaced_selection = true;
                    }
                    in_theme_section = false;
                }
                content.push_str(line);
                content.push('\n');
            }
            if found_theme_section && !replaced_selection {
                content.push_str(&format!("selection_color = \"{}\"\n", hex));
            }
        }

        if !found_theme_section {
            if !content.is_empty() && !content.ends_with('\n') {
                content.push('\n');
            }
            content.push_str(&format!("\n[theme]\nselection_color = \"{}\"\n", hex));
        }

        let _ = std::fs::write(&config_path, content);
    }

    pub fn current_theme(&self) -> &Theme {
        &self.theme
    }

    pub fn editor(&self) -> String {
        self.editor
            .clone()
            .or_else(|| std::env::var("VISUAL").ok())
            .or_else(|| std::env::var("EDITOR").ok())
            .unwrap_or_else(|| "vi".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_selection_color_from_theme_section() {
        let content = r##"
[theme]
selection_color = "#89b4fa"
"##;
        let config = Config::parse(content).unwrap();
        assert_eq!(config.theme.selection, Color::Rgb(137, 180, 250));
    }

    #[test]
    fn test_parse_default_selection_color_without_theme_section() {
        let content = "";
        let config = Config::parse(content).unwrap();
        assert_eq!(config.theme.selection, Color::Rgb(255, 140, 0));
    }
}
