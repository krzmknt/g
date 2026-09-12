use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub modifier: Modifier,
    /// Color of the underline (SGR 58). Terminals without support fall back
    /// to the text color.
    pub underline_color: Option<Color>,
}

impl Style {
    pub const fn new() -> Self {
        Self {
            fg: None,
            bg: None,
            modifier: Modifier::empty(),
            underline_color: None,
        }
    }

    pub const fn fg(mut self, color: Color) -> Self {
        self.fg = Some(color);
        self
    }

    pub const fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    pub const fn bold(mut self) -> Self {
        self.modifier = self.modifier.union(Modifier::BOLD);
        self
    }

    pub const fn dim(mut self) -> Self {
        self.modifier = self.modifier.union(Modifier::DIM);
        self
    }

    pub const fn italic(mut self) -> Self {
        self.modifier = self.modifier.union(Modifier::ITALIC);
        self
    }

    pub const fn underline(mut self) -> Self {
        self.modifier = self.modifier.union(Modifier::UNDERLINE);
        self
    }

    pub const fn reversed(mut self) -> Self {
        self.modifier = self.modifier.union(Modifier::REVERSED);
        self
    }

    pub const fn underline_color(mut self, color: Color) -> Self {
        self.underline_color = Some(color);
        self
    }

    pub fn to_ansi(self) -> String {
        let mut result = String::with_capacity(32);
        result.push_str("\x1b[0m"); // Reset first

        if self.modifier.contains(Modifier::BOLD) {
            result.push_str("\x1b[1m");
        }
        if self.modifier.contains(Modifier::DIM) {
            result.push_str("\x1b[2m");
        }
        if self.modifier.contains(Modifier::ITALIC) {
            result.push_str("\x1b[3m");
        }
        if self.modifier.contains(Modifier::UNDERLINE) {
            result.push_str("\x1b[4m");
            if let Some(color) = self.underline_color {
                let _ = write!(result, "{}", color.to_ansi_underline());
            }
        }
        if self.modifier.contains(Modifier::REVERSED) {
            result.push_str("\x1b[7m");
        }

        if let Some(fg) = self.fg {
            let _ = write!(result, "{}", fg.to_ansi_fg());
        }
        if let Some(bg) = self.bg {
            let _ = write!(result, "{}", bg.to_ansi_bg());
        }

        result
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Reset,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    Gray,
    DarkGray,
    LightRed,
    LightGreen,
    LightYellow,
    LightBlue,
    LightMagenta,
    LightCyan,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

impl Color {
    /// SGR 58 underline color sequence.
    pub fn to_ansi_underline(self) -> String {
        let index = match self {
            Color::Reset => return "\x1b[59m".to_string(),
            Color::Rgb(r, g, b) => return format!("\x1b[58;2;{};{};{}m", r, g, b),
            Color::Indexed(n) => n,
            Color::Black => 0,
            Color::Red => 1,
            Color::Green => 2,
            Color::Yellow => 3,
            Color::Blue => 4,
            Color::Magenta => 5,
            Color::Cyan => 6,
            Color::White => 7,
            Color::Gray | Color::DarkGray => 8,
            Color::LightRed => 9,
            Color::LightGreen => 10,
            Color::LightYellow => 11,
            Color::LightBlue => 12,
            Color::LightMagenta => 13,
            Color::LightCyan => 14,
        };
        format!("\x1b[58;5;{}m", index)
    }

    pub fn to_ansi_fg(self) -> String {
        match self {
            Color::Reset => "\x1b[39m".to_string(),
            Color::Black => "\x1b[30m".to_string(),
            Color::Red => "\x1b[31m".to_string(),
            Color::Green => "\x1b[32m".to_string(),
            Color::Yellow => "\x1b[33m".to_string(),
            Color::Blue => "\x1b[34m".to_string(),
            Color::Magenta => "\x1b[35m".to_string(),
            Color::Cyan => "\x1b[36m".to_string(),
            Color::White => "\x1b[37m".to_string(),
            Color::Gray => "\x1b[90m".to_string(),
            Color::DarkGray => "\x1b[90m".to_string(),
            Color::LightRed => "\x1b[91m".to_string(),
            Color::LightGreen => "\x1b[92m".to_string(),
            Color::LightYellow => "\x1b[93m".to_string(),
            Color::LightBlue => "\x1b[94m".to_string(),
            Color::LightMagenta => "\x1b[95m".to_string(),
            Color::LightCyan => "\x1b[96m".to_string(),
            Color::Indexed(n) => format!("\x1b[38;5;{}m", n),
            Color::Rgb(r, g, b) => format!("\x1b[38;2;{};{};{}m", r, g, b),
        }
    }

    pub fn to_ansi_bg(self) -> String {
        match self {
            Color::Reset => "\x1b[49m".to_string(),
            Color::Black => "\x1b[40m".to_string(),
            Color::Red => "\x1b[41m".to_string(),
            Color::Green => "\x1b[42m".to_string(),
            Color::Yellow => "\x1b[43m".to_string(),
            Color::Blue => "\x1b[44m".to_string(),
            Color::Magenta => "\x1b[45m".to_string(),
            Color::Cyan => "\x1b[46m".to_string(),
            Color::White => "\x1b[47m".to_string(),
            Color::Gray => "\x1b[100m".to_string(),
            Color::DarkGray => "\x1b[100m".to_string(),
            Color::LightRed => "\x1b[101m".to_string(),
            Color::LightGreen => "\x1b[102m".to_string(),
            Color::LightYellow => "\x1b[103m".to_string(),
            Color::LightBlue => "\x1b[104m".to_string(),
            Color::LightMagenta => "\x1b[105m".to_string(),
            Color::LightCyan => "\x1b[106m".to_string(),
            Color::Indexed(n) => format!("\x1b[48;5;{}m", n),
            Color::Rgb(r, g, b) => format!("\x1b[48;2;{};{};{}m", r, g, b),
        }
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return None;
        }

        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;

        Some(Color::Rgb(r, g, b))
    }

    pub fn to_hex(self) -> Option<String> {
        match self {
            Color::Rgb(r, g, b) => Some(format!("#{:02x}{:02x}{:02x}", r, g, b)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifier(u8);

impl Modifier {
    pub const BOLD: Self = Self(0b0000_0001);
    pub const DIM: Self = Self(0b0000_0010);
    pub const ITALIC: Self = Self(0b0000_0100);
    pub const UNDERLINE: Self = Self(0b0000_1000);
    pub const REVERSED: Self = Self(0b0001_0000);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

#[cfg(test)]
mod underline_tests {
    use super::*;

    #[test]
    fn underline_color_is_emitted_after_underline() {
        let style = Style::new()
            .underline()
            .underline_color(Color::Rgb(203, 166, 247));
        assert_eq!(style.underline_color, Some(Color::Rgb(203, 166, 247)));
        assert_eq!(style.to_ansi(), "\x1b[0m\x1b[4m\x1b[58;2;203;166;247m");
    }

    #[test]
    fn underline_color_variants_map_to_sgr_58() {
        assert_eq!(Color::Rgb(1, 2, 3).to_ansi_underline(), "\x1b[58;2;1;2;3m");
        assert_eq!(Color::Indexed(42).to_ansi_underline(), "\x1b[58;5;42m");
        assert_eq!(Color::LightRed.to_ansi_underline(), "\x1b[58;5;9m");
        assert_eq!(Color::Reset.to_ansi_underline(), "\x1b[59m");
    }
}
