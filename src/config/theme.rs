use crate::tui::Color;

#[derive(Debug, Clone)]
pub struct Theme {
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
    pub fn default() -> Self {
        Self {
            foreground: Color::Rgb(205, 214, 244),    // #cdd6f4
            border: Color::Rgb(108, 112, 134),        // #6c7086
            border_focused: Color::Rgb(137, 180, 250), // #89b4fa
            selection: Color::Rgb(49, 50, 68),        // #313244
            selection_text: Color::Rgb(205, 214, 244), // #cdd6f4
            diff_add: Color::Rgb(166, 227, 161),      // #a6e3a1
            diff_remove: Color::Rgb(243, 139, 168),   // #f38ba8
            diff_hunk: Color::Rgb(137, 220, 235),     // #89dceb
            staged: Color::Rgb(166, 227, 161),        // #a6e3a1
            unstaged: Color::Rgb(147, 153, 178),      // #9399b2 (gray)
            untracked: Color::Rgb(108, 112, 134),     // #6c7086
            branch_current: Color::Rgb(166, 227, 161), // #a6e3a1
            branch_local: Color::Rgb(137, 180, 250),  // #89b4fa
            branch_remote: Color::Rgb(203, 166, 247), // #cba6f7
        }
    }
}
