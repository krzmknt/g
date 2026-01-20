# Configuration System Design

## Overview

Configuration file at `~/.config/g/config.toml`. Custom TOML parser (no external dependencies).

## Configuration File Location

| Platform | Path                      |
| -------- | ------------------------- |
| Linux    | `~/.config/g/config.toml` |
| macOS    | `~/.config/g/config.toml` |
| Windows  | `%APPDATA%\g\config.toml` |

```rust
pub fn config_path() -> PathBuf {
    #[cfg(windows)]
    {
        let appdata = std::env::var("APPDATA")
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(appdata).join("g").join("config.toml")
    }

    #[cfg(not(windows))]
    {
        let home = std::env::var("HOME")
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".config").join("g").join("config.toml")
    }
}
```

## Default Configuration

```toml
# g - Git TUI Configuration

# Theme: "dark" or "light"
theme = "dark"

# Show line numbers in diff view
show_line_numbers = true

# Number of context lines in diff
diff_context_lines = 3

# Maximum commits to load initially
max_commits = 1000

# Date format for commits
# Supports: "relative", "iso", "local"
date_format = "relative"

# Auto-refresh interval in seconds (0 = disabled)
auto_refresh = 0

# Confirm before destructive operations
confirm_destructive = true

# Editor for commit messages (defaults to $EDITOR)
# editor = "vim"

[theme.dark]
# Base colors
background = "#1e1e2e"
foreground = "#cdd6f4"

# UI elements
border = "#6c7086"
border_focused = "#89b4fa"
selection = "#313244"
selection_text = "#cdd6f4"

# Syntax colors
diff_add = "#a6e3a1"
diff_remove = "#f38ba8"
diff_hunk = "#89dceb"

# Status colors
staged = "#a6e3a1"
unstaged = "#f9e2af"
untracked = "#6c7086"

# Branch colors
branch_current = "#a6e3a1"
branch_local = "#89b4fa"
branch_remote = "#cba6f7"

[theme.light]
background = "#eff1f5"
foreground = "#4c4f69"
border = "#9ca0b0"
border_focused = "#1e66f5"
selection = "#ccd0da"
selection_text = "#4c4f69"
diff_add = "#40a02b"
diff_remove = "#d20f39"
diff_hunk = "#04a5e5"
staged = "#40a02b"
unstaged = "#df8e1d"
untracked = "#9ca0b0"
branch_current = "#40a02b"
branch_local = "#1e66f5"
branch_remote = "#8839ef"

[keybindings]
# Custom keybindings (see keybindings.md for full list)
# Format: action = ["key1", "key2", ...]

[keybindings.global]
# quit = ["q", "Ctrl+c"]
# help = ["?"]

[keybindings.status]
# stage = ["Enter", "Space"]

[keybindings.branches]
# checkout = ["Enter"]

[keybindings.commits]
# details = ["Enter"]

[keybindings.diff]
# stage_hunk = ["Enter"]

[git]
# Default remote name
default_remote = "origin"

# Default branch for new repos
default_branch = "main"

# Sign commits with GPG
sign_commits = false

# GPG key ID (if sign_commits = true)
# gpg_key = "ABCD1234"
```

## Configuration Structure

```rust
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
    pub keybindings: Option<KeyBindingsConfig>,
    pub git: GitConfig,
}

#[derive(Debug, Clone)]
pub enum ThemeName {
    Dark,
    Light,
}

#[derive(Debug, Clone)]
pub enum DateFormat {
    Relative,  // "2 hours ago"
    Iso,       // "2024-01-15 14:30:00"
    Local,     // Based on locale
}

#[derive(Debug, Clone)]
pub struct Themes {
    pub dark: Theme,
    pub light: Theme,
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub background: Color,
    pub foreground: Color,
    pub border: Color,
    pub border_focused: Color,
    pub selection: Color,
    pub selection_text: Color,
    pub diff_add: Color,
    pub diff_remove: Color,
    pub diff_hunk: Color,
    pub staged: Color,
    pub unstaged: Color,
    pub untracked: Color,
    pub branch_current: Color,
    pub branch_local: Color,
    pub branch_remote: Color,
}

#[derive(Debug, Clone)]
pub struct GitConfig {
    pub default_remote: String,
    pub default_branch: String,
    pub sign_commits: bool,
    pub gpg_key: Option<String>,
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
            keybindings: None,
            git: GitConfig::default(),
        }
    }
}
```

## TOML Parser

Custom minimal TOML parser (no external dependencies).

```rust
pub struct TomlParser {
    input: Vec<char>,
    pos: usize,
}

#[derive(Debug, Clone)]
pub enum TomlValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<TomlValue>),
    Table(HashMap<String, TomlValue>),
}

impl TomlParser {
    pub fn parse(input: &str) -> Result<TomlValue, ParseError> {
        let mut parser = Self {
            input: input.chars().collect(),
            pos: 0,
        };
        parser.parse_document()
    }

    fn parse_document(&mut self) -> Result<TomlValue, ParseError> {
        let mut root = HashMap::new();
        let mut current_table: Vec<String> = Vec::new();

        while !self.is_eof() {
            self.skip_whitespace_and_comments();

            if self.is_eof() {
                break;
            }

            if self.peek() == Some('[') {
                // Table header
                current_table = self.parse_table_header()?;
            } else {
                // Key-value pair
                let (key, value) = self.parse_key_value()?;
                self.insert_value(&mut root, &current_table, &key, value)?;
            }
        }

        Ok(TomlValue::Table(root))
    }

    fn parse_table_header(&mut self) -> Result<Vec<String>, ParseError> {
        self.expect('[')?;
        let mut path = Vec::new();

        loop {
            let key = self.parse_key()?;
            path.push(key);

            self.skip_whitespace();
            match self.peek() {
                Some('.') => {
                    self.advance();
                    self.skip_whitespace();
                }
                Some(']') => {
                    self.advance();
                    break;
                }
                _ => return Err(ParseError::InvalidTableHeader),
            }
        }

        self.skip_to_newline();
        Ok(path)
    }

    fn parse_key_value(&mut self) -> Result<(String, TomlValue), ParseError> {
        let key = self.parse_key()?;
        self.skip_whitespace();
        self.expect('=')?;
        self.skip_whitespace();
        let value = self.parse_value()?;
        self.skip_to_newline();
        Ok((key, value))
    }

    fn parse_value(&mut self) -> Result<TomlValue, ParseError> {
        match self.peek() {
            Some('"') => self.parse_string(),
            Some('[') => self.parse_array(),
            Some('{') => self.parse_inline_table(),
            Some(c) if c.is_ascii_digit() || c == '-' || c == '+' => self.parse_number(),
            Some('t') | Some('f') => self.parse_boolean(),
            _ => Err(ParseError::InvalidValue),
        }
    }

    fn parse_string(&mut self) -> Result<TomlValue, ParseError> {
        self.expect('"')?;
        let mut s = String::new();

        while let Some(c) = self.peek() {
            if c == '"' {
                self.advance();
                return Ok(TomlValue::String(s));
            }
            if c == '\\' {
                self.advance();
                match self.peek() {
                    Some('n') => s.push('\n'),
                    Some('t') => s.push('\t'),
                    Some('r') => s.push('\r'),
                    Some('\\') => s.push('\\'),
                    Some('"') => s.push('"'),
                    _ => return Err(ParseError::InvalidEscape),
                }
            } else {
                s.push(c);
            }
            self.advance();
        }

        Err(ParseError::UnterminatedString)
    }

    fn parse_array(&mut self) -> Result<TomlValue, ParseError> {
        self.expect('[')?;
        let mut arr = Vec::new();

        loop {
            self.skip_whitespace_and_comments();

            if self.peek() == Some(']') {
                self.advance();
                break;
            }

            let value = self.parse_value()?;
            arr.push(value);

            self.skip_whitespace_and_comments();
            match self.peek() {
                Some(',') => {
                    self.advance();
                }
                Some(']') => {}
                _ => return Err(ParseError::InvalidArray),
            }
        }

        Ok(TomlValue::Array(arr))
    }

    // ... more parsing methods
}
```

## Config Loading

```rust
impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        let path = config_path();

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&path)?;
        Self::parse(&content)
    }

    pub fn parse(content: &str) -> Result<Self, ConfigError> {
        let toml = TomlParser::parse(content)?;
        let table = match toml {
            TomlValue::Table(t) => t,
            _ => return Err(ConfigError::InvalidFormat),
        };

        let mut config = Self::default();

        // Parse top-level values
        if let Some(TomlValue::String(s)) = table.get("theme") {
            config.theme = match s.as_str() {
                "dark" => ThemeName::Dark,
                "light" => ThemeName::Light,
                _ => return Err(ConfigError::InvalidTheme),
            };
        }

        if let Some(TomlValue::Boolean(b)) = table.get("show_line_numbers") {
            config.show_line_numbers = *b;
        }

        if let Some(TomlValue::Integer(n)) = table.get("diff_context_lines") {
            config.diff_context_lines = *n as u32;
        }

        if let Some(TomlValue::Integer(n)) = table.get("max_commits") {
            config.max_commits = *n as usize;
        }

        if let Some(TomlValue::String(s)) = table.get("date_format") {
            config.date_format = match s.as_str() {
                "relative" => DateFormat::Relative,
                "iso" => DateFormat::Iso,
                "local" => DateFormat::Local,
                _ => return Err(ConfigError::InvalidDateFormat),
            };
        }

        // Parse theme sections
        if let Some(TomlValue::Table(themes)) = table.get("theme") {
            if let Some(TomlValue::Table(dark)) = themes.get("dark") {
                config.themes.dark = Self::parse_theme(dark)?;
            }
            if let Some(TomlValue::Table(light)) = themes.get("light") {
                config.themes.light = Self::parse_theme(light)?;
            }
        }

        // Parse keybindings
        if let Some(TomlValue::Table(kb)) = table.get("keybindings") {
            config.keybindings = Some(Self::parse_keybindings(kb)?);
        }

        // Parse git section
        if let Some(TomlValue::Table(git)) = table.get("git") {
            config.git = Self::parse_git_config(git)?;
        }

        Ok(config)
    }

    fn parse_theme(table: &HashMap<String, TomlValue>) -> Result<Theme, ConfigError> {
        let get_color = |key: &str| -> Result<Color, ConfigError> {
            match table.get(key) {
                Some(TomlValue::String(s)) => Color::from_hex(s),
                _ => Err(ConfigError::MissingColor(key.to_string())),
            }
        };

        Ok(Theme {
            background: get_color("background")?,
            foreground: get_color("foreground")?,
            border: get_color("border")?,
            border_focused: get_color("border_focused")?,
            selection: get_color("selection")?,
            selection_text: get_color("selection_text")?,
            diff_add: get_color("diff_add")?,
            diff_remove: get_color("diff_remove")?,
            diff_hunk: get_color("diff_hunk")?,
            staged: get_color("staged")?,
            unstaged: get_color("unstaged")?,
            untracked: get_color("untracked")?,
            branch_current: get_color("branch_current")?,
            branch_local: get_color("branch_local")?,
            branch_remote: get_color("branch_remote")?,
        })
    }
}
```

## Color Parsing

```rust
impl Color {
    pub fn from_hex(hex: &str) -> Result<Self, ConfigError> {
        let hex = hex.trim_start_matches('#');

        if hex.len() != 6 {
            return Err(ConfigError::InvalidColor(hex.to_string()));
        }

        let r = u8::from_str_radix(&hex[0..2], 16)?;
        let g = u8::from_str_radix(&hex[2..4], 16)?;
        let b = u8::from_str_radix(&hex[4..6], 16)?;

        Ok(Color::Rgb(r, g, b))
    }
}
```

## Environment Variables

```rust
impl Config {
    pub fn editor(&self) -> String {
        self.editor.clone()
            .or_else(|| std::env::var("VISUAL").ok())
            .or_else(|| std::env::var("EDITOR").ok())
            .unwrap_or_else(|| "vi".to_string())
    }
}
```

## Config Hot Reload

```rust
pub struct ConfigWatcher {
    path: PathBuf,
    last_modified: SystemTime,
}

impl ConfigWatcher {
    pub fn new() -> Self {
        let path = config_path();
        let last_modified = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        Self { path, last_modified }
    }

    pub fn check_and_reload(&mut self, config: &mut Config) -> Result<bool, ConfigError> {
        let current_modified = std::fs::metadata(&self.path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        if current_modified > self.last_modified {
            *config = Config::load()?;
            self.last_modified = current_modified;
            Ok(true)  // Config was reloaded
        } else {
            Ok(false)  // No change
        }
    }
}
```

## Theme Accessor

```rust
impl Config {
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
}
```
