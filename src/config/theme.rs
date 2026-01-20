use crate::tui::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeName {
    Dark,
    Light,
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

impl Theme {
    pub fn dark() -> Self {
        Self {
            background: Color::Rgb(30, 30, 46),       // #1e1e2e
            foreground: Color::Rgb(205, 214, 244),    // #cdd6f4
            border: Color::Rgb(108, 112, 134),        // #6c7086
            border_focused: Color::Rgb(137, 180, 250), // #89b4fa
            selection: Color::Rgb(49, 50, 68),        // #313244
            selection_text: Color::Rgb(205, 214, 244), // #cdd6f4
            diff_add: Color::Rgb(166, 227, 161),      // #a6e3a1
            diff_remove: Color::Rgb(243, 139, 168),   // #f38ba8
            diff_hunk: Color::Rgb(137, 220, 235),     // #89dceb
            staged: Color::Rgb(166, 227, 161),        // #a6e3a1
            unstaged: Color::Rgb(249, 226, 175),      // #f9e2af
            untracked: Color::Rgb(108, 112, 134),     // #6c7086
            branch_current: Color::Rgb(166, 227, 161), // #a6e3a1
            branch_local: Color::Rgb(137, 180, 250),  // #89b4fa
            branch_remote: Color::Rgb(203, 166, 247), // #cba6f7
        }
    }

    pub fn light() -> Self {
        Self {
            background: Color::Rgb(239, 241, 245),    // #eff1f5
            foreground: Color::Rgb(76, 79, 105),      // #4c4f69
            border: Color::Rgb(156, 160, 176),        // #9ca0b0
            border_focused: Color::Rgb(30, 102, 245), // #1e66f5
            selection: Color::Rgb(204, 208, 218),     // #ccd0da
            selection_text: Color::Rgb(76, 79, 105),  // #4c4f69
            diff_add: Color::Rgb(64, 160, 43),        // #40a02b
            diff_remove: Color::Rgb(210, 15, 57),     // #d20f39
            diff_hunk: Color::Rgb(4, 165, 229),       // #04a5e5
            staged: Color::Rgb(64, 160, 43),          // #40a02b
            unstaged: Color::Rgb(223, 142, 29),       // #df8e1d
            untracked: Color::Rgb(156, 160, 176),     // #9ca0b0
            branch_current: Color::Rgb(64, 160, 43),  // #40a02b
            branch_local: Color::Rgb(30, 102, 245),   // #1e66f5
            branch_remote: Color::Rgb(136, 57, 239),  // #8839ef
        }
    }
}
