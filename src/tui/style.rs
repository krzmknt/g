use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub modifier: Modifier,
}

impl Style {
    pub const fn new() -> Self {
        Self {
            fg: None,
            bg: None,
            modifier: Modifier::empty(),
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

/// Minimum WCAG contrast ratio for a foreground to count as readable.
const MIN_CONTRAST_RATIO: f32 = 1.4;
/// Minimum Euclidean RGB distance for a foreground to count as readable.
const MIN_RGB_DISTANCE: f32 = 90.0;

impl Color {
    /// Resolve the color to RGB using the standard xterm palette for named
    /// and indexed colors. `Reset` has no known value.
    pub fn to_rgb(self) -> Option<(u8, u8, u8)> {
        match self {
            Color::Reset => None,
            Color::Black => Some((0, 0, 0)),
            Color::Red => Some((205, 49, 49)),
            Color::Green => Some((13, 188, 121)),
            Color::Yellow => Some((229, 229, 16)),
            Color::Blue => Some((36, 114, 200)),
            Color::Magenta => Some((188, 63, 188)),
            Color::Cyan => Some((17, 168, 205)),
            Color::White => Some((255, 255, 255)),
            Color::Gray | Color::DarkGray => Some((102, 102, 102)),
            Color::LightRed => Some((241, 76, 76)),
            Color::LightGreen => Some((35, 209, 139)),
            Color::LightYellow => Some((245, 245, 67)),
            Color::LightBlue => Some((59, 142, 234)),
            Color::LightMagenta => Some((214, 112, 214)),
            Color::LightCyan => Some((41, 184, 219)),
            Color::Indexed(n) => Some(indexed_to_rgb(n)),
            Color::Rgb(r, g, b) => Some((r, g, b)),
        }
    }

    /// Relative luminance as defined by WCAG 2.x.
    fn luminance(self) -> Option<f32> {
        let (r, g, b) = self.to_rgb()?;
        let channel = |v: u8| {
            let c = v as f32 / 255.0;
            if c <= 0.03928 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        Some(0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b))
    }

    /// Whether text in this color can be read on the given background.
    ///
    /// A foreground is unreadable when it is nearly the same color as the
    /// background or when its luminance contrast is too low. Colors with an
    /// unknown value are treated as unreadable so callers fall back safely.
    pub fn is_readable_on(self, bg: Color) -> bool {
        let (Some(fg_rgb), Some(bg_rgb)) = (self.to_rgb(), bg.to_rgb()) else {
            return false;
        };
        let (Some(fg_lum), Some(bg_lum)) = (self.luminance(), bg.luminance()) else {
            return false;
        };

        let distance = {
            let dr = fg_rgb.0 as f32 - bg_rgb.0 as f32;
            let dg = fg_rgb.1 as f32 - bg_rgb.1 as f32;
            let db = fg_rgb.2 as f32 - bg_rgb.2 as f32;
            (dr * dr + dg * dg + db * db).sqrt()
        };
        let (lighter, darker) = if fg_lum > bg_lum {
            (fg_lum, bg_lum)
        } else {
            (bg_lum, fg_lum)
        };
        let contrast = (lighter + 0.05) / (darker + 0.05);

        distance >= MIN_RGB_DISTANCE && contrast >= MIN_CONTRAST_RATIO
    }
}

fn indexed_to_rgb(n: u8) -> (u8, u8, u8) {
    const BASE: [Color; 16] = [
        Color::Black,
        Color::Red,
        Color::Green,
        Color::Yellow,
        Color::Blue,
        Color::Magenta,
        Color::Cyan,
        Color::White,
        Color::Gray,
        Color::LightRed,
        Color::LightGreen,
        Color::LightYellow,
        Color::LightBlue,
        Color::LightMagenta,
        Color::LightCyan,
        Color::White,
    ];
    match n {
        0..=15 => BASE[n as usize].to_rgb().unwrap_or((0, 0, 0)),
        16..=231 => {
            let i = n - 16;
            let level = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
            (level(i / 36), level((i / 6) % 6), level(i % 6))
        }
        232..=255 => {
            let v = 8 + (n - 232) * 10;
            (v, v, v)
        }
    }
}

impl Style {
    /// Place this style on a highlight background, keeping the original
    /// foreground unless it would be unreadable, in which case `fallback_fg`
    /// is used instead.
    pub fn on_background(mut self, bg: Color, fallback_fg: Color) -> Self {
        let readable = self.fg.is_some_and(|fg| fg.is_readable_on(bg));
        if !readable {
            self.fg = Some(fallback_fg);
        }
        self.bg = Some(bg);
        self
    }
}

#[cfg(test)]
mod readability_tests {
    use super::*;

    const ORANGE: Color = Color::Rgb(255, 140, 0);
    const BLACK: Color = Color::Black;

    #[test]
    fn named_and_indexed_colors_resolve_to_rgb() {
        assert_eq!(Color::White.to_rgb(), Some((255, 255, 255)));
        assert_eq!(Color::Indexed(9).to_rgb(), Color::LightRed.to_rgb());
        assert_eq!(Color::Indexed(16).to_rgb(), Some((0, 0, 0)));
        assert_eq!(Color::Indexed(231).to_rgb(), Some((255, 255, 255)));
        assert_eq!(Color::Indexed(232).to_rgb(), Some((8, 8, 8)));
        assert_eq!(Color::Reset.to_rgb(), None);
    }

    #[test]
    fn identical_colors_are_unreadable() {
        assert!(!ORANGE.is_readable_on(ORANGE));
        assert!(!Color::Rgb(250, 145, 10).is_readable_on(ORANGE));
    }

    #[test]
    fn distinct_colors_stay_readable() {
        // Cyan commit hash and light foreground on the default orange selection
        assert!(Color::Rgb(137, 220, 235).is_readable_on(ORANGE));
        assert!(Color::Rgb(205, 214, 244).is_readable_on(ORANGE));
        // Dim gray on blue selection
        assert!(Color::Rgb(108, 112, 134).is_readable_on(Color::Rgb(137, 180, 250)));
        assert!(Color::Black.is_readable_on(Color::White));
    }

    #[test]
    fn low_luminance_contrast_is_unreadable() {
        // Light foreground on white selection
        assert!(!Color::Rgb(205, 214, 244).is_readable_on(Color::White));
        // Peach on orange
        assert!(!Color::Rgb(250, 179, 135).is_readable_on(ORANGE));
    }

    #[test]
    fn on_background_keeps_readable_fg_and_sets_bg() {
        let style = Style::new().fg(Color::Rgb(137, 220, 235)).bold();
        let highlighted = style.on_background(ORANGE, BLACK);
        assert_eq!(highlighted.fg, Some(Color::Rgb(137, 220, 235)));
        assert_eq!(highlighted.bg, Some(ORANGE));
        assert!(highlighted.modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn on_background_replaces_unreadable_or_missing_fg() {
        assert_eq!(
            Style::new().fg(ORANGE).on_background(ORANGE, BLACK).fg,
            Some(BLACK)
        );
        assert_eq!(Style::new().on_background(ORANGE, BLACK).fg, Some(BLACK));
        assert_eq!(
            Style::new()
                .fg(Color::Reset)
                .on_background(ORANGE, BLACK)
                .fg,
            Some(BLACK)
        );
    }
}
